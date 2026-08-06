# Fresh Application Bootstrap

## 1. Freeze The Product Contract

Name the product's user-owned documents, application-owned data,
configuration, durable state, rebuildable cache, and ephemeral runtime
material. Declare the release-tested, supported, and unclaimed platform
coordinates. On Linux, assign every artifact to its XDG meaning before writing
code.

Define the first useful publication independently from background armament.
Chrome and any available substrate must appear without waiting for indexing,
network acquisition, corpus decoding, or GPU preparation that can arrive
later.

## 2. Establish Rust And Dependencies

Apply `$rust-bootstrap`. Depend on one coherent generation of `egui`,
`egui-wgpu`, `egui-winit`, `winit`, Dwemer Poolrooms, and
`eternalist-apps`. Keep the product's direct Poolrooms dependency when its UI
uses Poolrooms controls.

Start with the narrowest platform feature set actually claimed. The current
fleet coordinate disables egui-winit and winit default features and enables
X11 explicitly.

## 3. Implement The Native Seam

```rust
use dwemer_poolrooms::water::{Frame, Surface};
use eternalist_apps::{NativeApp, TraceGuard, WindowSpec};

struct App {
    water: Surface,
}

impl NativeApp for App {
    const WINDOW: WindowSpec = WindowSpec::new("product", [1_440.0, 920.0]);

    fn draw(&mut self, ui: &mut egui::Ui) {
        let _canvas = egui::CentralPanel::default().show_inside(ui, |ui| {
            let _heading = ui.heading("PRODUCT");
        });
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
    type Observation = ProductObservation;

    #[cfg(feature = "egui-test")]
    fn observe(&self, text_edit_focused: bool) -> Self::Observation {
        self.observation(text_edit_focused)
    }
}

fn run(app: App) -> anyhow::Result<()> {
    let trace = TraceGuard::arm()?;
    let ctx = egui::Context::default();
    dwemer_poolrooms::chrome::install(&ctx);
    let result = eternalist_apps::run(ctx, app);
    trace.flush();
    result
}
```

The application owns construction and publication. The host never discovers
product paths, starts domain workers, or chooses first-run behavior.

## 4. Compose Proved Application Primitives

Begin with the Eternalist primitives that match the product's actual logical
structure. Use raw egui and Poolrooms directly where no shared law exists; do
not invent a generic primitive from a single product merely to make its entry
point shorter.

Add an inspector only when the product has persistent-left-rail semantics:

```rust
let inspector = eternalist_apps::Inspector::new("product-inspector")
    .scroll_offset(self.inspector_scroll)
    .show(ui, |ui| self.inspector(ui));
self.inspector_scroll = inspector.scroll_offset;
self.water.heave(ui.ctx(), inspector.scroll_offset);
```

Use Poolrooms sections inside the body when disclosure is useful. The
application owns section order and persistence. Do not create an empty or
ceremonial inspector.

## 5. Prove The Product

Create a dependency-light product contract crate and an external acceptance
executable before feature sprawl. Follow [verification](verification.md), then
establish the app-owned source, audit, lifecycle, and native acceptance gates
described in [CI](ci.md).
