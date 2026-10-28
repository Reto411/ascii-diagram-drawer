//! Application layer — backends + composition root, [ADR 009](../../../doc/adr/009-ui-api-service-layer.md).
//!
//! Concrete implementations of the `ui-api` traits (thin wrappers over the context
//! crates) and the `Services` composition root (`native()` / `web()`). Platform IO
//! is confined to this layer. The PNG worker thread ([ADR 012](../../../doc/adr/012-responsive-ui-threading.md))
//! also lives here (S-011).
