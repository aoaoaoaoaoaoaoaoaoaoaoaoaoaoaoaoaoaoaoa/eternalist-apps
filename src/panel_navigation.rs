//! Focus containment and active-panel traversal for inspector sections.

#![deny(missing_docs)]

use std::hash::Hash;

use dwemer_poolrooms::chrome::{FoldWake, Section};

use crate::commands::{
    NEXT_CONTROL, NEXT_PANEL, PREVIOUS_CONTROL, PREVIOUS_PANEL, Shortcut, Stroke, take,
};

/// Persistent logical state for keyboard traversal among inspector panels.
///
/// Tab and Shift+Tab remain ordinary forward and backward traversal inside the
/// active panel. Control+Tab and Control+Shift+Tab move between panel headers.
/// Pointer engagement makes a panel active. A focused control outside these
/// panels is never admitted into their Tab loop.
#[derive(Debug, Default)]
pub struct PanelNavigator {
    active: Option<egui::Id>,
    prior: Vec<PanelRecord>,
    next: Vec<PanelRecord>,
    backward_loop: Option<egui::Id>,
    backward_entry: Option<egui::Id>,
}

impl PanelNavigator {
    /// Begin one UI pass and consume shared panel-traversal chords.
    ///
    /// The returned frame finalizes itself on drop after the caller has shown
    /// every panel in presentation order. A preceding modal layer suspends
    /// traversal without consuming its keys.
    pub fn frame<'navigator>(&'navigator mut self, ctx: &egui::Context) -> PanelFrame<'navigator> {
        self.next.clear();
        self.take_panel_chords(ctx);
        PanelFrame {
            navigator: self,
            ctx: ctx.clone(),
        }
    }

    /// Stable identity of the currently active panel, when one exists.
    #[must_use]
    pub const fn active(&self) -> Option<egui::Id> {
        self.active
    }

    fn take_panel_chords(&mut self, ctx: &egui::Context) {
        if self.prior.is_empty() || ctx.memory(|memory| memory.top_modal_layer().is_some()) {
            return;
        }
        let focused = ctx.memory(egui::Memory::focused);
        let backward = take_shortcut(ctx, PREVIOUS_PANEL[0]);
        let forward = if backward {
            false
        } else {
            take_shortcut(ctx, NEXT_PANEL[0])
        };
        if backward || forward {
            let active = self
                .active
                .and_then(|active| self.prior.iter().position(|panel| panel.id == active))
                .unwrap_or_default();
            let target = if backward {
                active.checked_sub(1).unwrap_or(self.prior.len() - 1)
            } else {
                (active + 1) % self.prior.len()
            };
            let panel = self.prior[target];
            self.active = Some(panel.id);
            ctx.memory_mut(|memory| {
                memory.move_focus(egui::FocusDirection::None);
                memory.request_focus(panel.header);
            });
            ctx.request_repaint();
            return;
        }
        if focused.is_some() {
            return;
        }
        let active = self
            .active
            .and_then(|active| self.prior.iter().find(|panel| panel.id == active))
            .copied()
            .unwrap_or(self.prior[0]);
        if take_shortcut(ctx, PREVIOUS_CONTROL[0]) {
            ctx.memory_mut(|memory| memory.move_focus(egui::FocusDirection::None));
            self.backward_entry = Some(active.id);
            ctx.request_repaint();
        } else if take_shortcut(ctx, NEXT_CONTROL[0]) {
            ctx.memory_mut(|memory| {
                memory.move_focus(egui::FocusDirection::None);
                memory.request_focus(active.header);
            });
            ctx.request_repaint();
        }
    }

    fn finish(&mut self, ctx: &egui::Context) {
        let previous = self.active;
        std::mem::swap(&mut self.prior, &mut self.next);
        if self
            .active
            .is_none_or(|active| !self.prior.iter().any(|panel| panel.id == active))
        {
            self.active = self.prior.first().map(|panel| panel.id);
        }
        if self
            .backward_loop
            .is_some_and(|panel| !self.prior.iter().any(|candidate| candidate.id == panel))
        {
            self.backward_loop = None;
        }
        if self
            .backward_entry
            .is_some_and(|panel| !self.prior.iter().any(|candidate| candidate.id == panel))
        {
            self.backward_entry = None;
        }
        if self.active != previous {
            ctx.request_repaint();
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct PanelRecord {
    id: egui::Id,
    header: egui::Id,
}

/// One-pass guard used to show every navigable inspector panel.
#[must_use = "a panel frame must remain alive while its sections are shown"]
pub struct PanelFrame<'navigator> {
    navigator: &'navigator mut PanelNavigator,
    ctx: egui::Context,
}

impl PanelFrame<'_> {
    /// Make a panel active before transferring focus into one of its controls.
    ///
    /// Call this in the same UI scope and with the same identity salt used by
    /// [`Self::section`]. A panel omitted from the pass is discarded during
    /// finalization, just like an active panel removed by ordinary layout.
    pub fn activate(&mut self, ui: &egui::Ui, id_salt: impl Hash) {
        let id = ui.make_persistent_id(id_salt);
        if self.navigator.active != Some(id) {
            self.navigator.active = Some(id);
            self.ctx.request_repaint();
        }
    }

    /// Show one Poolrooms disclosure as a keyboard-contained logical panel.
    pub fn section(
        &mut self,
        ui: &mut egui::Ui,
        id_salt: impl Hash + Clone,
        title: &'static str,
        default_open: bool,
        add: impl FnOnce(&mut egui::Ui),
    ) -> PanelResponse {
        let id = ui.make_persistent_id(id_salt.clone());
        let header_id = id.with("header");
        let start_id = id.with("panel-focus-start");
        let end_id = id.with("panel-focus-end");
        assert!(
            self.navigator.next.iter().all(|panel| panel.id != id),
            "duplicate panel ID {id:?}"
        );
        if self.navigator.active.is_none() && self.navigator.next.is_empty() {
            self.navigator.active = Some(id);
        }
        let active = self.navigator.active.is_some_and(|active| active == id);

        if ui.memory(|memory| memory.has_focus(header_id))
            && take_shortcut(ui.ctx(), PREVIOUS_CONTROL[0])
        {
            self.navigator.backward_loop = Some(id);
        }

        let start = ui.interact(
            egui::Rect::from_min_size(ui.cursor().left_top(), egui::Vec2::ZERO),
            start_id,
            egui::Sense::focusable_noninteractive(),
        );
        let loop_backward = start.has_focus() && self.navigator.backward_loop == Some(id);
        if start.has_focus() && !loop_backward {
            ui.memory_mut(|memory| {
                memory.move_focus(egui::FocusDirection::None);
                memory.request_focus(header_id);
            });
            ui.ctx().request_repaint();
        }
        if loop_backward {
            self.navigator.backward_loop = None;
        }

        let section = Section::new(title)
            .default_open(default_open)
            .active(active)
            .show(ui, id_salt, add);
        if section.header.gained_focus() {
            section.header.scroll_to_me(Some(egui::Align::Center));
        }

        let pointer_engaged = ui.input(|input| {
            input.pointer.any_pressed()
                && input
                    .pointer
                    .interact_pos()
                    .is_some_and(|pointer| section.response.rect.contains(pointer))
        });
        if (pointer_engaged || section.header.has_focus()) && self.navigator.active != Some(id) {
            self.navigator.active = Some(id);
            ui.ctx().request_repaint();
        }

        let backward_entry = self.navigator.backward_entry == Some(id);
        let seek_last = loop_backward || backward_entry;
        if backward_entry {
            self.navigator.backward_entry = None;
        }
        if seek_last {
            ui.memory_mut(|memory| {
                memory.request_focus(end_id);
                memory.move_focus(egui::FocusDirection::Previous);
            });
        }
        let end = ui.interact(
            egui::Rect::from_min_size(ui.cursor().left_bottom(), egui::Vec2::ZERO),
            end_id,
            egui::Sense::focusable_noninteractive(),
        );
        if end.has_focus() && !seek_last {
            ui.memory_mut(|memory| {
                memory.move_focus(egui::FocusDirection::None);
                memory.request_focus(header_id);
            });
            ui.ctx().request_repaint();
        } else if seek_last {
            ui.ctx().request_repaint();
        }

        self.navigator.next.push(PanelRecord {
            id,
            header: header_id,
        });
        PanelResponse {
            wake: section.wake,
            response: section.response,
            header: section.header,
            active: self.navigator.active == Some(id),
            activated: section.activated,
        }
    }
}

impl Drop for PanelFrame<'_> {
    fn drop(&mut self) {
        self.navigator.finish(&self.ctx);
    }
}

/// Physical and logical witnesses emitted by one navigable panel.
#[derive(Debug)]
pub struct PanelResponse {
    /// Delayed Poolrooms water forcing from a fold transition.
    pub wake: Option<FoldWake>,
    /// Response covering the complete panel.
    pub response: egui::Response,
    /// Focusable disclosure header.
    pub header: egui::Response,
    /// Whether the panel owns intra-panel Tab traversal.
    pub active: bool,
    /// Whether the disclosure accepted a pointer, accessibility, or exact key activation.
    pub activated: bool,
}

fn take_shortcut(ctx: &egui::Context, shortcut: Shortcut) -> bool {
    take(ctx, shortcut) == Stroke::Fresh
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy)]
    struct Ids {
        headers: [egui::Id; 2],
        options: [[egui::Id; 2]; 2],
        outside: egui::Id,
    }

    fn key(modifiers: egui::Modifiers) -> egui::RawInput {
        egui::RawInput {
            modifiers,
            events: vec![egui::Event::Key {
                key: egui::Key::Tab,
                physical_key: Some(egui::Key::Tab),
                pressed: true,
                repeat: false,
                modifiers,
            }],
            ..egui::RawInput::default()
        }
    }

    fn release(modifiers: egui::Modifiers) -> egui::RawInput {
        egui::RawInput {
            modifiers: egui::Modifiers::NONE,
            events: vec![egui::Event::Key {
                key: egui::Key::Tab,
                physical_key: Some(egui::Key::Tab),
                pressed: false,
                repeat: false,
                modifiers,
            }],
            ..egui::RawInput::default()
        }
    }

    fn pass(ctx: &egui::Context, navigator: &mut PanelNavigator, input: egui::RawInput) -> Ids {
        let mut ids = Ids {
            headers: [egui::Id::NULL; 2],
            options: [[egui::Id::NULL; 2]; 2],
            outside: egui::Id::NULL,
        };
        let _output = ctx.run_ui(input, |ui| {
            let mut panels = navigator.frame(ui.ctx());
            for index in 0..2 {
                let panel = panels.section(
                    ui,
                    ("panel", index),
                    if index == 0 { "FIRST" } else { "SECOND" },
                    true,
                    |ui| {
                        ids.options[index][0] = ui.button("one").id;
                        ids.options[index][1] = ui.button("two").id;
                    },
                );
                ids.headers[index] = panel.header.id;
            }
            drop(panels);
            ids.outside = ui.button("outside").id;
        });
        ids
    }

    fn stroke(
        ctx: &egui::Context,
        navigator: &mut PanelNavigator,
        modifiers: egui::Modifiers,
    ) -> Ids {
        let ids = pass(ctx, navigator, key(modifiers));
        let _released = pass(ctx, navigator, release(modifiers));
        ids
    }

    #[test]
    fn tab_is_caged_and_control_tab_crosses_panels() {
        let ctx = egui::Context::default();
        let mut navigator = PanelNavigator::default();
        let ids = pass(&ctx, &mut navigator, egui::RawInput::default());

        let _ids = stroke(&ctx, &mut navigator, egui::Modifiers::NONE);
        assert_eq!(ctx.memory(egui::Memory::focused), Some(ids.headers[0]));
        let _ids = stroke(&ctx, &mut navigator, egui::Modifiers::NONE);
        assert_eq!(ctx.memory(egui::Memory::focused), Some(ids.options[0][0]));
        let _ids = stroke(&ctx, &mut navigator, egui::Modifiers::NONE);
        assert_eq!(ctx.memory(egui::Memory::focused), Some(ids.options[0][1]));
        let _ids = stroke(&ctx, &mut navigator, egui::Modifiers::NONE);
        assert_eq!(ctx.memory(egui::Memory::focused), Some(ids.headers[0]));
        assert_ne!(ctx.memory(egui::Memory::focused), Some(ids.outside));

        let control = egui::Modifiers::CTRL.plus(egui::Modifiers::COMMAND);
        let _ids = stroke(&ctx, &mut navigator, control);
        assert_eq!(ctx.memory(egui::Memory::focused), Some(ids.headers[1]));
        let _ids = stroke(&ctx, &mut navigator, control.plus(egui::Modifiers::SHIFT));
        assert_eq!(ctx.memory(egui::Memory::focused), Some(ids.headers[0]));
    }

    #[test]
    fn shift_tab_walks_backward_inside_the_active_panel() {
        let ctx = egui::Context::default();
        let mut navigator = PanelNavigator::default();
        let ids = pass(&ctx, &mut navigator, egui::RawInput::default());
        let _ids = stroke(&ctx, &mut navigator, egui::Modifiers::NONE);
        assert_eq!(ctx.memory(egui::Memory::focused), Some(ids.headers[0]));

        let _ids = stroke(&ctx, &mut navigator, egui::Modifiers::SHIFT);
        let _settle = pass(&ctx, &mut navigator, egui::RawInput::default());
        assert_eq!(ctx.memory(egui::Memory::focused), Some(ids.options[0][1]));
        assert_ne!(ctx.memory(egui::Memory::focused), Some(ids.outside));
    }

    #[test]
    fn removing_the_active_panel_promotes_the_first_survivor() {
        let ctx = egui::Context::default();
        let mut navigator = PanelNavigator::default();
        let _ids = pass(&ctx, &mut navigator, egui::RawInput::default());
        let control = egui::Modifiers::CTRL.plus(egui::Modifiers::COMMAND);
        let _ids = stroke(&ctx, &mut navigator, control);

        let mut remaining = egui::Id::NULL;
        let _output = ctx.run_ui(egui::RawInput::default(), |ui| {
            remaining = ui.make_persistent_id(("panel", 0));
            let mut panels = navigator.frame(ui.ctx());
            let _first = panels.section(ui, ("panel", 0), "FIRST", true, |ui| {
                let _option = ui.button("one");
            });
        });

        assert_eq!(navigator.active(), Some(remaining));
    }

    #[test]
    fn programmatic_focus_transfer_activates_its_panel() {
        let ctx = egui::Context::default();
        let mut navigator = PanelNavigator::default();
        let _ids = pass(&ctx, &mut navigator, egui::RawInput::default());

        let mut second = egui::Id::NULL;
        let _output = ctx.run_ui(egui::RawInput::default(), |ui| {
            let mut panels = navigator.frame(ui.ctx());
            second = ui.make_persistent_id(("panel", 1));
            panels.activate(ui, ("panel", 1));
            for index in 0..2 {
                let _panel = panels.section(
                    ui,
                    ("panel", index),
                    if index == 0 { "FIRST" } else { "SECOND" },
                    true,
                    |ui| {
                        let response = ui.button("one");
                        if index == 1 {
                            response.request_focus();
                        }
                    },
                );
            }
        });

        assert_eq!(navigator.active(), Some(second));
    }

    #[test]
    #[should_panic(expected = "duplicate panel ID")]
    fn duplicate_panel_identity_is_rejected() {
        let ctx = egui::Context::default();
        let mut navigator = PanelNavigator::default();
        let _output = ctx.run_ui(egui::RawInput::default(), |ui| {
            let mut panels = navigator.frame(ui.ctx());
            let _first = panels.section(ui, "panel", "FIRST", true, |_| {});
            let _second = panels.section(ui, "panel", "SECOND", true, |_| {});
        });
    }

    #[test]
    fn panel_traversal_relinquishes_overmodified_tab() {
        let ctx = egui::Context::default();
        let mut navigator = PanelNavigator::default();
        let _ids = pass(&ctx, &mut navigator, egui::RawInput::default());
        let first = navigator.active();
        let modifiers = egui::Modifiers::CTRL
            .plus(egui::Modifiers::COMMAND)
            .plus(egui::Modifiers::ALT);
        let _ids = pass(&ctx, &mut navigator, key(modifiers));

        assert_eq!(navigator.active(), first);
        assert!(ctx.input(|input| input.key_pressed(egui::Key::Tab)));
    }

    #[test]
    fn panel_traversal_does_not_bleed_through_a_modal_layer() {
        let ctx = egui::Context::default();
        let mut navigator = PanelNavigator::default();
        let _ids = pass(&ctx, &mut navigator, egui::RawInput::default());
        let first = navigator.active();
        let modal = || egui::Modal::new(egui::Id::new("panel-barrier"));
        let _prime = ctx.run_ui(egui::RawInput::default(), |ui| {
            let _modal = modal().show(ui.ctx(), |ui| ui.label("modal"));
        });

        let modifiers = egui::Modifiers::CTRL.plus(egui::Modifiers::COMMAND);
        let _stroke = ctx.run_ui(key(modifiers), |ui| {
            let mut panels = navigator.frame(ui.ctx());
            let _first = panels.section(ui, ("panel", 0), "FIRST", true, |_| {});
            let _second = panels.section(ui, ("panel", 1), "SECOND", true, |_| {});
            let _modal = modal().show(ui.ctx(), |ui| ui.label("modal"));
        });

        assert_eq!(navigator.active(), first);
        assert!(ctx.input(|input| input.key_pressed(egui::Key::Tab)));
    }
}
