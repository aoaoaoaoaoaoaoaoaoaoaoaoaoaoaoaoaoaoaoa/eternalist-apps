//! Opt-in geometry and visibility for a persistent left inspector.
//!
//! This module owns panel placement, scrolling, animated concealment, its F9
//! idiom, and the resulting water forcing. Section structure, commands,
//! persistence, and domain actions remain with the caller.

use brass_poolrooms::{
    chrome::{MechanismSize, Monoglyph, MonoglyphResponse},
    water::{Poke, Surface},
};
use egui::{Id, InnerResponse, Rect, Response, ScrollArea, Ui};

use crate::commands::{Stroke, TOGGLE_INSPECTOR, take};

/// Default width for a dense inspector rail.
pub const WIDTH: f32 = brass_poolrooms::chrome::INSPECTOR_WIDTH;

const ACTUATOR_INSET: f32 = 4.0;
const BOUNDARY_HALF_WIDTH: f32 = 5.0;
const ACTUATOR_LINGER_SECONDS: f64 = 0.24;
const PANEL_SWEEP_IMPULSE: f32 = 1.10;
const MOTION_EPSILON: f32 = 0.01;

/// An optional, animated left rail containing application-owned controls.
///
/// F9 and the small boundary actuator conceal or reveal the complete rail.
/// Visibility is session state keyed by the inspector identity. The caller may
/// restore scroll position, but need not own visibility, animation, shortcut
/// routing, or water coupling.
#[derive(Clone, Copy, Debug)]
pub struct Inspector {
    panel: Id,
    scroll: Id,
    width: f32,
    offset: Option<f32>,
}

impl Inspector {
    /// Name one inspector and its session visibility and scroll state.
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
    ///
    /// A fully concealed inspector does not evaluate `add`; its return is
    /// `R::default()`, the application's empty action. Call
    /// [`InspectorResponse::agitate`] after layout to apply the shared scroll,
    /// actuator, and panel-sweep water law.
    pub fn show<R: Default>(
        self,
        ui: &mut Ui,
        add: impl FnOnce(&mut Ui) -> R,
    ) -> InspectorResponse<R> {
        let ctx = ui.ctx().clone();
        let visibility_id = self.panel.with("visibility");
        let mut expanded = ctx
            .data(|data| data.get_temp::<bool>(visibility_id))
            .unwrap_or(true);
        if ctx.memory(|memory| memory.top_modal_layer().is_none()) {
            match take(&ctx, TOGGLE_INSPECTOR[0]) {
                Stroke::Fresh => expanded = !expanded,
                Stroke::None | Stroke::Repeat => {}
            }
        }

        let available = ui.available_rect_before_wrap();
        let mut scroll_before = None;
        let panel = egui::Panel::left(self.panel)
            .resizable(false)
            .exact_size(self.width)
            .show_collapsible(ui, &mut expanded, |ui| {
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
        let extent =
            (ui.available_rect_before_wrap().left() - available.left()).clamp(0.0, self.width);

        let (inner, response, mut scroll_offset) =
            if let Some(InnerResponse { inner, response }) = panel {
                (inner.inner, Some(response), inner.state.offset.y.max(0.0))
            } else {
                let offset = self.offset.unwrap_or_else(|| {
                    ctx.data(|data| {
                        data.get_temp::<f32>(self.panel.with("scroll-offset"))
                            .unwrap_or(0.0)
                    })
                });
                (R::default(), None, offset)
            };
        if brass_poolrooms::chrome::take_control_wheel(&ctx) {
            let (scroll_id, prior) = scroll_before.expect("visible inspector body did not run");
            scroll_offset = prior.offset.y.max(0.0);
            prior.store(&ctx, scroll_id);
            ctx.request_repaint();
        }
        ctx.data_mut(|data| {
            let _old = data.insert_temp(self.panel.with("scroll-offset"), scroll_offset);
        });

        let actuator = visibility_actuator(&ctx, self.panel, available, extent, expanded);
        if actuator.clicked() {
            expanded = !expanded;
            ctx.request_repaint();
        }
        ctx.data_mut(|data| {
            let _old = data.insert_temp(visibility_id, expanded);
        });

        let sweep = panel_sweep(&ctx, self.panel, available, extent, self.width);

        InspectorResponse {
            inner,
            response,
            scroll_offset,
            expanded,
            extent,
            actuator,
            sweep,
        }
    }
}

fn visibility_actuator(
    ctx: &egui::Context,
    panel: Id,
    available: Rect,
    extent: f32,
    expanded: bool,
) -> VisibilityActuator {
    let edge = available.left() + extent;
    let boundary_center = edge.clamp(
        available.left() + BOUNDARY_HALF_WIDTH,
        available.right() - BOUNDARY_HALF_WIDTH,
    );
    let boundary_rect = Rect::from_center_size(
        egui::pos2(boundary_center, available.center().y),
        egui::vec2(2.0 * BOUNDARY_HALF_WIDTH, available.height()),
    );
    let boundary = egui::Area::new(panel.with("visibility-boundary"))
        .order(egui::Order::Foreground)
        .fixed_pos(boundary_rect.min)
        .show(ctx, |ui| {
            ui.allocate_exact_size(boundary_rect.size(), egui::Sense::CLICK)
                .1
        })
        .inner;

    let now = ctx.input(|input| input.time);
    let pointer = ctx.input(|input| input.pointer.hover_pos());
    let state_id = panel.with("visibility-actuator-state");
    let mut state = ctx
        .data(|data| data.get_temp::<ActuatorState>(state_id))
        .unwrap_or_default();
    let was_visible = state.armed && now <= state.visible_until;
    if boundary.hovered() {
        if !was_visible {
            state.anchor_y = pointer.map_or(available.center().y, |position| position.y);
        }
        state.armed = true;
        state.visible_until = now + ACTUATOR_LINGER_SECONDS;
    }

    let side = MechanismSize::Small.side();
    let actuator_rect = Rect::from_min_size(
        egui::pos2(
            (edge - side / 2.0).clamp(available.left(), available.right() - side),
            (state.anchor_y - side / 2.0).clamp(
                available.top() + ACTUATOR_INSET,
                available.bottom() - side - ACTUATOR_INSET,
            ),
        ),
        egui::vec2(side, side),
    );
    let over_actuator = state.armed
        && pointer.is_some_and(|position| actuator_rect.expand(ACTUATOR_INSET).contains(position));
    if over_actuator {
        state.visible_until = now + ACTUATOR_LINGER_SECONDS;
    }

    let mut button = (state.armed && now <= state.visible_until).then(|| {
        egui::Area::new(panel.with("visibility-actuator"))
            .order(egui::Order::Foreground)
            .fixed_pos(actuator_rect.min)
            .show(ctx, |ui| {
                Monoglyph::new(if expanded { '◀' } else { '▶' })
                    .size(MechanismSize::Small)
                    .focusable(false)
                    .show(ui)
                    .on_hover_text(if expanded {
                        "Hide inspector · F9"
                    } else {
                        "Show inspector · F9"
                    })
            })
            .inner
    });
    if button.as_ref().is_some_and(|button| button.hovered()) {
        state.visible_until = now + ACTUATOR_LINGER_SECONDS;
    }
    if state.armed && now <= state.visible_until {
        ctx.request_repaint_after(std::time::Duration::from_secs_f64(
            (state.visible_until - now).max(0.0),
        ));
    } else {
        state.armed = false;
        button = None;
    }
    ctx.data_mut(|data| {
        let _old = data.insert_temp(state_id, state);
    });

    VisibilityActuator { boundary, button }
}

#[derive(Clone, Copy, Debug, Default)]
struct ActuatorState {
    anchor_y: f32,
    visible_until: f64,
    armed: bool,
}

struct VisibilityActuator {
    boundary: Response,
    button: Option<MonoglyphResponse>,
}

impl VisibilityActuator {
    fn clicked(&self) -> bool {
        self.boundary.clicked() || self.button.as_ref().is_some_and(MonoglyphResponse::clicked)
    }
}

fn panel_sweep(
    ctx: &egui::Context,
    panel: Id,
    available: Rect,
    extent: f32,
    width: f32,
) -> Option<PanelSweep> {
    let motion_id = panel.with("water-extent");
    let prior_extent = ctx
        .data(|data| data.get_temp::<f32>(motion_id))
        .unwrap_or(extent);
    ctx.data_mut(|data| {
        let _old = data.insert_temp(motion_id, extent);
    });
    let travel = extent - prior_extent;
    (travel.abs() > MOTION_EPSILON).then(|| {
        let edge = available.left() + extent;
        PanelSweep {
            rect: Rect::from_min_max(
                egui::pos2(edge - 1.0, available.top()),
                egui::pos2(edge + 1.0, available.bottom()),
            ),
            impulse: PANEL_SWEEP_IMPULSE * travel.abs() / width,
            travel,
        }
    })
}

#[derive(Clone, Copy)]
struct PanelSweep {
    rect: Rect,
    impulse: f32,
    travel: f32,
}

/// Application result, panel geometry, visibility, and water forcing from one
/// [`Inspector`] frame.
pub struct InspectorResponse<R> {
    /// Value returned by the application-owned body, or its empty default while
    /// the inspector is fully concealed.
    pub inner: R,
    /// Egui response covering the complete inspector panel while any portion is
    /// visible.
    pub response: Option<Response>,
    /// Resulting nonnegative vertical offset in logical points.
    pub scroll_offset: f32,
    expanded: bool,
    extent: f32,
    actuator: VisibilityActuator,
    sweep: Option<PanelSweep>,
}

impl<R> InspectorResponse<R> {
    /// Whether the inspector's target state is fully deployed.
    #[must_use]
    pub const fn is_expanded(&self) -> bool {
        self.expanded
    }

    /// Current visible horizontal extent, including an in-progress slide.
    #[must_use]
    pub const fn visible_extent(&self) -> f32 {
        self.extent
    }

    /// Stable pointer target spanning the inspector boundary.
    #[must_use]
    pub fn boundary(&self) -> &Response {
        &self.actuator.boundary
    }

    /// Response for the small conceal/reveal actuator while it is revealed.
    #[must_use]
    pub fn actuator(&self) -> Option<&Response> {
        self.actuator.button.as_deref()
    }

    /// Apply the shared actuator, scroll, and moving-wall water law.
    pub fn agitate(&self, water: &mut Surface) {
        if let Some(actuator) = &self.actuator.button {
            water.monoglyph(actuator);
        }
        water.heave(&self.actuator.boundary.ctx, self.scroll_offset);
        if let Some(sweep) = self.sweep {
            water.poke(
                sweep.rect,
                Poke::slide(sweep.impulse, sweep.travel.signum()),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn viewport() -> Rect {
        Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(600.0, 320.0))
    }

    fn frame(
        ctx: &egui::Context,
        input: egui::RawInput,
        offset: Option<f32>,
        value: &mut u16,
    ) -> (f32, Rect) {
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
                Some(rail.rect)
            });
            outcome = Some((
                response.scroll_offset,
                response.inner.expect("visible inspector omitted its body"),
            ));
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
