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
`egui-wgpu`, `egui-winit`, `winit`, Brass Poolrooms, and
`eternalist-apps`. Keep the product's direct Poolrooms dependency when its UI
uses Poolrooms controls.

Keep direct `egui-wgpu` dependencies on `default-features = false`; the host
selects exactly Vulkan on Linux, Metal on macOS, and DX12 on Windows. Do not
re-enable wgpu's omnibus defaults in the product. Direct `egui-winit` and
`winit` dependencies should likewise disable defaults and enable only the
window-system coordinate the product claims; Linux applications currently use
X11 explicitly.

## 3. Implement The Native Seam

```rust
use brass_poolrooms::water::{Frame, Surface};
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
    brass_poolrooms::chrome::install(&ctx);
    let result = eternalist_apps::run(ctx, app);
    trace.flush();
    result
}
```

The application owns construction and publication. The host never discovers
product paths, starts domain workers, or chooses first-run behavior.

For native user preferences, define one typed `Configuration`, select the
platform-correct config path, and let `ConfigurationLedger` own strict TOML
admission, settlement, merge, and atomic replacement. Project the same
`SettingSpec` values through contextual controls and `SettingsSheet`. Wire the
ledger deadline through `NativeApp::service_deadline`; absorb completions before
layout; request an explicit reload when the sheet opens or the window regains
focus. A fault must open the sheet and block mutation until a valid reload.
Do not add an independent settings serializer, permissive unknown-key mode, or
filesystem work to `draw`.

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
inspector.agitate(&mut self.water);
```

Use `PanelNavigator` when two or more inspector sections share the active-panel
keyboard grammar. Use Poolrooms `Section` directly when disclosure alone is
needed. The application owns section identity, order, contents, fold defaults,
actions, and persistence. Do not create an empty or ceremonial inspector.

Declare recurring application actions as typed `CommandSpec` values and forge
one `CommandCanon`. Route input, generate button labels, and render
`CommandGuide` from that canon; execute each returned `CommandDispatch` through
the domain. Add Alt mnemonics conservatively. Apply the design language's
[basic-controls checklist](design-language.md#basic-controls) to every
navigable surface. `CommandGuide` supplies only universal keyboard and guide
sections. Author every target-specific `GuideSection` in the product, name it
in user vocabulary, and include it only where its target exists. Physical
control classes never propagate help into applications. The canon owns no
callback bus, availability state, feedback channel, or keymap persistence.

Adopt `Cabinet` only for a genuine persistent, reorderable, one-level shelved
collection. Project product storage into `Cabinet::forge`, retain the returned
`CabinetShelfEdit` between frames, and apply `CabinetAction` values through
domain methods. Choose `show_renamable` and retain `CabinetEntryEdit` only when
the product admits entry renaming; the ordinary `show` surface does not expose
that affordance. Enable the crate's `serde` feature only when direct cabinet
serialization is the honest product projection.

## 5. Prove The Product

Create a dependency-light product contract crate and an external acceptance
executable before feature sprawl. Follow [verification](verification.md), then
establish the app-owned source, audit, lifecycle, and native acceptance gates
described in [CI](ci.md).
