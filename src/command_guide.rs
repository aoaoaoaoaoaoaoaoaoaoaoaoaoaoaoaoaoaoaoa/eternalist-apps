//! Modal command guidance rendered from the same canon that routes input.

#![deny(missing_docs)]

use std::fmt::Debug;

use dwemer_poolrooms::chrome::{self, Keycap, MechanismSize, Monoglyph, MonoglyphResponse, Symbol};

use crate::commands::{
    ACTIVATE, ADJUST, BOUNDS, CommandCanon, CommandScope, CommandSpec, CommandStatus,
    HELP_SHORTCUTS, NEXT_CONTROL, NEXT_PANEL, PREVIOUS_CONTROL, PREVIOUS_PANEL, Shortcut, Stroke,
    UNWIND, take,
};

const GUIDE_NAME_SIZE: f32 = 15.0;
const GUIDE_DETAIL_SIZE: f32 = 14.0;
const GUIDE_COLUMN_SPACING: f32 = 10.0;
const GUIDE_ROW_SPACING: f32 = 4.0;
const GUIDE_SECTION_SPACING: f32 = 9.0;

#[derive(Clone, Copy, Debug)]
struct GuideColumns {
    bindings: f32,
    action: f32,
    consequence: f32,
}

impl GuideColumns {
    fn for_width(width: f32) -> Self {
        let bindings = (width * 0.18).clamp(96.0, 132.0);
        let action = (width * 0.22).clamp(112.0, 164.0);
        Self {
            bindings,
            action,
            consequence: (width - bindings - action - 2.0 * GUIDE_COLUMN_SPACING).max(96.0),
        }
    }
}

const KEYBOARD_GESTURES: [GuideGesture; 4] = [
    GuideGesture::new(
        "Next control",
        "Moves focus forward through the current keyboard region.",
        &NEXT_CONTROL,
    ),
    GuideGesture::new(
        "Previous control",
        "Moves focus backward through the current keyboard region.",
        &PREVIOUS_CONTROL,
    ),
    GuideGesture::new(
        "Activate",
        "Presses or toggles the focused control.",
        &ACTIVATE,
    ),
    GuideGesture::new(
        "Close current layer",
        "Closes only the topmost modal, popup, or transient layer.",
        &UNWIND,
    ),
];
const PANEL_GESTURES: [GuideGesture; 2] = [
    GuideGesture::new(
        "Next panel",
        "Moves focus to the next inspector panel.",
        &NEXT_PANEL,
    ),
    GuideGesture::new(
        "Previous panel",
        "Moves focus to the previous inspector panel.",
        &PREVIOUS_PANEL,
    ),
];
const RAIL_GESTURES: [GuideGesture; 2] = [
    GuideGesture::new(
        "Adjust rail",
        "Changes a focused rail by one station; hovered rails also accept the wheel.",
        &ADJUST,
    ),
    GuideGesture::new(
        "Rail bounds",
        "Moves a focused rail directly to its first or last admissible station.",
        &BOUNDS,
    ),
];
const GUIDE_GESTURES: [GuideGesture; 1] = [GuideGesture::new(
    "Toggle this guide",
    "Opens from application chrome or the keyboard.",
    &HELP_SHORTCUTS,
)];
const GUIDE_SECTION: GuideSection = GuideSection::new("GUIDE", &GUIDE_GESTURES);

/// Baseline keyboard grammar shared by Eternalist applications.
pub const KEYBOARD_IDIOMS: GuideSection =
    GuideSection::new("KEYBOARD NAVIGATION", &KEYBOARD_GESTURES);

/// Inspector-panel traversal hints for applications using `PanelNavigator`.
pub const PANEL_IDIOMS: GuideSection = GuideSection::new("INSPECTOR PANELS", &PANEL_GESTURES);

/// Focused and hovered adjustment hints for applications using Poolrooms rails.
pub const RAIL_IDIOMS: GuideSection = GuideSection::new("RAILS", &RAIL_GESTURES);

/// One target-relative interaction hint shown in a command guide.
///
/// Unlike a command shortcut, a gesture is interpreted only by its focused
/// target and is never globally routed.
#[derive(Clone, Copy, Debug)]
pub struct GuideGesture {
    label: &'static str,
    detail: &'static str,
    shortcuts: &'static [Shortcut],
}

impl GuideGesture {
    /// Declare one target-relative keyboard gesture.
    #[must_use]
    pub const fn new(
        label: &'static str,
        detail: &'static str,
        shortcuts: &'static [Shortcut],
    ) -> Self {
        Self {
            label,
            detail,
            shortcuts,
        }
    }

    /// Visible gesture label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        self.label
    }

    /// Consequence and ownership explanation.
    #[must_use]
    pub const fn detail(self) -> &'static str {
        self.detail
    }

    /// Keys interpreted by the focused target.
    #[must_use]
    pub const fn shortcuts(self) -> &'static [Shortcut] {
        self.shortcuts
    }
}

/// Named group of target-relative interactions.
#[derive(Clone, Copy, Debug)]
pub struct GuideSection {
    title: &'static str,
    gestures: &'static [GuideGesture],
}

impl GuideSection {
    /// Declare one help section.
    #[must_use]
    pub const fn new(title: &'static str, gestures: &'static [GuideGesture]) -> Self {
        Self { title, gestures }
    }

    /// Section heading.
    #[must_use]
    pub const fn title(self) -> &'static str {
        self.title
    }

    /// Stable gesture rows.
    #[must_use]
    pub const fn gestures(self) -> &'static [GuideGesture] {
        self.gestures
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum GuidePage {
    #[default]
    Context,
    All,
}

/// Stateful modal help surface for one application command canon.
#[derive(Debug, Default)]
pub struct CommandGuide {
    open: bool,
    rect: Option<egui::Rect>,
    page: GuidePage,
    restore_focus: Option<egui::Id>,
    pending_focus: Option<FocusReturn>,
    focus_close: bool,
}

#[derive(Debug)]
struct FocusReturn {
    target: Option<egui::Id>,
    closed_frame: u64,
}

impl CommandGuide {
    /// Whether the modal guide is currently open.
    #[must_use]
    pub const fn is_open(&self) -> bool {
        self.open
    }

    /// Geometry occupied by the guide card in its most recent open pass.
    ///
    /// Applications may publish this rectangle as a one-way acceptance
    /// target. It is absent while the guide is closed.
    #[must_use]
    pub const fn rect(&self) -> Option<egui::Rect> {
        self.rect
    }

    /// Open the guide and remember the current focus restoration target.
    pub fn open(&mut self, ctx: &egui::Context) {
        self.settle_focus(ctx);
        if self.open {
            return;
        }
        self.restore_focus = self.pending_focus.take().map_or_else(
            || ctx.memory(egui::Memory::focused),
            |pending| pending.target,
        );
        self.open = true;
        self.focus_close = true;
        ctx.request_repaint();
    }

    /// Close the guide and restore the control that opened it when possible.
    pub fn close(&mut self, ctx: &egui::Context) {
        if !self.open {
            return;
        }
        self.open = false;
        self.rect = None;
        self.focus_close = false;
        self.pending_focus = Some(FocusReturn {
            target: self.restore_focus.take(),
            closed_frame: ctx.cumulative_frame_nr(),
        });
        ctx.request_repaint();
    }

    /// Consume F1 or question mark and toggle the guide.
    ///
    /// Question mark defers to a focused text editor; F1 remains an
    /// application-level help key.
    pub fn take_shortcuts(&mut self, ctx: &egui::Context) -> bool {
        self.settle_focus(ctx);
        let question = if ctx.text_edit_focused() {
            Stroke::None
        } else {
            take(ctx, HELP_SHORTCUTS[0])
        };
        let function = take(ctx, HELP_SHORTCUTS[1]);
        let invoked = question == Stroke::Fresh || function == Stroke::Fresh;
        if invoked {
            if self.open {
                self.close(ctx);
            } else {
                self.open(ctx);
            }
        }
        invoked
    }

    /// Show the persistent small help plunger and toggle the guide when used.
    pub fn activator(&mut self, ui: &mut egui::Ui) -> MonoglyphResponse {
        self.settle_focus(ui.ctx());
        let response = Monoglyph::symbol(Symbol::Help)
            .size(MechanismSize::Small)
            .show(ui)
            .on_hover_text(format!(
                "Help · {} or {}",
                HELP_SHORTCUTS[0].label(ui.ctx()),
                HELP_SHORTCUTS[1].label(ui.ctx())
            ));
        if response.clicked() {
            if self.open {
                self.close(ui.ctx());
            } else {
                self.open(ui.ctx());
            }
        }
        response
    }

    /// Render the guide above the completed application UI.
    ///
    /// Active contexts are ordered most-specific first. Scope names must be
    /// stable prose such as MAP, LIBRARY, or APPLICATION.
    pub fn show<'reason, C, S>(
        &mut self,
        ctx: &egui::Context,
        canon: &CommandCanon<C, S>,
        contexts: &[S],
        scope_name: impl Fn(S) -> &'static str,
        status: impl Fn(C) -> CommandStatus<'reason>,
        extra_sections: &[GuideSection],
    ) where
        C: Copy + Debug + Eq + 'static,
        S: Copy + Debug + Eq + 'static,
    {
        self.settle_focus(ctx);
        if !self.open {
            self.rect = None;
            return;
        }
        let width = (ctx.content_rect().width() - 48.0).clamp(340.0, 760.0);
        let body_height = (ctx.content_rect().height() - 230.0).clamp(220.0, 560.0);
        let mut close = false;
        let focus_close = self.focus_close;
        let page = &mut self.page;
        let modal = egui::Modal::new(egui::Id::new("eternalist-command-guide"))
            .frame(
                egui::Frame::new()
                    .fill(chrome::SURFACE)
                    .stroke(egui::Stroke::new(1.5_f32, chrome::EDGE_STRONG))
                    .corner_radius(2)
                    .inner_margin(egui::Margin::same(14)),
            )
            .backdrop_color(egui::Color32::from_black_alpha(176))
            .show(ctx, |ui| {
                ui.set_width(width);
                let _header = ui.horizontal(|ui| {
                    let _title = ui.label(chrome::title("HELP & COMMANDS"));
                    let _close =
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let response = Monoglyph::symbol(Symbol::Remove)
                                .size(MechanismSize::Small)
                                .show(ui)
                                .on_hover_text("Close help · Escape");
                            if focus_close {
                                response.request_focus();
                            }
                            close |= response.clicked();
                        });
                });
                let _hint = ui.label(chrome::muted(format!(
                    "{} or {} toggles this guide",
                    HELP_SHORTCUTS[0].label(ui.ctx()),
                    HELP_SHORTCUTS[1].label(ui.ctx())
                )));
                ui.add_space(10.0);
                let _pages = ui.horizontal(|ui| {
                    if page_button(ui, "CURRENT CONTEXT", *page == GuidePage::Context) {
                        *page = GuidePage::Context;
                    }
                    if page_button(ui, "ALL COMMANDS", *page == GuidePage::All) {
                        *page = GuidePage::All;
                    }
                });
                ui.add_space(8.0);
                let _body = egui::ScrollArea::vertical()
                    .id_salt("eternalist-command-guide-body")
                    .min_scrolled_height(body_height)
                    .max_height(body_height)
                    .auto_shrink([false, false])
                    .show(ui, |ui| match *page {
                        GuidePage::Context => {
                            show_context(ui, canon, contexts, &scope_name, &status, extra_sections);
                        }
                        GuidePage::All => {
                            show_all(ui, canon, &scope_name, &status, extra_sections);
                        }
                    });
                ui.min_rect()
            });
        self.rect = Some(modal.inner);
        self.focus_close = false;
        if close || modal.should_close() {
            self.close(ctx);
        }
    }

    fn settle_focus(&mut self, ctx: &egui::Context) {
        if self.pending_focus.is_some() && ctx.input(focus_return_interdicted) {
            self.pending_focus = None;
            return;
        }
        // egui admits interaction against the preceding pass's modal layer.
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

fn focus_return_interdicted(input: &egui::InputState) -> bool {
    input.pointer.any_pressed()
        || input.events.iter().any(|event| {
            matches!(
                event,
                egui::Event::Copy
                    | egui::Event::Cut
                    | egui::Event::Paste(_)
                    | egui::Event::Text(_)
                    | egui::Event::Key { pressed: true, .. }
                    | egui::Event::AccessKitActionRequest(_)
            )
        })
}

fn page_button(ui: &mut egui::Ui, label: &'static str, selected: bool) -> bool {
    let button = egui::Button::new(chrome::section_title(label))
        .fill(if selected {
            chrome::RAISED
        } else {
            chrome::CONTROL
        })
        .stroke(egui::Stroke::new(
            if selected { 1.5_f32 } else { 1.0_f32 },
            if selected { chrome::HOT } else { chrome::EDGE },
        ));
    let response = ui.add(button);
    chrome::shallow_tension(ui, &response);
    chrome::exact_activation(ui, &response)
}

fn show_context<'reason, C, S>(
    ui: &mut egui::Ui,
    canon: &CommandCanon<C, S>,
    contexts: &[S],
    scope_name: &impl Fn(S) -> &'static str,
    status: &impl Fn(C) -> CommandStatus<'reason>,
    extra_sections: &[GuideSection],
) where
    C: Copy + Debug + Eq + 'static,
    S: Copy + Debug + Eq + 'static,
{
    for context in contexts {
        show_command_group(
            ui,
            scope_name(*context),
            canon,
            canon
                .specs()
                .iter()
                .filter(|spec| spec.scope() == CommandScope::Context(*context)),
            status,
        );
    }
    show_command_group(
        ui,
        "APPLICATION",
        canon,
        canon
            .specs()
            .iter()
            .filter(|spec| spec.scope() == CommandScope::Global),
        status,
    );
    show_guide_sections(ui, extra_sections);
}

fn show_all<'reason, C, S>(
    ui: &mut egui::Ui,
    canon: &CommandCanon<C, S>,
    scope_name: &impl Fn(S) -> &'static str,
    status: &impl Fn(C) -> CommandStatus<'reason>,
    extra_sections: &[GuideSection],
) where
    C: Copy + Debug + Eq + 'static,
    S: Copy + Debug + Eq + 'static,
{
    show_command_group(
        ui,
        "APPLICATION",
        canon,
        canon
            .specs()
            .iter()
            .filter(|spec| spec.scope() == CommandScope::Global),
        status,
    );
    let mut scopes = Vec::new();
    for spec in canon.specs() {
        let CommandScope::Context(scope) = spec.scope() else {
            continue;
        };
        if !scopes.contains(&scope) {
            scopes.push(scope);
        }
    }
    for scope in scopes {
        show_command_group(
            ui,
            scope_name(scope),
            canon,
            canon
                .specs()
                .iter()
                .filter(|spec| spec.scope() == CommandScope::Context(scope)),
            status,
        );
    }
    show_guide_sections(ui, extra_sections);
}

fn show_command_group<'reason, 'spec, C, S>(
    ui: &mut egui::Ui,
    title: &'static str,
    canon: &CommandCanon<C, S>,
    specs: impl Iterator<Item = &'spec CommandSpec<C, S>>,
    status: &impl Fn(C) -> CommandStatus<'reason>,
) where
    C: Copy + Debug + Eq + 'static,
    S: Copy + Debug + Eq + 'static,
{
    let rows = specs
        .filter_map(|spec| {
            let state = status(spec.command());
            (state != CommandStatus::Hidden).then_some((spec, state))
        })
        .collect::<Vec<_>>();
    if rows.is_empty() {
        return;
    }
    let _title = ui.label(chrome::eyebrow(title));
    ui.add_space(4.0);
    let columns = GuideColumns::for_width(ui.available_width());
    let _grid = egui::Grid::new(("eternalist-command-guide-commands", title))
        .num_columns(3)
        .min_col_width(0.0)
        .min_row_height(0.0)
        .spacing(egui::vec2(GUIDE_COLUMN_SPACING, GUIDE_ROW_SPACING))
        .striped(false)
        .show(ui, |ui| {
            for (spec, state) in rows {
                let enabled = state == CommandStatus::Enabled;
                let bindings = canon
                    .shortcuts(spec.command())
                    .iter()
                    .copied()
                    .chain(spec.mnemonic_key().map(Shortcut::mnemonic));
                let name =
                    spec.widget_text_with_font(ui, &egui::FontId::proportional(GUIDE_NAME_SIZE));
                let detail = command_detail(spec.detail(), state);
                show_guide_row(ui, columns, enabled, bindings, name, detail);
            }
        });
    ui.add_space(GUIDE_SECTION_SPACING);
}

fn show_guide_sections(ui: &mut egui::Ui, extra_sections: &[GuideSection]) {
    show_gesture_group(ui, KEYBOARD_IDIOMS);
    for section in extra_sections {
        show_gesture_group(ui, *section);
    }
    show_gesture_group(ui, GUIDE_SECTION);
}

fn show_gesture_group(ui: &mut egui::Ui, section: GuideSection) {
    let _title = ui.label(chrome::eyebrow(section.title()));
    ui.add_space(4.0);
    let columns = GuideColumns::for_width(ui.available_width());
    let _grid = egui::Grid::new(("eternalist-command-guide-gestures", section.title()))
        .num_columns(3)
        .min_col_width(0.0)
        .min_row_height(0.0)
        .spacing(egui::vec2(GUIDE_COLUMN_SPACING, GUIDE_ROW_SPACING))
        .striped(false)
        .show(ui, |ui| {
            for gesture in section.gestures() {
                show_guide_row(
                    ui,
                    columns,
                    true,
                    gesture.shortcuts().iter().copied(),
                    egui::RichText::new(gesture.label())
                        .size(GUIDE_NAME_SIZE)
                        .into(),
                    guide_detail(gesture.detail()).into(),
                );
            }
        });
    ui.add_space(GUIDE_SECTION_SPACING);
}

fn show_guide_row(
    ui: &mut egui::Ui,
    columns: GuideColumns,
    enabled: bool,
    bindings: impl IntoIterator<Item = Shortcut>,
    name: egui::WidgetText,
    detail: egui::WidgetText,
) {
    let _bindings = ui.add_enabled_ui(enabled, |ui| {
        ui.set_width(columns.bindings);
        let _keys = ui.horizontal_wrapped(|ui| {
            for shortcut in bindings {
                let _cap = Keycap::new(shortcut.label(ui.ctx())).show(ui);
            }
        });
    });
    let _name = ui.add_enabled_ui(enabled, |ui| {
        ui.set_width(columns.action);
        let _label = ui.add(egui::Label::new(name).wrap());
    });
    let _detail = ui.add_enabled_ui(enabled, |ui| {
        ui.set_width(columns.consequence);
        let _label = ui.add(egui::Label::new(detail).wrap());
    });
    ui.end_row();
}

fn command_detail(detail: &str, state: CommandStatus<'_>) -> egui::WidgetText {
    let text = match state {
        CommandStatus::Disabled(reason) if detail.is_empty() => format!("Unavailable: {reason}"),
        CommandStatus::Disabled(reason) => format!("{detail} · Unavailable: {reason}"),
        CommandStatus::Enabled | CommandStatus::Hidden => detail.to_owned(),
    };
    guide_detail(text).into()
}

fn guide_detail(text: impl Into<String>) -> egui::RichText {
    egui::RichText::new(text)
        .size(GUIDE_DETAIL_SIZE)
        .color(chrome::MUTED)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn question_mark() -> egui::RawInput {
        egui::RawInput {
            modifiers: egui::Modifiers::SHIFT,
            events: vec![egui::Event::Key {
                key: egui::Key::Questionmark,
                physical_key: Some(egui::Key::Slash),
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::SHIFT,
            }],
            ..egui::RawInput::default()
        }
    }

    #[test]
    fn question_mark_defers_to_text_entry() {
        let ctx = egui::Context::default();
        let mut guide = CommandGuide::default();
        let mut text = String::new();
        let _prime = ctx.run_ui(egui::RawInput::default(), |ui| {
            ui.text_edit_singleline(&mut text).request_focus();
        });
        let _output = ctx.run_ui(question_mark(), |ui| {
            assert!(!guide.take_shortcuts(ui.ctx()));
            let _editor = ui.text_edit_singleline(&mut text);
        });
        assert!(!guide.is_open());
        assert!(ctx.input(|state| state.key_pressed(egui::Key::Questionmark)));
    }

    #[test]
    fn delayed_focus_return_yields_only_to_fresh_navigation() {
        fn close(navigate: bool) -> (bool, bool) {
            let ctx = egui::Context::default();
            let mut guide = CommandGuide::default();
            let modal = || egui::Modal::new(egui::Id::new("focus-return-modal"));

            let _open = ctx.run_ui(egui::RawInput::default(), |ui| {
                ui.button("target").request_focus();
                let _other = ui.button("other");
                guide.open(ui.ctx());
                let _modal = modal().show(ui.ctx(), |ui| ui.button("close"));
            });
            let _close = ctx.run_ui(egui::RawInput::default(), |ui| {
                let _target = ui.button("target");
                let _other = ui.button("other");
                let _modal = modal().show(ui.ctx(), |ui| ui.button("close"));
                guide.close(ui.ctx());
            });
            let input = if navigate {
                egui::RawInput {
                    modifiers: egui::Modifiers::CTRL,
                    events: vec![egui::Event::Key {
                        key: egui::Key::Tab,
                        physical_key: Some(egui::Key::Tab),
                        pressed: true,
                        repeat: false,
                        modifiers: egui::Modifiers::CTRL,
                    }],
                    ..egui::RawInput::default()
                }
            } else {
                egui::RawInput::default()
            };
            let _retire = ctx.run_ui(input, |ui| {
                guide.settle_focus(ui.ctx());
                let _target = ui.button("target");
                let other = ui.button("other");
                if navigate {
                    other.request_focus();
                }
            });
            let mut focused = (false, false);
            let _handoff = ctx.run_ui(egui::RawInput::default(), |ui| {
                guide.settle_focus(ui.ctx());
                focused.0 = ui.button("target").has_focus();
                focused.1 = ui.button("other").has_focus();
            });
            focused
        }

        // Modal retirement spans egui frames. A delayed restoration must
        // survive that handoff, yet never overwrite a newer user navigation.
        assert_eq!(close(false), (true, false));
        assert_eq!(close(true), (false, true));
    }
}
