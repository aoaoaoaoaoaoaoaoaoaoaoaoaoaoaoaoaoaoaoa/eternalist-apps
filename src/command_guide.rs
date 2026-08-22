//! Modal command guidance rendered from the same canon that routes input.

#![deny(missing_docs)]

use std::fmt::Debug;

use brass_poolrooms::chrome::{
    self, Keycap, MechanismSize, Monoglyph, MonoglyphResponse, ScrewScroll, Symbol,
};

use crate::commands::{
    ACTIVATE, CommandCanon, CommandScope, CommandSpec, CommandStatus, HELP_SHORTCUTS, NEXT_CONTROL,
    PREVIOUS_CONTROL, Shortcut, Stroke, UNWIND, take,
};
use crate::modal::{ModalShell, card_frame, scroll_aperture};

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
const GUIDE_GESTURES: [GuideGesture; 1] = [GuideGesture::new(
    "Toggle this guide",
    "Opens from application chrome or the keyboard.",
    &HELP_SHORTCUTS,
)];
const GUIDE_SECTION: GuideSection = GuideSection::new("GUIDE", &GUIDE_GESTURES);

/// Baseline keyboard grammar rendered by every command guide.
const KEYBOARD_IDIOMS: GuideSection = GuideSection::new("KEYBOARD NAVIGATION", &KEYBOARD_GESTURES);

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

/// Application-owned group of target-relative interactions.
///
/// Eternalist deliberately exports no ready-made target sections: the
/// application names each target in product vocabulary and decides exactly
/// which contexts admit it.
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
    shell: ModalShell,
    page: GuidePage,
}

impl CommandGuide {
    /// Whether the modal guide is currently open.
    #[must_use]
    pub const fn is_open(&self) -> bool {
        self.shell.is_open()
    }

    /// Geometry occupied by the guide card in its most recent open pass.
    ///
    /// Applications may publish this rectangle as a one-way acceptance
    /// target. It is absent while the guide is closed.
    #[must_use]
    pub const fn rect(&self) -> Option<egui::Rect> {
        self.shell.rect()
    }

    /// Open the guide and remember the current focus restoration target.
    pub fn open(&mut self, ctx: &egui::Context) {
        self.shell.open(ctx);
    }

    /// Close the guide and restore the control that opened it when possible.
    pub fn close(&mut self, ctx: &egui::Context) {
        self.shell.close(ctx);
    }

    /// Consume F1 or question mark and toggle the guide.
    ///
    /// Question mark defers to a focused text editor; F1 remains an
    /// application-level help key. Call this before rendering application UI:
    /// while the guide is open it quarantines wheel input from underlying
    /// controls and returns that input only to [`Self::show`].
    pub fn take_shortcuts(&mut self, ctx: &egui::Context) -> bool {
        self.shell.prepare(ctx);
        let question = if ctx.text_edit_focused() {
            Stroke::None
        } else {
            take(ctx, HELP_SHORTCUTS[0])
        };
        let function = take(ctx, HELP_SHORTCUTS[1]);
        let invoked = question == Stroke::Fresh || function == Stroke::Fresh;
        if invoked {
            self.shell.toggle(ctx);
        }
        self.shell.quarantine_wheel(ctx);
        invoked
    }

    /// Show the persistent small help plunger and toggle the guide when used.
    pub fn activator(&mut self, ui: &mut egui::Ui) -> MonoglyphResponse {
        self.shell.prepare(ui.ctx());
        let response = Monoglyph::symbol(Symbol::Help)
            .size(MechanismSize::Medium)
            .show(ui)
            .on_hover_text(format!(
                "Help · {} or {}",
                HELP_SHORTCUTS[0].label(ui.ctx()),
                HELP_SHORTCUTS[1].label(ui.ctx())
            ));
        if response.clicked() {
            self.shell.toggle(ui.ctx());
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
        application_sections: &[GuideSection],
    ) where
        C: Copy + Debug + Eq + 'static,
        S: Copy + Debug + Eq + 'static,
    {
        if !self.shell.begin_present(ctx) {
            return;
        }
        let width = (ctx.content_rect().width() - 48.0).clamp(340.0, 760.0);
        let mut close = false;
        let page = &mut self.page;
        let modal = egui::Modal::new(egui::Id::new("eternalist-command-guide"))
            .frame(card_frame())
            .backdrop_color(egui::Color32::from_black_alpha(176))
            .show(ctx, |ui| {
                ui.set_width(width);
                let chrome_top = ui.cursor().top();
                let _header = ui.horizontal(|ui| {
                    let _title = ui.label(chrome::title("HELP & COMMANDS"));
                    let _close =
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let response = Monoglyph::symbol(Symbol::Remove)
                                .size(MechanismSize::Small)
                                .focusable(false)
                                .show(ui)
                                .on_hover_text("Close help · Escape");
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
                let aperture = scroll_aperture(ctx, ui.cursor().top() - chrome_top, 560.0, 560.0);
                let body = ScrewScroll::vertical()
                    .id_salt("eternalist-command-guide-body")
                    .min_scrolled_height(aperture.height)
                    .max_height(aperture.height)
                    .auto_shrink([false, false])
                    .show(ui, |ui| match *page {
                        GuidePage::Context => {
                            show_context(
                                ui,
                                canon,
                                contexts,
                                &scope_name,
                                &status,
                                application_sections,
                            );
                        }
                        GuidePage::All => {
                            show_all(ui, canon, &scope_name, &status, application_sections);
                        }
                    });
                record_rect(ui.ctx(), "eternalist.command-guide.body", body.inner_rect);
            });
        // Modal retirement follows the presented surface by one pass, so the
        // public state and witness geometry cannot diverge.
        self.shell
            .finish_present(ctx, modal.response.rect, close || modal.should_close());
    }
}

#[inline]
fn record_rect(ctx: &egui::Context, name: &'static str, rect: egui::Rect) {
    #[cfg(all(
        feature = "egui-test",
        any(target_os = "linux", target_os = "macos", target_os = "windows")
    ))]
    egui_tester_witness::egui::record_rect(ctx, name, rect);
    #[cfg(not(all(
        feature = "egui-test",
        any(target_os = "linux", target_os = "macos", target_os = "windows")
    )))]
    let _ = (ctx, name, rect);
}

fn page_button(ui: &mut egui::Ui, label: &'static str, selected: bool) -> bool {
    let button = egui::Button::new(chrome::section_title(label))
        .selected(selected)
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
    if response.has_focus() {
        ui.painter().rect_stroke(
            response.rect.expand(1.0),
            2,
            egui::Stroke::new(1.0_f32, chrome::HOT),
            egui::StrokeKind::Outside,
        );
    }
    chrome::exact_activation(ui, &response)
}

fn show_context<'reason, C, S>(
    ui: &mut egui::Ui,
    canon: &CommandCanon<C, S>,
    contexts: &[S],
    scope_name: &impl Fn(S) -> &'static str,
    status: &impl Fn(C) -> CommandStatus<'reason>,
    application_sections: &[GuideSection],
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
    show_guide_sections(ui, application_sections);
}

fn show_all<'reason, C, S>(
    ui: &mut egui::Ui,
    canon: &CommandCanon<C, S>,
    scope_name: &impl Fn(S) -> &'static str,
    status: &impl Fn(C) -> CommandStatus<'reason>,
    application_sections: &[GuideSection],
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
    show_guide_sections(ui, application_sections);
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

fn show_guide_sections(ui: &mut egui::Ui, application_sections: &[GuideSection]) {
    show_gesture_group(ui, KEYBOARD_IDIOMS);
    for section in application_sections {
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

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum Never {}

    const NO_COMMANDS: [CommandSpec<Never, ()>; 0] = [];

    fn question_mark() -> egui::RawInput {
        egui::RawInput {
            events: vec![
                egui::Event::ModifiersChanged(egui::Modifiers::SHIFT),
                egui::Event::Key {
                    key: egui::Key::Questionmark,
                    physical_key: Some(egui::Key::Slash),
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::SHIFT,
                },
            ],
            ..egui::RawInput::default()
        }
    }

    #[test]
    fn question_mark_defers_to_text_entry() {
        let ctx = egui::Context::default();
        let mut guide = CommandGuide::default();
        let mut text = String::new();
        ctx.run_ui(egui::RawInput::default(), |ui| {
            ui.text_edit_singleline(&mut text).request_focus();
        })
        .drop_without_applying_deltas();
        ctx.run_ui(question_mark(), |ui| {
            assert!(!guide.take_shortcuts(ui.ctx()));
            let _editor = ui.text_edit_singleline(&mut text);
        })
        .drop_without_applying_deltas();
        assert!(!guide.is_open());
        assert!(ctx.input(|state| state.key_pressed(egui::Key::Questionmark)));
    }

    #[test]
    fn open_guide_owns_wheel_across_its_late_render() {
        fn wheel_present(ctx: &egui::Context) -> bool {
            ctx.input(|input| {
                input.smooth_scroll_delta != egui::Vec2::ZERO
                    || input
                        .events
                        .iter()
                        .any(|event| matches!(event, egui::Event::MouseWheel { .. }))
            })
        }

        let ctx = egui::Context::default();
        let canon = CommandCanon::new(&NO_COMMANDS);
        let mut guide = CommandGuide::default();
        let show = |guide: &mut CommandGuide, ctx: &egui::Context| {
            guide.show(
                ctx,
                &canon,
                &[()],
                |()| "APPLICATION",
                |command| match command {},
                &[],
            );
        };
        ctx.run_ui(egui::RawInput::default(), |ui| {
            guide.open(ui.ctx());
            show(&mut guide, ui.ctx());
        })
        .drop_without_applying_deltas();

        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(900.0, 700.0),
            )),
            events: vec![
                egui::Event::PointerMoved(egui::pos2(450.0, 350.0)),
                egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Line,
                    delta: egui::vec2(0.0, -1.0),
                    phase: egui::TouchPhase::Move,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            ..egui::RawInput::default()
        };
        let mut arrived = false;
        let mut reached_underlay = false;
        let mut escaped_modal = false;
        ctx.run_ui(input, |ui| {
            arrived = wheel_present(ui.ctx());
            let _invoked = guide.take_shortcuts(ui.ctx());
            reached_underlay = wheel_present(ui.ctx());
            show(&mut guide, ui.ctx());
            escaped_modal = wheel_present(ui.ctx());
        })
        .drop_without_applying_deltas();

        assert!(arrived);
        assert!(!reached_underlay);
        assert!(!escaped_modal);
        assert!(guide.is_open());
    }

    #[test]
    fn delayed_focus_return_yields_only_to_fresh_navigation() {
        fn close(navigate: bool) -> (bool, bool) {
            let ctx = egui::Context::default();
            let mut guide = CommandGuide::default();
            let modal = || egui::Modal::new(egui::Id::new("focus-return-modal"));

            ctx.run_ui(egui::RawInput::default(), |ui| {
                ui.button("target").request_focus();
                let _other = ui.button("other");
                guide.open(ui.ctx());
                let _modal = modal().show(ui.ctx(), |ui| ui.button("close"));
            })
            .drop_without_applying_deltas();
            ctx.run_ui(egui::RawInput::default(), |ui| {
                let _target = ui.button("target");
                let _other = ui.button("other");
                let _modal = modal().show(ui.ctx(), |ui| ui.button("close"));
                guide.close(ui.ctx());
            })
            .drop_without_applying_deltas();
            let input = if navigate {
                egui::RawInput {
                    events: vec![
                        egui::Event::ModifiersChanged(egui::Modifiers::CTRL),
                        egui::Event::Key {
                            key: egui::Key::Tab,
                            physical_key: Some(egui::Key::Tab),
                            pressed: true,
                            repeat: false,
                            modifiers: egui::Modifiers::CTRL,
                        },
                    ],
                    ..egui::RawInput::default()
                }
            } else {
                egui::RawInput::default()
            };
            ctx.run_ui(input, |ui| {
                guide.shell.prepare(ui.ctx());
                let _target = ui.button("target");
                let other = ui.button("other");
                if navigate {
                    other.request_focus();
                }
            })
            .drop_without_applying_deltas();
            let mut focused = (false, false);
            ctx.run_ui(egui::RawInput::default(), |ui| {
                guide.shell.prepare(ui.ctx());
                focused.0 = ui.button("target").has_focus();
                focused.1 = ui.button("other").has_focus();
            })
            .drop_without_applying_deltas();
            focused
        }

        // Modal retirement spans egui frames. A delayed restoration must
        // survive that handoff, yet never overwrite a newer user navigation.
        assert_eq!(close(false), (true, false));
        assert_eq!(close(true), (false, true));
    }
}
