use super::Exhibit;
use anyhow::Result;
use dwemer_poolrooms::{
    chrome, egui,
    water::{Domain, Floor, Frame, Surface, Wetness},
};
use eternalist_apps::{NativeApp, TraceGuard, WindowSpec};

struct Atelier<A> {
    exhibit: A,
    water: Surface,
}

impl<A: Exhibit> NativeApp for Atelier<A> {
    const WINDOW: WindowSpec = WindowSpec::new(A::TITLE, A::SIZE);

    fn draw(&mut self, ui: &mut egui::Ui) {
        let basin = ui.max_rect();
        self.water.begin(Domain::basin(basin));
        self.water.set_floor(Some(Floor::shallow(basin)));
        self.exhibit.ui(ui, &mut self.water);
    }

    fn after_present(&mut self) -> bool {
        false
    }

    fn water(
        &mut self,
        ctx: &egui::Context,
        pixels_per_point: f32,
        tooltip_rects: &[egui::Rect],
    ) -> Frame {
        self.water.frame(ctx, pixels_per_point, tooltip_rects, None)
    }

    fn register_gpu(
        _renderer: &mut egui_wgpu::Renderer,
        _device: &egui_wgpu::wgpu::Device,
        _format: egui_wgpu::wgpu::TextureFormat,
    ) {
    }

    #[cfg(feature = "egui-test")]
    type Observation = ();

    #[cfg(feature = "egui-test")]
    fn observe(&self, _text_edit_focused: bool) -> Self::Observation {}
}

pub fn run(exhibit: impl Exhibit + 'static) -> Result<()> {
    let trace = TraceGuard::arm()?;
    let ctx = egui::Context::default();
    chrome::install(&ctx);
    let result = eternalist_apps::run(
        ctx,
        Atelier {
            exhibit,
            water: Surface::new(Wetness::Wet),
        },
    );
    trace.flush();
    result
}
