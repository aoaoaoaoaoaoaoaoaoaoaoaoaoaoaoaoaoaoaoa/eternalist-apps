use dwemer_poolrooms::{egui, water::Surface};

pub trait Exhibit {
    const TITLE: &'static str;
    #[cfg(not(target_arch = "wasm32"))]
    const SIZE: [f64; 2];
    #[cfg(target_arch = "wasm32")]
    const CANVAS_ID: &'static str;
    #[cfg(target_arch = "wasm32")]
    const READY_MESSAGE: &'static str;

    fn ui(&mut self, ui: &mut egui::Ui, water: &mut Surface);
}

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
mod native;
#[cfg(target_arch = "wasm32")]
mod web;

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
pub use native::run;
#[cfg(target_arch = "wasm32")]
pub use web::run;
