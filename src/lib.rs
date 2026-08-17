//! Reusable high-level application primitives and a native lifecycle for
//! Eternalist-style egui products.
//!
//! [`Inspector`], [`LivingWait`], [`cabinet`], [`commands`],
//! [`command_guide`], and [`panel_navigation`] are renderer-neutral logical
//! primitives. Native targets additionally expose the one-window `NativeApp`
//! lifecycle, Poolrooms-water composition, responsiveness tracing, and
//! optional post-present acceptance witnessing. The native support layer also
//! owns generic bounded drains, latest-wins worker mailboxes, and settled
//! background-write scheduling. Domain state, typed command consequences,
//! storage paths and formats, fixtures, oracles, and acceptance stories remain
//! application concerns.

#[cfg(all(test, target_os = "linux"))]
use arboard as _;
#[cfg(all(test, target_os = "linux"))]
use egui_tester as _;

pub mod cabinet;
pub mod command_guide;
pub mod commands;
#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
mod crash_reports;
pub mod inspector;
pub mod living_wait;
pub mod panel_navigation;

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
mod native;
#[cfg(target_os = "linux")]
mod native_cursor;
#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
pub mod persistence;
#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
pub mod responsiveness;

pub use cabinet::{
    Berth as CabinetBerth, Cabinet, CabinetAction, CabinetEntry, CabinetKey,
    EntryEdit as CabinetEntryEdit, Shelf as CabinetShelf, ShelfBerth as CabinetShelfBerth,
    ShelfEdit as CabinetShelfEdit,
};
pub use inspector::{Inspector, InspectorResponse};
pub use living_wait::LivingWait;

#[cfg(all(
    feature = "egui-test",
    any(target_os = "linux", target_os = "macos", target_os = "windows")
))]
#[doc(hidden)]
pub use crash_reports::native_crash_acceptance;
#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
pub use crash_reports::{CrashProduct, CrashReportSpec};
#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
pub use native::{
    CloseDisposition, NativeApp, NativeWake, ResponsivenessSpec, WindowSpec, run, run_with,
};
#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
pub use persistence::{ScribeOutcome, SettledScribe};
#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
pub use responsiveness::TraceGuard;
