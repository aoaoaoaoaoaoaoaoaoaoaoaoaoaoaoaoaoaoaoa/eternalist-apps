use egui::{Context, Event, Id, Rect, Vec2};

use brass_poolrooms::chrome;

const CARD_GUTTER: f32 = 24.0;
const CARD_INSET: i8 = 14;

#[derive(Clone, Copy, Debug)]
pub(crate) struct ScrollExtent {
    pub(crate) height: f32,
}

/// Scrap the pass in which egui sizes a newly shown area.
///
/// egui lays a new area out once invisibly and without interaction, and the
/// next pass hit-tests input against that layout: a click on the area's
/// published geometry would reach the widget beneath it. Discarding the
/// sizing pass reruns layout within the same frame, so the presented and
/// witnessed pass is the interactable one. Call it first inside the area.
pub(crate) fn scrap_sizing_pass(ui: &egui::Ui) {
    if ui.is_sizing_pass() {
        ui.ctx()
            .request_discard("an area's sizing pass is invisible and inert");
    }
}

/// Show `modal` without ever presenting its sizing pass.
pub(crate) fn show_modal<R>(
    modal: egui::Modal,
    ctx: &Context,
    add: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::ModalResponse<R> {
    modal.show(ctx, |ui| {
        scrap_sizing_pass(ui);
        add(ui)
    })
}

pub(crate) fn card_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(chrome::SURFACE)
        .stroke(egui::Stroke::new(1.5_f32, chrome::EDGE_STRONG))
        .corner_radius(2)
        .inner_margin(egui::Margin::same(CARD_INSET))
}

pub(crate) fn scroll_extent(ctx: &Context, fixed_chrome: f32, ceiling: f32) -> ScrollExtent {
    let height =
        (ctx.content_rect().height() - 2.0 * (CARD_GUTTER + f32::from(CARD_INSET)) - fixed_chrome)
            .clamp(0.0, ceiling);
    ScrollExtent { height }
}

#[derive(Debug, Default)]
pub(crate) struct ModalShell {
    open: bool,
    deferred_close_frame: Option<u64>,
    rect: Option<Rect>,
    restore_focus: Option<Id>,
    pending_focus: Option<FocusReturn>,
    wheel: Option<QuarantinedWheel>,
}

#[derive(Debug)]
struct FocusReturn {
    target: Option<Id>,
    closed_frame: u64,
}

#[derive(Debug)]
struct QuarantinedWheel {
    frame: u64,
    events: Vec<Event>,
    smooth_delta: Vec2,
}

impl ModalShell {
    pub(crate) const fn is_open(&self) -> bool {
        self.open
    }

    pub(crate) const fn rect(&self) -> Option<Rect> {
        self.rect
    }

    pub(crate) fn prepare(&mut self, ctx: &Context) {
        self.settle_deferred_close(ctx);
        self.settle_focus(ctx);
    }

    pub(crate) fn open(&mut self, ctx: &Context) {
        self.prepare(ctx);
        if self.open {
            return;
        }
        self.restore_focus = self.pending_focus.take().map_or_else(
            || ctx.memory(egui::Memory::focused),
            |pending| pending.target,
        );
        // Boundary close controls duplicate Escape and the modal shortcut.
        // Relinquish underlay focus instead of turning either boundary or
        // submerged chrome into a continuous Poolrooms tension radiator.
        if let Some(focused) = ctx.memory(egui::Memory::focused) {
            ctx.memory_mut(|memory| memory.surrender_focus(focused));
        }
        self.open = true;
        ctx.request_repaint();
    }

    pub(crate) fn close(&mut self, ctx: &Context) {
        self.deferred_close_frame = None;
        self.close_now(ctx);
    }

    pub(crate) fn toggle(&mut self, ctx: &Context) {
        if self.open {
            self.close(ctx);
        } else {
            self.open(ctx);
        }
    }

    fn close_now(&mut self, ctx: &Context) {
        if !self.open {
            return;
        }
        self.open = false;
        self.rect = None;
        self.pending_focus = Some(FocusReturn {
            target: self.restore_focus.take(),
            closed_frame: ctx.cumulative_frame_nr(),
        });
        ctx.request_repaint();
    }

    pub(crate) fn quarantine_wheel(&mut self, ctx: &Context) {
        if !self.open {
            self.wheel = None;
            return;
        }
        let frame = ctx.cumulative_frame_nr();
        if self
            .wheel
            .as_ref()
            .is_some_and(|wheel| wheel.frame == frame)
        {
            return;
        }
        self.wheel = Some(ctx.input_mut(|input| {
            let mut events = Vec::new();
            input.events.retain(|event| {
                if matches!(event, Event::MouseWheel { .. }) {
                    events.push(event.clone());
                    false
                } else {
                    true
                }
            });
            QuarantinedWheel {
                frame,
                events,
                smooth_delta: std::mem::take(&mut input.smooth_scroll_delta),
            }
        }));
    }

    pub(crate) fn begin_present(&mut self, ctx: &Context) -> bool {
        self.prepare(ctx);
        if !self.open {
            self.rect = None;
            self.wheel = None;
            return false;
        }
        let Some(wheel) = self.wheel.take() else {
            return true;
        };
        if wheel.frame == ctx.cumulative_frame_nr() {
            ctx.input_mut(|input| {
                input.events.extend(wheel.events);
                input.smooth_scroll_delta += wheel.smooth_delta;
            });
        }
        true
    }

    pub(crate) fn finish_present(&mut self, ctx: &Context, rect: Rect, close: bool) {
        consume_wheel(ctx);
        self.rect = Some(rect);
        if close {
            self.deferred_close_frame = Some(ctx.cumulative_frame_nr());
            ctx.request_repaint();
        }
    }

    fn settle_deferred_close(&mut self, ctx: &Context) {
        let due = self
            .deferred_close_frame
            .is_some_and(|frame| frame < ctx.cumulative_frame_nr());
        if due {
            self.deferred_close_frame = None;
            self.close_now(ctx);
        }
    }

    fn settle_focus(&mut self, ctx: &Context) {
        if self.pending_focus.is_some() && ctx.input(focus_return_interdicted) {
            self.pending_focus = None;
            return;
        }
        // Egui admits interaction against the preceding pass's modal layer.
        // One complete nonmodal pass must retire it before the underlying
        // target can re-enter the focus census.
        let due = self.pending_focus.as_ref().is_some_and(|pending| {
            pending.closed_frame.saturating_add(1) < ctx.cumulative_frame_nr()
        });
        if !due {
            if self.pending_focus.is_some() {
                ctx.request_repaint();
            }
            return;
        }
        let pending = self.pending_focus.take();
        if let Some(target) = pending.and_then(|pending| pending.target) {
            ctx.memory_mut(|memory| memory.request_focus(target));
        } else if let Some(focused) = ctx.memory(egui::Memory::focused) {
            ctx.memory_mut(|memory| memory.surrender_focus(focused));
        }
        ctx.request_repaint();
    }
}

fn consume_wheel(ctx: &Context) {
    ctx.input_mut(|input| {
        input
            .events
            .retain(|event| !matches!(event, Event::MouseWheel { .. }));
        input.smooth_scroll_delta = Vec2::ZERO;
    });
}

fn focus_return_interdicted(input: &egui::InputState) -> bool {
    input.pointer.any_pressed()
        || input.events.iter().any(|event| {
            matches!(
                event,
                Event::Copy
                    | Event::Cut
                    | Event::Paste(_)
                    | Event::Text(_)
                    | Event::Key { pressed: true, .. }
                    | Event::AccessKitActionRequest(_)
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    // egui hit-tests each pass's input against the previous pass. Unless a
    // fresh modal's invisible, inert sizing pass is discarded, a click landing
    // right after the modal first appears reaches the widget beneath it.
    #[test]
    fn opening_frame_click_reaches_the_modal() {
        let ctx = Context::default();
        let screen = Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(800.0, 600.0));
        // One frame: (beneath clicked, modal clicked, modal button center).
        let frame = |events: Vec<Event>| {
            let input = egui::RawInput {
                screen_rect: Some(screen),
                events,
                ..egui::RawInput::default()
            };
            let mut witnessed = (false, false, egui::Pos2::ZERO);
            ctx.run_ui(input, |ui| {
                let beneath = ui.add_sized(ui.available_size(), egui::Button::new("beneath"));
                let modal = show_modal(egui::Modal::new(Id::new("probe")), ui.ctx(), |ui| {
                    ui.button("modal")
                })
                .inner;
                witnessed = (beneath.clicked(), modal.clicked(), modal.rect.center());
            })
            .drop_without_applying_deltas();
            witnessed
        };
        let (_, _, at) = frame(Vec::new());
        let press = |pressed| Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        let _pressed = frame(vec![Event::PointerMoved(at), press(true)]);
        let (beneath, modal, _) = frame(vec![press(false)]);
        assert!(modal && !beneath, "beneath: {beneath}, modal: {modal}");
    }
}
