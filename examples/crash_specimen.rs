//! Deliberately failing native specimen for the opt-in crash-path acceptance.

#![expect(
    unused_crate_dependencies,
    reason = "the acceptance specimen shares this package's native host dependencies"
)]

#[cfg(all(target_os = "linux", feature = "egui-test"))]
use std::{path::PathBuf, time::Instant};

#[cfg(all(target_os = "linux", feature = "egui-test"))]
use anyhow::{Context as _, Result};
#[cfg(all(target_os = "linux", feature = "egui-test"))]
use brass_poolrooms::{
    chrome,
    water::{Domain, Floor, Frame, Surface, Wetness},
};
#[cfg(all(target_os = "linux", feature = "egui-test"))]
use eternalist_apps::{CrashReportSpec, NativeApp, ProductIdentity, WindowSpec};

#[cfg(all(target_os = "linux", feature = "egui-test"))]
struct CrashSpecimen {
    detonate: bool,
    armed: bool,
    water: Surface,
}

#[cfg(all(target_os = "linux", feature = "egui-test"))]
impl NativeApp for CrashSpecimen {
    const PRODUCT: ProductIdentity = eternalist_apps::ACCEPTANCE;
    const RELEASE: &'static str = env!("CARGO_PKG_VERSION");
    const WINDOW: WindowSpec = WindowSpec::new("Eternalist · crash-path specimen", [720.0, 480.0]);

    fn crash_reports(_ingress: &eternalist_apps::Ingress) -> Result<Option<CrashReportSpec>> {
        let Some(state) = std::env::var_os("ETERNALIST_CRASH_STATE").map(PathBuf::from) else {
            return Ok(None);
        };
        let Ok(endpoint) = std::env::var("ETERNALIST_CRASH_INTAKE") else {
            return Ok(None);
        };
        Ok(Some(CrashReportSpec::acceptance(
            Self::PRODUCT,
            Self::RELEASE,
            state,
            endpoint,
        )))
    }

    fn draw(&mut self, ui: &mut egui::Ui) {
        let basin = ui.max_rect();
        self.water.begin(Domain::basin(basin));
        self.water.set_floor(Some(Floor::shallow(basin)));
        let _body = egui::CentralPanel::default().show(ui, |ui| {
            ui.label(chrome::title("CRASH-PATH SPECIMEN"));
            ui.label(chrome::muted("release acceptance apparatus"));
        });
    }

    fn after_present(&mut self) -> bool {
        self.armed = self.detonate;
        false
    }

    fn service_deadline(&self, now: Instant) -> Option<Instant> {
        self.armed.then_some(now)
    }

    fn service_deadline_reached(&mut self, _now: Instant) -> bool {
        panic!("deliberate crash-report acceptance detonation")
    }

    fn water(
        &mut self,
        ctx: &egui::Context,
        pixels_per_point: f32,
        tooltip_rects: &[egui::Rect],
    ) -> Frame {
        self.water.frame(ctx, pixels_per_point, tooltip_rects, None)
    }

    type Observation = bool;

    fn observe(&self, _text_edit_focused: bool) -> Self::Observation {
        true
    }
}

#[cfg(all(target_os = "linux", feature = "egui-test"))]
fn main() -> Result<()> {
    let detonate = std::env::args_os().any(|argument| argument == "--detonate");
    let ctx = egui::Context::default();
    chrome::install(&ctx);
    eternalist_apps::run(
        eternalist_apps::Ingress::Desktop,
        ctx,
        CrashSpecimen {
            detonate,
            armed: false,
            water: Surface::new(Wetness::Wet),
        },
    )
    .context("run crash-path specimen")
}

#[cfg(not(all(target_os = "linux", feature = "egui-test")))]
fn main() {}
