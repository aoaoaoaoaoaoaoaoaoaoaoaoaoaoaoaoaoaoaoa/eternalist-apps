//! Opt-in geometry for the fleet's persistent left inspector.

use egui::{Id, InnerResponse, Response, ScrollArea, Ui};

/// Fleet default for a dense, fixed-width inspector rail.
pub const WIDTH: f32 = dwemer_poolrooms::chrome::INSPECTOR_WIDTH;

/// An optional persistent left rail containing application-owned controls.
///
/// The inspector owns only panel geometry and vertical scrolling. Section
/// order, titles, fold state, actions, water response, and persistence remain
/// application decisions.
#[derive(Clone, Copy, Debug)]
pub struct Inspector {
    panel: Id,
    scroll: Id,
    width: f32,
    offset: Option<f32>,
}

impl Inspector {
    /// Name one inspector and its persistent scroll state.
    pub fn new(id: impl std::hash::Hash) -> Self {
        let panel = Id::new(id);
        Self {
            panel,
            scroll: panel.with("scroll"),
            width: WIDTH,
            offset: None,
        }
    }

    /// Override the fleet width when the product's content proves the need.
    #[must_use]
    pub const fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Override the default scroll identity.
    #[must_use]
    pub fn scroll_id(mut self, id: impl std::hash::Hash) -> Self {
        self.scroll = Id::new(id);
        self
    }

    /// Restore an application-persisted vertical offset.
    #[must_use]
    pub const fn scroll_offset(mut self, offset: f32) -> Self {
        self.offset = Some(offset);
        self
    }

    /// Show the rail inside an application's root UI.
    pub fn show<R>(self, ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> InspectorResponse<R> {
        let panel = egui::Panel::left(self.panel)
            .resizable(false)
            .exact_size(self.width)
            .show_inside(ui, |ui| {
                let mut scroll = ScrollArea::vertical()
                    .id_salt(self.scroll)
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                    .auto_shrink([false, false]);
                if let Some(offset) = self.offset {
                    scroll = scroll.vertical_scroll_offset(offset.max(0.0));
                }
                scroll.show(ui, |ui| {
                    ui.add_space(ui.spacing().item_spacing.x);
                    ui.set_width(ui.available_width());
                    add(ui)
                })
            });
        let InnerResponse { inner, response } = panel;
        InspectorResponse {
            inner: inner.inner,
            response,
            scroll_offset: inner.state.offset.y.max(0.0),
        }
    }
}

/// The application result, panel response, and resulting scroll position.
#[derive(Debug)]
pub struct InspectorResponse<R> {
    pub inner: R,
    pub response: Response,
    pub scroll_offset: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inspector_returns_application_value_without_mandating_sections() {
        let ctx = egui::Context::default();
        let mut result = None;
        let _output = ctx.run_ui(egui::RawInput::default(), |ui| {
            result = Some(Inspector::new("test-inspector").show(ui, |ui| {
                let _label = ui.label("application-owned body");
                17_u8
            }));
        });
        assert_eq!(result.map(|result| result.inner), Some(17));
    }

    #[test]
    fn persisted_offsets_are_refined_to_nonnegative_values() {
        let ctx = egui::Context::default();
        let mut result = None;
        let _output = ctx.run_ui(egui::RawInput::default(), |ui| {
            result = Some(
                Inspector::new("test-inspector")
                    .scroll_offset(-40.0)
                    .show(ui, |_| ()),
            );
        });
        assert_eq!(result.map(|result| result.scroll_offset), Some(0.0));
    }
}
