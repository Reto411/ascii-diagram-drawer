//! PNG rasterization worker (ADR 012): the UI thread never blocks on rasterization.
//!
//! Pattern: one-shot job submission + result return over channels, polled on
//! subsequent frames while the GUI shows an "exporting" state. Native targets run
//! jobs on a dedicated worker thread behind a **bounded** queue (overflow is a typed
//! rejection — no unbounded growth); wasm32 has no threads, so the same API executes
//! inline (`PngWorker::new()` there *is* `inline()`). Both paths share one code path
//! and the same golden expectations (ADR 012 decision point 5).
//!
//! Drop semantics: dropping a threaded worker stops new submissions, but the finite
//! bounded queue still drains — already-submitted jobs (queued or in flight) all
//! finish. [`JobState::Abandoned`] is a defensive state for an abnormally dead worker.

use std::sync::mpsc::{self, Receiver, TryRecvError};
#[cfg(not(target_arch = "wasm32"))]
use std::sync::mpsc::{Sender, SyncSender};
#[cfg(not(target_arch = "wasm32"))]
use std::thread::JoinHandle;

use render_pipeline::ResolvedGrid;
use ui_api::{ExportError, Exporter};

use crate::exporter::GridExporter;

/// Default bounded-queue capacity: a user realistically queues at most one export at
/// a time while the "exporting" state is shown; the headroom absorbs bursts.
pub const DEFAULT_QUEUE_CAPACITY: usize = 4;

/// The bounded queue was full when a job was submitted — no unbounded growth (AC1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueueFull;

impl std::fmt::Display for QueueFull {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("png job queue is full — try again after an in-flight export finishes")
    }
}

impl std::error::Error for QueueFull {}

/// The state of a submitted job, as observed by polling (ADR 012: the GUI polls on
/// subsequent frames instead of blocking).
#[derive(Debug)]
pub enum JobState {
    /// Rasterization is still queued or in flight.
    Pending,
    /// Finished: the PNG bytes.
    Done(Vec<u8>),
    /// Failed with a seam-level export error (rasterizer backend missing, encode …).
    Error(ExportError),
    /// The job will never run — its result channel is gone. Defensive state for an
    /// abnormally dead worker; normal drops let queued jobs finish (see below).
    Abandoned,
}

/// A handle to a submitted PNG job. Poll it (non-blocking) from the UI thread.
#[derive(Debug)]
pub struct PngJob {
    reply: Receiver<Result<Vec<u8>, ExportError>>,
}

impl PngJob {
    /// Test hook: wrap an existing reply channel. Lets GUI-level tests drive the
    /// Pending → Done/Error transitions of a "slow exporter" deterministically (S-015)
    /// and observe `Abandoned`.
    #[must_use]
    pub fn from_reply(reply: Receiver<Result<Vec<u8>, ExportError>>) -> Self {
        Self { reply }
    }

    /// Non-blocking poll — designed to be called once per frame while "exporting".
    #[must_use]
    pub fn poll(&self) -> JobState {
        match self.reply.try_recv() {
            Ok(Ok(png)) => JobState::Done(png),
            Ok(Err(err)) => JobState::Error(err),
            Err(TryRecvError::Empty) => JobState::Pending,
            Err(TryRecvError::Disconnected) => JobState::Abandoned,
        }
    }
}

/// One unit of work moved (by ownership — no shared mutable state) to the worker.
#[cfg(not(target_arch = "wasm32"))]
struct PngWork {
    grid: ResolvedGrid,
    reply: Sender<Result<Vec<u8>, ExportError>>,
}

/// The PNG worker behind ADR 012's "exporting" state. Threaded on native targets,
/// inline on wasm32 (no threads exist there) — one API either way.
#[derive(Debug)]
pub struct PngWorker {
    #[cfg(not(target_arch = "wasm32"))]
    queue: Option<SyncSender<PngWork>>,
    #[cfg(not(target_arch = "wasm32"))]
    _thread: Option<JoinHandle<()>>,
}

#[cfg(not(target_arch = "wasm32"))]
impl PngWorker {
    /// The dedicated worker thread with [`DEFAULT_QUEUE_CAPACITY`] queue slots.
    #[must_use]
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_QUEUE_CAPACITY)
    }

    /// A threaded worker with an explicit bounded-queue capacity (clamped to ≥ 1).
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        let (queue, receiver) = mpsc::sync_channel(capacity.max(1));
        let thread = std::thread::Builder::new()
            .name("png-worker".to_owned())
            .spawn(move || run_worker(receiver))
            .expect("spawning the PNG worker thread");
        Self {
            queue: Some(queue),
            _thread: Some(thread),
        }
    }
}

impl PngWorker {
    /// The default executor for the current target (ADR 012, “one API either way”):
    /// threaded on native, inline on wasm32. This is what the composition root
    /// (`Services`) uses — it never needs to know which one it got.
    #[must_use]
    pub fn for_target() -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        {
            Self::new()
        }
        #[cfg(target_arch = "wasm32")]
        {
            Self::inline()
        }
    }

    /// A synchronous executor: `submit` rasterizes inline before returning, so the
    /// first poll already yields `Done`/`Error`. Used by wasm32 (no threads) and by
    /// tests that want the worker code path without a thread.
    #[must_use]
    pub fn inline() -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        return Self {
            queue: None,
            _thread: None,
        };
        #[cfg(target_arch = "wasm32")]
        return Self {};
    }

    /// Submits a job. On threaded workers this never blocks beyond the enqueue —
    /// a full bounded queue yields [`QueueFull`]; inline workers rasterize now.
    pub fn submit(&self, grid: &ResolvedGrid) -> Result<PngJob, QueueFull> {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(queue) = &self.queue {
            let (reply_tx, reply_rx) = mpsc::channel();
            queue
                .try_send(PngWork {
                    grid: grid.clone(),
                    reply: reply_tx,
                })
                .map_err(|_| QueueFull)?;
            return Ok(PngJob { reply: reply_rx });
        }

        // Inline path (always on wasm32; native via `inline()`): run the same
        // exporter call and hand back an already-complete job.
        let (reply_tx, reply_rx) = mpsc::channel();
        let result = GridExporter.to_png(grid);
        reply_tx
            .send(result)
            .expect("the reply receiver is alive for this call");
        Ok(PngJob { reply: reply_rx })
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Default for PngWorker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(target_arch = "wasm32")]
impl Default for PngWorker {
    fn default() -> Self {
        Self::inline()
    }
}

/// Worker loop: process jobs one at a time until the queue is disconnected. A
/// disconnect (the `PngWorker` was dropped) still drains the finite bounded buffer —
/// dropping never loses already-submitted work; it only stops *new* submissions.
#[cfg(not(target_arch = "wasm32"))]
fn run_worker(receiver: mpsc::Receiver<PngWork>) {
    let exporter = GridExporter;
    for work in receiver.iter() {
        // A dropped handle means the send fails — the job is simply unobserved.
        let _ = work.reply.send(exporter.to_png(&work.grid));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    /// A 300×150 frame: rasterization takes well over 100 ms (measured ≈ 150 ms in
    /// release, slower in debug) — enough headroom for the timing assertions.
    fn large_grid() -> ResolvedGrid {
        let (w, h) = (300u32, 150u32);
        let row = "+".to_string() + &"-".repeat((w - 2) as usize) + "+";
        let mut rows: Vec<String> = vec![row.clone()];
        for _ in 1..h.saturating_sub(1) {
            rows.push("|".to_string() + &" ".repeat((w - 2) as usize) + "|");
        }
        rows.push(row);
        let refs: Vec<&str> = rows.iter().map(|s| s.as_str()).collect();
        ResolvedGrid::from_rows(&refs).expect("valid grid")
    }

    fn small_grid() -> ResolvedGrid {
        ResolvedGrid::from_rows(&["+---+", "| A |", "+---+"]).expect("valid grid")
    }

    /// Polls until the job leaves `Pending`, bounded so a hang fails loudly.
    fn wait_for(job: &PngJob) -> JobState {
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            let state = job.poll();
            if !matches!(state, JobState::Pending) || Instant::now() > deadline {
                return state;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    /// AC1 — a full bounded queue rejects with the typed error; nothing grows
    /// unbounded. With capacity 1 and a long job in flight, three rapid submits must
    /// include at least one `QueueFull` (and at most two successes).
    #[test]
    fn overflow_is_a_typed_rejection() {
        let worker = PngWorker::with_capacity(1);
        let grid = large_grid();

        let outcomes: Vec<Result<PngJob, QueueFull>> =
            (0..3).map(|_| worker.submit(&grid)).collect();
        let rejected = outcomes
            .iter()
            .filter(|o| matches!(o, Err(QueueFull)))
            .count();
        assert!(
            rejected >= 1,
            "capacity 1 + in-flight job must reject: {outcomes:?}"
        );
        assert!(
            rejected <= 2,
            "at most two of three rapid submits can succeed"
        );

        // Every accepted job still completes correctly.
        for job in outcomes.iter().flatten() {
            match wait_for(job) {
                JobState::Done(png) => assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n"),
                other => panic!("expected Done, got {other:?}"),
            }
        }
    }

    /// AC2 — submitting does not block beyond the enqueue: for a job whose
    /// rasterization takes well over 100 ms, `submit` returns in milliseconds and the
    /// first poll still sees `Pending`; the result then arrives intact.
    #[test]
    fn submit_returns_while_rasterization_is_in_flight() {
        let worker = PngWorker::new();
        let grid = large_grid();

        let started = Instant::now();
        let job = worker.submit(&grid).expect("submits");
        let submit_elapsed = started.elapsed();
        assert!(
            submit_elapsed < Duration::from_millis(50),
            "submit blocked for {submit_elapsed:?} — it must return at enqueue time"
        );

        // The work takes >100 ms; microseconds after submit it cannot be done.
        assert!(
            matches!(job.poll(), JobState::Pending),
            "first poll must still be Pending (rasterization in flight)"
        );

        match wait_for(&job) {
            JobState::Done(png) => {
                let direct = exporters::to_png(&grid).expect("direct call");
                assert_eq!(png, direct, "worker result must equal the direct call");
            }
            other => panic!("expected Done, got {other:?}"),
        }
    }

    /// AC4 — the worker API and the direct exporter call produce byte-identical PNGs
    /// for the same grid (the shared-golden requirement: one code path either way).
    #[test]
    fn worker_output_is_byte_identical_to_the_direct_call() {
        let worker = PngWorker::new();
        let grid = small_grid();

        let job = worker.submit(&grid).expect("submits");
        match wait_for(&job) {
            JobState::Done(png) => {
                assert_eq!(png, exporters::to_png(&grid).expect("direct call"));
            }
            other => panic!("expected Done, got {other:?}"),
        }
    }

    /// The inline executor shares the API without a thread: the first poll is already
    /// complete (never `Pending`), and the bytes match the direct call. This is the
    /// exact code path wasm32 runs.
    #[test]
    fn inline_executor_completes_on_first_poll() {
        let worker = PngWorker::inline();
        let grid = small_grid();

        let job = worker.submit(&grid).expect("submits");
        match job.poll() {
            JobState::Done(png) => {
                assert_eq!(png, exporters::to_png(&grid).expect("direct call"));
            }
            other => panic!("inline submit must be complete on first poll, got {other:?}"),
        }
    }

    /// A job whose result channel is gone reports `Abandoned` (defensive state —
    /// observed here directly through the reply-channel hook).
    #[test]
    fn a_job_without_a_worker_reports_abandoned() {
        let (tx, rx) = mpsc::channel::<Result<Vec<u8>, ExportError>>();
        drop(tx);
        let job = PngJob::from_reply(rx);
        assert!(matches!(job.poll(), JobState::Abandoned));
    }

    /// The remaining observable surfaces: a failed rasterization is surfaced as
    /// `Error`, `QueueFull` carries an actionable message, and `Default` builds the
    /// threaded worker (native) which serves a job end-to-end.
    #[test]
    fn error_results_and_queue_full_message_are_surfaced() {
        let (tx, rx) = mpsc::channel::<Result<Vec<u8>, ExportError>>();
        tx.send(Err(ExportError::Raster {
            message: "boom".to_owned(),
        }))
        .expect("test channel is open");
        drop(tx);
        let job = PngJob::from_reply(rx);
        match job.poll() {
            JobState::Error(err) => assert!(matches!(err, ExportError::Raster { .. })),
            other => panic!("expected Error, got {other:?}"),
        }

        assert!(QueueFull.to_string().contains("queue is full"));

        let worker = PngWorker::default();
        let job = worker.submit(&small_grid()).expect("submits");
        match wait_for(&job) {
            JobState::Done(png) => assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n"),
            other => panic!("expected Done, got {other:?}"),
        }
    }

    /// Drop semantics: dropping the worker stops new submissions but never loses
    /// already-submitted work — a queued job still completes after the drop.
    #[test]
    fn dropping_the_worker_still_completes_queued_jobs() {
        let worker = PngWorker::with_capacity(2);
        let big = large_grid();
        let small = small_grid();

        let first = worker.submit(&big).expect("submits");
        // `big` is in flight (rasterization takes >100 ms), so `small` sits queued.
        std::thread::sleep(Duration::from_millis(50));
        let queued = worker.submit(&small).expect("goes into the queue");
        drop(worker);

        // New submissions are impossible (the sender is gone), but both existing jobs
        // finish: the finite bounded buffer drains on disconnect.
        for job in [&first, &queued] {
            match wait_for(job) {
                JobState::Done(png) => assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n"),
                other => panic!("expected Done, got {other:?}"),
            }
        }
    }
}
