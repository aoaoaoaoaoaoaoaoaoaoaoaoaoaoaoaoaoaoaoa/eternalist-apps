use super::Exhibit;
use anyhow::Result;
use brass_poolrooms::{
    chrome, egui,
    water::{Domain, Floor, Frame, Surface, Wetness},
};
use eternalist_apps::{NativeApp, ProductIdentity, TraceGuard, WindowSpec};

struct Atelier<A> {
    exhibit: A,
    water: Surface,
}

impl<A: Exhibit> NativeApp for Atelier<A> {
    const PRODUCT: ProductIdentity =
        ProductIdentity::declare("moe.eternalist.atelier", "Eternalist Atelier");
    const RELEASE: &'static str = env!("CARGO_PKG_VERSION");
    const WINDOW: WindowSpec = WindowSpec::new(A::TITLE, A::SIZE);

    fn draw(&mut self, ui: &mut egui::Ui) {
        let basin = ui.max_rect();
        self.water.begin(Domain::basin(basin));
        self.water.set_floor(Some(Floor::shallow(basin)));
        self.exhibit.ui(ui, &mut self.water);
    }

    fn water(
        &mut self,
        ctx: &egui::Context,
        pixels_per_point: f32,
        tooltip_rects: &[egui::Rect],
    ) -> Frame {
        self.water.frame(ctx, pixels_per_point, tooltip_rects, None)
    }

    #[cfg(feature = "egui-test")]
    type Observation = A::Observation;

    #[cfg(feature = "egui-test")]
    fn observe(&self, text_edit_focused: bool) -> Self::Observation {
        self.exhibit.observe(text_edit_focused)
    }
}

pub fn run(exhibit: impl Exhibit + 'static) -> Result<()> {
    let trace = TraceGuard::arm()?;
    let ctx = egui::Context::default();
    chrome::install(&ctx);
    let result = eternalist_apps::run(
        eternalist_apps::Ingress::Desktop,
        ctx,
        Atelier {
            exhibit,
            water: Surface::new(Wetness::Wet),
        },
    );
    trace.flush();
    result
}
