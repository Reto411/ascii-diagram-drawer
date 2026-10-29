//! Core services — the application-layer backends behind the ADR 009 seam.
//!
//! Each module wraps one context crate and implements one `ui-api` trait:
//!
//! | service   | backend            | wraps          |
//! |-----------|--------------------|----------------|
//! | store     | `FsStore` / `MemoryStore` | `ad-format` (parse/serialize) + platform IO |
//! | renderer  | `PipelineRenderer` | `render-pipeline` (shared registry)  |
//! | exporter  | `GridExporter`     | `exporters`                          |
//! | catalog   | `RegistryCatalog`  | `shape-registry` (shared registry)   |
//!
//! Renderer and catalog share one [`SharedRegistry`] handle, so template registration
//! (S-017) is visible to both at once.
//!
//! [`Services`](services::Services) is the composition root: native vs. web differ
//! only in the store backend. [`fakes`] provides in-memory stand-ins for GUI tests
//! (the T7 pattern, §6.3). [`PngWorker`](worker::PngWorker) offloads rasterization
//! to a worker thread (native) / inline (wasm) per ADR 012.

pub mod catalog;
pub mod collections;
pub mod exporter;
pub mod fakes;
pub mod registry_state;
pub mod renderer;
pub mod services;
pub mod store;
pub mod worker;

pub use catalog::RegistryCatalog;
pub use collections::{CollectionLoadWarning, load_from_dir};
pub use exporter::GridExporter;
pub use registry_state::SharedRegistry;
pub use renderer::PipelineRenderer;
pub use services::Services;
pub use store::{FsStore, MemoryStore};
pub use worker::{JobState, PngJob, PngWorker, QueueFull};
