//! Reusable high-level application primitives and a native lifecycle for
//! Eternalist-style egui products.
//!
//! [`Inspector`] and [`LivingWait`] are renderer-neutral logical primitives.
//! Native targets additionally expose the one-window `NativeApp` lifecycle,
//! Poolrooms-water composition, responsiveness tracing, and optional
//! post-present acceptance witnessing. Domain state, workers, persistence,
//! fixtures, oracles, and acceptance stories remain application concerns.

pub mod cabinet;
pub mod inspector;
pub mod living_wait;

#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(not(target_arch = "wasm32"))]
pub mod responsiveness;

pub use cabinet::{
    Berth as CabinetBerth, Cabinet, CabinetAction, CabinetEntry, CabinetKey,
    EntryEdit as CabinetEntryEdit, Shelf as CabinetShelf, ShelfBerth as CabinetShelfBerth,
    ShelfEdit as CabinetShelfEdit,
};
pub use inspector::{Inspector, InspectorResponse};
pub use living_wait::LivingWait;

#[cfg(not(target_arch = "wasm32"))]
pub use native::{CloseDisposition, NativeApp, ResponsivenessSpec, WindowSpec, run};
#[cfg(not(target_arch = "wasm32"))]
pub use responsiveness::TraceGuard;
