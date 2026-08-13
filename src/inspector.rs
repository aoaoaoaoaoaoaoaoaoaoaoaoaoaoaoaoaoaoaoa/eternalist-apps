//! Opt-in geometry for a persistent left inspector.
//!
//! This module owns panel placement and scroll mechanics only. Section
//! structure, commands, persistence, and water forcing remain with the caller.

use egui::{Id, InnerResponse, Response, ScrollArea, Ui};

/// Default width for a dense inspector rail.
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

    /// Override the standard width when the product's content proves the need.
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
        let scroll_before = egui::scroll_area::State::load(ui.ctx(), self.scroll);
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
        let mut scroll_offset = inner.state.offset.y.max(0.0);
        if dwemer_poolrooms::chrome::take_control_wheel(ui.ctx()) {
            let prior = scroll_before.unwrap_or_default();
            scroll_offset = prior.offset.y.max(0.0);
            prior.store(ui.ctx(), inner.id);
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
