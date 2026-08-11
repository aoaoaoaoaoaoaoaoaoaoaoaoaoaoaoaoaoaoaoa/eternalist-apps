//! Reusable high-level application primitives and a native lifecycle for
//! Eternalist-style egui products.
//!
//! [`Inspector`], [`LivingWait`], [`cabinet`], [`commands`],
//! [`command_guide`], and [`panel_navigation`] are renderer-neutral logical
//! primitives. Native targets additionally expose the one-window `NativeApp`
//! lifecycle, Poolrooms-water composition, responsiveness tracing, and
//! optional post-present acceptance witnessing. Domain state, typed command
//! consequences, workers, persistence, fixtures, oracles, and acceptance
//! stories remain application concerns.

#[cfg(all(test, target_os = "linux"))]
use egui_tester as _;

pub mod cabinet;
pub mod command_guide;
pub mod commands;
pub mod inspector;
pub mod living_wait;
pub mod panel_navigation;

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
mod native;
#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
pub mod responsiveness;

pub use cabinet::{
    Berth as CabinetBerth, Cabinet, CabinetAction, CabinetEntry, CabinetKey,
    EntryEdit as CabinetEntryEdit, Shelf as CabinetShelf, ShelfBerth as CabinetShelfBerth,
    ShelfEdit as CabinetShelfEdit,
};
pub use inspector::{Inspector, InspectorResponse};
pub use living_wait::LivingWait;

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
pub use native::{CloseDisposition, NativeApp, ResponsivenessSpec, WindowSpec, run};
#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
pub use responsiveness::TraceGuard;
