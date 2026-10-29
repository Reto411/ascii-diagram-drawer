//! SVG/PNG export flows (S-015, FR5/RS4): plain functions over the seam services,
//! unit-tested headlessly against fakes — the S-012…S-017 pattern.
//!
//! Two flows:
//! - **SVG** is synchronous and cheap (svgbob translation of the exact text bytes) —
//!   one call, the exporter's bytes go straight to the sink.
//! - **PNG** rasterizes through [`PngWorker`] (ADR 012): submit once, then
//!   [`PngExport::tick`] once per frame while the UI shows its "exporting" state.
//!   Nothing on the frame path ever blocks on rasterization (RS4).

use std::path::{Path, PathBuf};

use core_services::{JobState, PngJob, PngWorker, Services};
use diagram_model::Diagram;
use ui_api::ResolvedGrid;

/// The whole-diagram scale as offered by the GUI control (FR8): the aggregate
/// enforces ≥ 1; the upper bound keeps grids usable (an 8×-scaled 200-cell row is
/// already 1600 characters wide). Implementation choice, not a format constraint.
pub const MAX_GUI_SCALE: u32 = 8;

/// The control must be able to scale *up* at all — compile-time guard (S-015 AC3).
const _: () = assert!(MAX_GUI_SCALE >= 2);

/// The exact SVG bytes for `diagram`: rendered through the seam renderer and passed
/// to `Exporter::to_svg` — byte-identical to what the canvas shows (WYSIWYG).
///
/// # Errors
/// A human-readable message when the model cannot be rendered or the exporter fails.
pub fn svg_bytes(services: &Services, diagram: &Diagram) -> Result<String, String> {
    let grid = services
        .renderer
        .render(diagram)
        .map_err(|e| e.to_string())?;
    services.exporter.to_svg(&grid).map_err(|e| e.to_string())
}

/// Sibling file path for an export of `document` with extension `ext`
/// (`diagram.ad` → `diagram.svg`) — the same rule as S-014's `.txt` output.
#[must_use]
pub fn export_output_path(document: &Path, ext: &str) -> PathBuf {
    document.with_extension(ext)
}

/// One completed PNG job, as observed by [`PngExport::tick`] — the bytes are ready
/// for the sink, or the failure is ready for a user notice.
#[derive(Debug, PartialEq, Eq)]
pub enum PngTick {
    /// Rasterization finished: the exact PNG bytes.
    Done(Vec<u8>),
    /// The job failed with a seam-level export error (message suitable for display).
    Failed(String),
}

/// The in-flight "exporting…" state of a PNG export (RS4): holds the worker's job
/// handle; the UI ticks it once per frame and stays responsive meanwhile.
#[derive(Debug)]
pub struct PngExport {
    job: Option<PngJob>,
}

impl PngExport {
    /// Submits `diagram` to the worker (rendering it first) and starts tracking the
    /// job.
    ///
    /// # Errors
    /// A human-readable message when the model cannot be rendered, or the bounded
    /// job queue is full (the user should retry once an in-flight export finishes).
    pub fn start(services: &Services, diagram: &Diagram) -> Result<Self, String> {
        let grid = services
            .renderer
            .render(diagram)
            .map_err(|e| e.to_string())?;
        Self::start_grid(&services.png_worker, &grid)
    }

    /// Submits a pre-rendered `grid` to the worker (separated for tests and for
    /// callers that already hold the frame's grid).
    ///
    /// # Errors
    /// A human-readable message when the bounded job queue is full.
    pub fn start_grid(worker: &PngWorker, grid: &ResolvedGrid) -> Result<Self, String> {
        worker
            .submit(grid)
            .map(Self::from_job)
            .map_err(|e| e.to_string())
    }

    /// Wraps an existing job handle — the test seam that lets GUI tests drive a
    /// "slow exporter" deterministically (S-015 AC2).
    #[must_use]
    pub fn from_job(job: PngJob) -> Self {
        Self { job: Some(job) }
    }

    /// Polls the job once (non-blocking). `None` = still exporting; `Some` = the
    /// terminal state — the job is consumed, further ticks report `None`.
    #[must_use]
    pub fn tick(&mut self) -> Option<PngTick> {
        let job = self.job.as_ref()?;
        match job.poll() {
            JobState::Pending => None,
            JobState::Done(bytes) => {
                self.job = None;
                Some(PngTick::Done(bytes))
            }
            JobState::Error(err) => {
                self.job = None;
                Some(PngTick::Failed(err.to_string()))
            }
            JobState::Abandoned => {
                self.job = None;
                Some(PngTick::Failed(
                    "export aborted — no worker result".to_owned(),
                ))
            }
        }
    }
}

/// Writes export bytes to `path` on native targets; wasm has no filesystem — the
/// browser download lands in S-019 (same sanctioned exception as S-014's `.txt`).
#[cfg(not(target_arch = "wasm32"))]
pub fn write_bytes_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    std::fs::write(path, bytes).map_err(|e| format!("could not write {}: {e}", path.display()))
}

/// AC — web target: no filesystem; the browser download lands in S-019.
#[cfg(target_arch = "wasm32")]
pub fn write_bytes_file(_path: &Path, _bytes: &[u8]) -> Result<(), String> {
    Err("export download arrives with the wasm milestone (S-019)".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, mpsc};

    /// A `Services` built entirely from fakes with deterministic media bytes.
    fn fake_services() -> Services {
        use core_services::fakes::{FakeCatalog, FakeExporter, FakeRenderer, FakeStore};
        use ui_api::ShapeInfo;

        Services {
            store: Arc::new(FakeStore::default()),
            renderer: Arc::new(FakeRenderer::with_grid(
                ResolvedGrid::from_rows(&["+---+"]).expect("valid grid"),
            )),
            exporter: Arc::new(FakeExporter),
            catalog: Arc::new(FakeCatalog::default().add(
                ShapeInfo::builtin(
                    "box",
                    "Box",
                    diagram_model::geometry::CellSize::new(9, 5).expect("valid size"),
                ),
                Vec::new(),
            )),
            png_worker: Arc::new(PngWorker::inline()),
        }
    }

    /// S-015 AC1 — the SVG sink receives exactly the exporter's bytes.
    #[test]
    fn svg_bytes_are_the_exporters_exact_bytes() {
        let services = fake_services();
        let diagram = Diagram::new();
        assert_eq!(
            svg_bytes(&services, &diagram).expect("fakes never fail"),
            "<svg xmlns=\"http://www.w3.org/2000/svg\">fake</svg>"
        );
    }

    /// Sibling-path rule: `diagram.ad` → `diagram.svg` / `diagram.png`.
    #[test]
    fn export_paths_are_document_siblings() {
        let doc = Path::new("/tmp/diagram.ad");
        assert_eq!(
            export_output_path(doc, "svg"),
            PathBuf::from("/tmp/diagram.svg")
        );
        assert_eq!(
            export_output_path(doc, "png"),
            PathBuf::from("/tmp/diagram.png")
        );
    }

    /// S-015 AC2 — a fake slow exporter: the job stays Pending until its reply
    /// arrives, then tick yields exactly the Done bytes; the export is terminal.
    #[test]
    fn pending_job_stays_pending_until_the_reply_arrives() {
        let (tx, rx) = mpsc::channel::<Result<Vec<u8>, ui_api::ExportError>>();
        let mut export = PngExport::from_job(PngJob::from_reply(rx));

        assert_eq!(export.tick(), None, "no reply yet → still exporting");
        tx.send(Ok(b"\x89PNG\r\n\x1a\n".to_vec()))
            .expect("sender alive");
        assert_eq!(
            export.tick(),
            Some(PngTick::Done(b"\x89PNG\r\n\x1a\n".to_vec()))
        );
        assert_eq!(export.tick(), None, "terminal: the job is consumed");
    }

    /// S-015 AC2 — a failed job surfaces as a displayable message, not a panic.
    #[test]
    fn failed_job_yields_a_message() {
        let (tx, rx) = mpsc::channel::<Result<Vec<u8>, ui_api::ExportError>>();
        let mut export = PngExport::from_job(PngJob::from_reply(rx));
        tx.send(Err(ui_api::ExportError::Unsupported))
            .expect("sender alive");
        assert!(matches!(export.tick(), Some(PngTick::Failed(_))));
    }

    /// S-015 AC2 — a disconnected job (dead worker) reports as failed, not pending.
    #[test]
    fn abandoned_job_yields_a_message() {
        let (tx, rx) = mpsc::channel::<Result<Vec<u8>, ui_api::ExportError>>();
        let mut export = PngExport::from_job(PngJob::from_reply(rx));
        drop(tx);
        assert!(matches!(export.tick(), Some(PngTick::Failed(_))));
    }

    /// S-015 AC2 — end-to-end through the real (threaded) worker: submit, tick to
    /// Done without blocking, and the bytes are a real PNG.
    #[test]
    fn real_worker_runs_the_full_flow() {
        let services = Services::native();
        let mut diagram = Diagram::new();
        let id = diagram.new_object_id();
        let instance = diagram_model::objects::ShapeInstance::new(
            id,
            "box",
            "",
            diagram_model::geometry::CellPos::origin(),
            diagram_model::geometry::CellSize::new(9, 5).expect("valid size"),
        )
        .expect("valid instance");
        diagram.place_shape(instance).expect("places");

        let mut export = PngExport::start(&services, &diagram).expect("submits");
        // The threaded worker may already be done for a tiny grid; either way the
        // flow ends in Done with real PNG bytes. Polls are spaced like frames
        // (~2 ms ≪ the 16 ms frame budget) — a tight spin would outrun thread start.
        for _ in 0..500 {
            match export.tick() {
                None => std::thread::sleep(std::time::Duration::from_millis(2)),
                Some(PngTick::Done(bytes)) => {
                    assert_eq!(&bytes[0..4], &b"\x89PNG"[..4], "PNG signature");
                    return;
                }
                Some(PngTick::Failed(msg)) => panic!("real worker failed: {msg}"),
            }
        }
        panic!("worker did not finish after 500 frame-spaced polls");
    }

    /// `start` renders through the seam renderer — an unrenderable model fails with
    /// a message before anything is submitted.
    #[test]
    fn start_rejects_unrenderable_models() {
        let failing = Services {
            renderer: Arc::new(core_services::fakes::FakeRenderer::failing("nope")),
            ..fake_services()
        };
        assert!(PngExport::start(&failing, &Diagram::new()).is_err());
    }

    /// Native write helper: bytes land on disk.
    #[test]
    fn write_bytes_file_round_trips() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("out.png");
        write_bytes_file(&path, b"1234").expect("writes");
        assert_eq!(std::fs::read(&path).expect("reads"), b"1234");
    }
}
