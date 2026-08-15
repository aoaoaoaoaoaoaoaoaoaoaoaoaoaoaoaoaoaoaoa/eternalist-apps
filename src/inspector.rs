//! Opt-in geometry for a persistent left inspector.
//!
//! This module owns panel placement and scroll mechanics only. Section
//! structure, commands, persistence, and water forcing remain with the caller.

use egui::{Id, InnerResponse, Response, ScrollArea, Ui};

/// Default width for a dense inspector rail.
pub const WIDTH: f32 = brass_poolrooms::chrome::INSPECTOR_WIDTH;

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
    pub fn new(id: impl egui::AsId) -> Self {
        let panel = Id::new(id);
        Self {
            panel,
            scroll: panel.with("scroll"),
            width: WIDTH,
            offset: None,
        }
    }

    /// Override the standard width when the product's content proves the need.
    #[must_use]
    pub const fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Override the default scroll identity.
    #[must_use]
    pub fn scroll_id(mut self, id: impl egui::AsId) -> Self {
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
        let mut scroll_before = None;
        let panel = egui::Panel::left(self.panel)
            .resizable(false)
            .exact_size(self.width)
            .show(ui, |ui| {
                let scroll_id = ui.make_persistent_id(egui::IdSalt::new(self.scroll));
                scroll_before = Some((
                    scroll_id,
                    egui::scroll_area::State::load(ui.ctx(), scroll_id).unwrap_or_default(),
                ));
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
        let mut scroll_offset = inner.state.offset.y.max(0.0);
        if brass_poolrooms::chrome::take_control_wheel(ui.ctx()) {
            let (scroll_id, prior) = scroll_before.expect("inspector body did not run");
            scroll_offset = prior.offset.y.max(0.0);
            prior.store(ui.ctx(), scroll_id);
            ui.ctx().request_repaint();
        }
        InspectorResponse {
            inner: inner.inner,
            response,
            scroll_offset,
        }
    }
}

/// The application result, panel response, and resulting scroll position.
#[derive(Debug)]
pub struct InspectorResponse<R> {
    /// Value returned by the application-owned inspector body.
    pub inner: R,
    /// Egui response covering the complete inspector panel.
    pub response: Response,
    /// Resulting nonnegative vertical offset in logical points.
    pub scroll_offset: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn viewport() -> egui::Rect {
        egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(600.0, 320.0))
    }

    fn frame(
        ctx: &egui::Context,
        input: egui::RawInput,
        offset: Option<f32>,
        value: &mut u16,
    ) -> (f32, egui::Rect) {
        let mut outcome = None;
        ctx.run_ui(input, |ui| {
            let mut inspector = Inspector::new("wheel-regression");
            if let Some(offset) = offset {
                inspector = inspector.scroll_offset(offset);
            }
            let response = inspector.show(ui, |ui| {
                ui.add_space(200.0);
                let rail = brass_poolrooms::chrome::Rail::new(value, 1..=12).show(ui);
                ui.add_space(400.0);
                rail.rect
            });
            outcome = Some((response.scroll_offset, response.inner));
        })
        .drop_without_applying_deltas();
        outcome.expect("inspector did not render")
    }

    fn input(events: Vec<egui::Event>) -> egui::RawInput {
        egui::RawInput {
            screen_rect: Some(viewport()),
            events,
            ..egui::RawInput::default()
        }
    }

    #[test]
    fn rail_wheel_preserves_enclosing_inspector_scroll() {
        let ctx = egui::Context::default();
        let mut value = 6;
        let (prior_offset, rail) = frame(&ctx, input(Vec::new()), Some(160.0), &mut value);
        assert!(prior_offset > 100.0);

        let wheel = vec![
            egui::Event::PointerMoved(rail.center()),
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Line,
                delta: egui::vec2(0.0, 1.0),
                phase: egui::TouchPhase::Move,
                modifiers: egui::Modifiers::NONE,
            },
        ];
        let (claimed_offset, _) = frame(&ctx, input(wheel), None, &mut value);
        let (settled_offset, _) = frame(&ctx, input(Vec::new()), None, &mut value);

        // A ScrollArea scopes its salt through the containing panel. Loading
        // the raw salt here silently produced a default state, so a rail's
        // wheel claim reset the inspector to its origin.
        assert_eq!(value, 7);
        assert!((claimed_offset - prior_offset).abs() <= f32::EPSILON);
        assert!((settled_offset - prior_offset).abs() <= f32::EPSILON);
    }
}
