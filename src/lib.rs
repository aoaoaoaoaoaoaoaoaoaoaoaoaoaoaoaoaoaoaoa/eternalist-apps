//! Reusable high-level application primitives and a native lifecycle for
//! Eternalist-style egui products.
//!
//! [`ApplicationHeader`], [`Inspector`], [`LivingWait`], [`cabinet`], [`commands`],
//! [`command_guide`], [`panel_navigation`], and [`settings`] are renderer-neutral
//! logical primitives. Native targets additionally expose the one-window `NativeApp`
//! lifecycle, Poolrooms-water composition, responsiveness tracing, and
//! optional post-present acceptance witnessing. The native support layer also
//! owns generic bounded drains, latest-wins worker mailboxes, and settled
//! background-write scheduling. Domain state, typed command consequences,
//! product schemas and storage paths, fixtures, oracles, and acceptance stories
//! remain application concerns. [`configuration`] owns the strict TOML mechanics
//! for native application settings. [`ProductIdentity`] is the one declaration
//! from which a product's platform directories and crash-report identity derive.

#[cfg(all(test, target_os = "linux"))]
use arboard as _;
#[cfg(all(test, target_os = "linux"))]
use egui_tester as _;

pub mod application_header;
pub mod cabinet;
pub mod command_guide;
pub mod commands;
#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
pub mod configuration;
#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
mod crash_reports;
pub mod inspector;
pub mod living_wait;
mod modal;
pub mod panel_navigation;
mod product;
pub mod settings;
pub mod witness;

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
mod native;
#[cfg(target_os = "linux")]
mod native_cursor;
#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
mod paths;
#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
pub mod persistence;
#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
pub mod responsiveness;

pub use application_header::{ApplicationHeader, ApplicationHeaderResponse};
pub use cabinet::{
    Berth as CabinetBerth, Cabinet, CabinetAction, CabinetEntry, CabinetKey, CabinetResponse,
    EntryEdit as CabinetEntryEdit, Shelf as CabinetShelf, ShelfBerth as CabinetShelfBerth,
    ShelfEdit as CabinetShelfEdit,
};
pub use inspector::{Inspector, InspectorResponse};
pub use living_wait::LivingWait;
pub use product::ProductIdentity;

/// The GPU seam consumed by [`NativeApp::register_gpu`]; products must not pin it themselves.
#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
pub use egui_wgpu;

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
pub use crash_reports::CrashReportSpec;
#[cfg(all(
    feature = "egui-test",
    any(target_os = "linux", target_os = "macos", target_os = "windows")
))]
#[doc(hidden)]
pub use crash_reports::{ACCEPTANCE, native_crash_acceptance};
#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
pub use native::{
    CloseDisposition, NativeApp, NativeWake, ResponsivenessSpec, WindowSpec, run, run_with,
};
#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
pub use paths::ApplicationPaths;
#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
pub use persistence::{ScribeOutcome, SettledScribe};
#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
pub use responsiveness::TraceGuard;
