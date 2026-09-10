//! The Drawer: the Inspector's handheld disposition.
//!
//! A Drawer rises from the bottom edge and shows one Panel at a time on a
//! phone, or as many as fit on a wider handheld. A horizontal swipe across
//! the body pages between Panels; the center arrow lowers the Drawer to a
//! title bar or raises it again. Panel structure, persistence, and domain
//! actions remain with the caller, exactly as for the docked Inspector.

use crate::witness::{self, ApplicationTarget};
use brass_poolrooms::chrome::{self, MechanismSize, Monoglyph, MonoglyphResponse, Symbol};
use egui::{Id, Rect, Ui};

/// Logical width one Panel column claims before a second column fits.
const PANEL_WIDTH: f32 = 340.0;
/// Height of the lowered Drawer: its title bar alone.
const LOWERED_HEIGHT: f32 = 42.0;
/// Least height of the raised Drawer.
const MIN_RAISED_HEIGHT: f32 = 220.0;
/// Horizontal travel that pages to the adjacent Panel.
const SWIPE_THRESHOLD: f32 = 64.0;

/// The bottom Inspector disposition for handhelds.
#[derive(Clone, Copy, Debug)]
pub struct Drawer<'a> {
    id: Id,
    roster: &'a [&'a str],
    initial_panel: usize,
}

#[derive(Clone, Copy, Debug)]
struct State {
    first: usize,
    raised: bool,
    swipe: Option<Swipe>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            first: 0,
            raised: true,
            swipe: None,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Swipe {
    device: egui::TouchDeviceId,
    id: egui::TouchId,
    origin: egui::Pos2,
}

impl<'a> Drawer<'a> {
    /// Name one Drawer and the Panels it pages through, in order.
    pub fn new(id: impl egui::AsId, roster: &'a [&'a str]) -> Self {
        Self {
            id: Id::new(id),
            roster,
            initial_panel: 0,
        }
    }

    /// The Panel shown when this Drawer has no session state yet.
    #[must_use]
    pub const fn initial_panel(mut self, panel: usize) -> Self {
        self.initial_panel = panel;
        self
    }

    /// Show the Drawer inside an application's root UI.
    ///
    /// `add` renders one Panel by its index in the roster. Feed
    /// [`DrawerResponse::controls`] to the water like any other mechanisms.
    pub fn show(self, ui: &mut Ui, mut add: impl FnMut(&mut Ui, usize)) -> DrawerResponse {
        assert!(
            !self.roster.is_empty(),
            "a Drawer needs at least one Panel to show"
        );
        let ctx = ui.ctx().clone();
        let state_id = self.id.with("state");
        let mut state = ctx
            .data(|data| data.get_temp::<State>(state_id))
            .unwrap_or(State {
                first: self.initial_panel,
                ..State::default()
            });
        let available = ui.available_rect_before_wrap();
        let columns = columns_for(available.width()).clamp(1, self.roster.len());
        let last_start = self.roster.len().saturating_sub(columns);
        state.first = state.first.min(last_start);

        let maximum = (available.height() * 0.68).max(MIN_RAISED_HEIGHT);
        let raised_height = (available.height() * 0.44).clamp(MIN_RAISED_HEIGHT, maximum);
        let lowered = egui::Panel::bottom(self.id.with("lowered"))
            .resizable(false)
            .exact_size(LOWERED_HEIGHT)
            .show_separator_line(true);
        let raised = egui::Panel::bottom(self.id.with("raised"))
            .resizable(false)
            .exact_size(raised_height)
            .show_separator_line(true);

        let mut showing_raised = state.raised;
        let mut requested = None;
        let mut controls = Vec::with_capacity(3);
        let roster = self.roster;
        let state_ref = &mut state;
        let panel = egui::Panel::show_switched(
            ui,
            &mut showing_raised,
            lowered,
            raised,
            |ui, showing_raised| {
                if showing_raised {
                    show_raised(
                        ui,
                        roster,
                        state_ref,
                        columns,
                        last_start,
                        &mut requested,
                        &mut controls,
                        &mut add,
                    );
                } else {
                    let (_title, raise, ()) = navigation_row(
                        ui,
                        self.id.with("lowered-row"),
                        |ui| {
                            ui.label(chrome::section_title(
                                roster[state_ref.first].to_uppercase(),
                            ))
                        },
                        |ui| {
                            Monoglyph::symbol(Symbol::ArrowUp)
                                .show(ui)
                                .on_hover_text("Raise the Inspector")
                        },
                        |_ui| {},
                    );
                    witness::response(ui, ApplicationTarget::DrawerRaise, &raise);
                    if raise.clicked() {
                        requested = Some(true);
                    }
                    controls.push(raise);
                }
            },
        );
        state.raised = requested.unwrap_or(showing_raised);
        if state.raised != showing_raised {
            ctx.request_repaint();
        }
        ctx.data_mut(|data| {
            let _prior = data.insert_temp(state_id, state);
        });
        DrawerResponse {
            controls,
            domain: panel.response.rect,
            panel: state.first,
            raised: state.raised,
        }
    }
}

#[allow(
    clippy::too_many_arguments,
    reason = "one Drawer frame threads its whole state through the raised body"
)]
fn show_raised(
    ui: &mut Ui,
    roster: &[&str],
    state: &mut State,
    columns: usize,
    last_start: usize,
    requested: &mut Option<bool>,
    controls: &mut Vec<MonoglyphResponse>,
    add: &mut impl FnMut(&mut Ui, usize),
) {
    let shown_end = (state.first + columns).min(roster.len());
    let position = if columns == 1 {
        format!(
            "{} · {}/{}",
            roster[state.first].to_uppercase(),
            state.first + 1,
            roster.len()
        )
    } else {
        format!(
            "{} · {}–{}/{}",
            roster[state.first].to_uppercase(),
            state.first + 1,
            shown_end,
            roster.len()
        )
    };
    let previous_enabled = state.first > 0;
    let next_enabled = state.first < last_start;
    let (previous, lower, next) = navigation_row(
        ui,
        Id::new(("eternalist-drawer-raised-row", roster.len())),
        |ui| {
            let previous = ui
                .add_enabled_ui(previous_enabled, |ui| {
                    Monoglyph::symbol(Symbol::ArrowLeft).show(ui)
                })
                .inner
                .on_hover_text("Previous Inspector panel");
            let _position = ui.label(chrome::section_title(&position));
            previous
        },
        |ui| {
            Monoglyph::symbol(Symbol::ArrowDown)
                .show(ui)
                .on_hover_text("Lower the Inspector")
        },
        |ui| {
            ui.add_enabled_ui(next_enabled, |ui| {
                Monoglyph::symbol(Symbol::ArrowRight).show(ui)
            })
            .inner
            .on_hover_text("Next Inspector panel")
        },
    );
    witness::response(ui, ApplicationTarget::DrawerPrevious, &previous);
    witness::response(ui, ApplicationTarget::DrawerLower, &lower);
    witness::response(ui, ApplicationTarget::DrawerNext, &next);
    if previous.clicked() {
        let _stepped = step(state, last_start, -1);
    }
    if next.clicked() {
        let _stepped = step(state, last_start, 1);
    }
    if lower.clicked() {
        *requested = Some(false);
    }
    controls.extend([previous, lower, next]);

    let body = ui.available_rect_before_wrap();
    let mut scroll_ids = Vec::with_capacity(columns);
    ui.columns(columns, |columns_ui| {
        for (column, panel_ui) in columns_ui.iter_mut().enumerate() {
            let panel = state.first + column;
            let _title = panel_ui.label(chrome::section_title(roster[panel].to_uppercase()));
            let _separator = panel_ui.separator();
            let scroll = egui::ScrollArea::vertical()
                .id_salt(("eternalist-drawer-panel", panel))
                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded)
                .scroll_source(egui::scroll_area::ScrollSource::ALL)
                .auto_shrink([false, false])
                .show(panel_ui, |ui| {
                    ui.set_width(ui.available_width());
                    add(ui, panel);
                });
            scroll_ids.push(scroll.id.with("area"));
        }
    });
    page_by_swipe(ui, state, body, last_start, &scroll_ids);
}

/// A horizontal swipe across the body pages between Panels, unless a control
/// or a drag-and-drop payload has claimed the touch.
fn page_by_swipe(ui: &Ui, state: &mut State, body: Rect, last_start: usize, scroll_ids: &[Id]) {
    let touches = ui.input(|input| {
        input
            .events
            .iter()
            .filter_map(|event| match event {
                egui::Event::Touch {
                    device_id,
                    id,
                    phase,
                    pos,
                    ..
                } => Some((*device_id, *id, *phase, *pos)),
                _ => None,
            })
            .collect::<Vec<_>>()
    });
    let claimed = egui::DragAndDrop::has_any_payload(ui.ctx())
        || ui
            .ctx()
            .dragged_id()
            .or_else(|| ui.ctx().drag_stopped_id())
            .is_some_and(|id| !scroll_ids.contains(&id));
    if claimed {
        state.swipe = None;
        return;
    }
    for (device, id, phase, pos) in touches {
        let active = state
            .swipe
            .is_some_and(|swipe| swipe.device == device && swipe.id == id);
        match phase {
            egui::TouchPhase::Start if body.contains(pos) && state.swipe.is_none() => {
                state.swipe = Some(Swipe {
                    device,
                    id,
                    origin: pos,
                });
            }
            egui::TouchPhase::End if active => {
                if let Some(swipe) = state.swipe.take() {
                    let delta = pos - swipe.origin;
                    if delta.x.abs() >= SWIPE_THRESHOLD && delta.x.abs() > delta.y.abs() * 1.25 {
                        let _stepped = step(state, last_start, if delta.x < 0.0 { 1 } else { -1 });
                        ui.ctx().request_repaint();
                    }
                }
            }
            egui::TouchPhase::Cancel if active => {
                state.swipe = None;
            }
            egui::TouchPhase::Start
            | egui::TouchPhase::Move
            | egui::TouchPhase::End
            | egui::TouchPhase::Cancel => {}
        }
    }
}

fn navigation_row<L, C, R>(
    ui: &mut Ui,
    id: Id,
    left: impl FnOnce(&mut Ui) -> L,
    center: impl FnOnce(&mut Ui) -> C,
    right: impl FnOnce(&mut Ui) -> R,
) -> (L, C, R) {
    let side = MechanismSize::Large.side();
    let (rect, _response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), side), egui::Sense::hover());
    let mut left_ui = ui.new_child(
        egui::UiBuilder::new()
            .id_salt(id.with("left"))
            .max_rect(rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    let left = left(&mut left_ui);
    let center_rect = Rect::from_center_size(rect.center(), egui::Vec2::splat(side));
    let mut center_ui = ui.new_child(
        egui::UiBuilder::new()
            .id_salt(id.with("center"))
            .max_rect(center_rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    let center = center(&mut center_ui);
    let mut right_ui = ui.new_child(
        egui::UiBuilder::new()
            .id_salt(id.with("right"))
            .max_rect(rect)
            .layout(egui::Layout::right_to_left(egui::Align::Center)),
    );
    let right = right(&mut right_ui);
    (left, center, right)
}

/// How many Panel columns a width holds, at least one.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a clamped nonnegative column count is exact in practical widths"
)]
fn columns_for(width: f32) -> usize {
    (width / PANEL_WIDTH).floor().max(1.0) as usize
}

fn step(state: &mut State, last_start: usize, displacement: isize) -> bool {
    let prior = state.first;
    state.first = state
        .first
        .saturating_add_signed(displacement)
        .min(last_start);
    state.first != prior
}

/// Geometry, mechanisms, and paging from one [`Drawer`] frame.
pub struct DrawerResponse {
    /// The navigation mechanisms shown this frame, for water coupling.
    pub controls: Vec<MonoglyphResponse>,
    /// The rectangle the Drawer occupies.
    pub domain: Rect,
    /// Index into the roster of the first Panel shown.
    pub panel: usize,
    /// Whether the Drawer is raised rather than lowered to its title bar.
    pub raised: bool,
}
