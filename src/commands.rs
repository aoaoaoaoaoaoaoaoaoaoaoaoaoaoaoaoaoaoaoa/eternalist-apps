//! Semantic commands, portable accelerators, and exact input dispatch.
//!
//! A command is an application-owned typed value. This module supplies the
//! stable metadata and routing law around it; it never performs domain work.

#![deny(missing_docs)]

use std::{fmt::Debug, ops::Deref};

pub(crate) const HELP_SHORTCUTS: [Shortcut; 2] = [
    Shortcut::new(ShortcutModifiers::NONE, ShortcutKey::QuestionMark),
    Shortcut::new(ShortcutModifiers::NONE, ShortcutKey::Function(1)),
];
pub(crate) const NEXT_CONTROL: [Shortcut; 1] =
    [Shortcut::new(ShortcutModifiers::NONE, ShortcutKey::Tab)];
pub(crate) const PREVIOUS_CONTROL: [Shortcut; 1] =
    [Shortcut::new(ShortcutModifiers::SHIFT, ShortcutKey::Tab)];
pub(crate) const NEXT_PANEL: [Shortcut; 1] =
    [Shortcut::new(ShortcutModifiers::CONTROL, ShortcutKey::Tab)];
pub(crate) const PREVIOUS_PANEL: [Shortcut; 1] = [Shortcut::new(
    ShortcutModifiers::CONTROL.plus(ShortcutModifiers::SHIFT),
    ShortcutKey::Tab,
)];
pub(crate) const TOGGLE_INSPECTOR: [Shortcut; 1] = [Shortcut::new(
    ShortcutModifiers::NONE,
    ShortcutKey::Function(9),
)];
pub(crate) const ACTIVATE: [Shortcut; 2] = [
    Shortcut::new(ShortcutModifiers::NONE, ShortcutKey::Enter),
    Shortcut::new(ShortcutModifiers::NONE, ShortcutKey::Space),
];
pub(crate) const ADJUST: [Shortcut; 2] = [
    Shortcut::new(ShortcutModifiers::NONE, ShortcutKey::ArrowLeft),
    Shortcut::new(ShortcutModifiers::NONE, ShortcutKey::ArrowRight),
];
const VERTICAL_ADJUST: [Shortcut; 2] = [
    Shortcut::new(ShortcutModifiers::NONE, ShortcutKey::ArrowUp),
    Shortcut::new(ShortcutModifiers::NONE, ShortcutKey::ArrowDown),
];
pub(crate) const BOUNDS: [Shortcut; 2] = [
    Shortcut::new(ShortcutModifiers::NONE, ShortcutKey::Home),
    Shortcut::new(ShortcutModifiers::NONE, ShortcutKey::End),
];
pub(crate) const UNWIND: [Shortcut; 1] =
    [Shortcut::new(ShortcutModifiers::NONE, ShortcutKey::Escape)];
const RESERVED: [Shortcut; 16] = [
    HELP_SHORTCUTS[0],
    HELP_SHORTCUTS[1],
    NEXT_CONTROL[0],
    PREVIOUS_CONTROL[0],
    NEXT_PANEL[0],
    PREVIOUS_PANEL[0],
    TOGGLE_INSPECTOR[0],
    ACTIVATE[0],
    ACTIVATE[1],
    ADJUST[0],
    ADJUST[1],
    VERTICAL_ADJUST[0],
    VERTICAL_ADJUST[1],
    BOUNDS[0],
    BOUNDS[1],
    UNWIND[0],
];

/// Platform-neutral modifier set used by a [Shortcut].
///
/// Primary means Command on macOS and Control elsewhere. Control always means
/// the physical Control key, which is why panel traversal can lawfully use
/// Control+Tab without colliding with the macOS application switcher.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the four flags are the orthogonal powerset of keyboard modifiers and serialize legibly"
)]
pub struct ShortcutModifiers {
    primary: bool,
    control: bool,
    alt: bool,
    shift: bool,
}

impl ShortcutModifiers {
    /// No modifiers.
    pub const NONE: Self = Self {
        primary: false,
        control: false,
        alt: false,
        shift: false,
    };
    /// Command on macOS and Control elsewhere.
    pub const PRIMARY: Self = Self {
        primary: true,
        ..Self::NONE
    };
    /// Physical Control on every platform.
    pub const CONTROL: Self = Self {
        control: true,
        ..Self::NONE
    };
    /// Alt, called Option on macOS.
    pub const ALT: Self = Self {
        alt: true,
        ..Self::NONE
    };
    /// Shift.
    pub const SHIFT: Self = Self {
        shift: true,
        ..Self::NONE
    };

    /// Union two modifier sets.
    #[must_use]
    pub const fn plus(self, other: Self) -> Self {
        Self {
            primary: self.primary || other.primary,
            control: self.control || other.control,
            alt: self.alt || other.alt,
            shift: self.shift || other.shift,
        }
    }

    /// Whether this set names the cross-platform primary modifier.
    #[must_use]
    pub const fn primary(self) -> bool {
        self.primary
    }

    /// Whether this set names physical Control.
    #[must_use]
    pub const fn control(self) -> bool {
        self.control
    }

    /// Whether this set names Alt or Option.
    #[must_use]
    pub const fn alt(self) -> bool {
        self.alt
    }

    /// Whether this set names Shift.
    #[must_use]
    pub const fn shift(self) -> bool {
        self.shift
    }

    fn egui(self) -> egui::Modifiers {
        let mut modifiers = egui::Modifiers::NONE;
        if self.primary {
            modifiers = modifiers.plus(egui::Modifiers::COMMAND);
        }
        if self.control {
            modifiers = modifiers.plus(egui::Modifiers::CTRL);
        }
        if self.alt {
            modifiers = modifiers.plus(egui::Modifiers::ALT);
        }
        if self.shift {
            modifiers = modifiers.plus(egui::Modifiers::SHIFT);
        }
        modifiers
    }

    fn valid(self) -> bool {
        !(self.primary && self.control)
    }

    fn portable_projection(self, mac: bool) -> u8 {
        let mut projection = 0;
        if self.primary {
            projection |= if mac { 1 << 0 } else { 1 << 1 };
        }
        if self.control {
            projection |= 1 << 1;
        }
        if self.alt {
            projection |= 1 << 2;
        }
        if self.shift {
            projection |= 1 << 3;
        }
        projection
    }
}

/// Logical key admitted by shared command metadata.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum ShortcutKey {
    /// One ASCII letter or digit.
    Character(char),
    /// Function key F1 through F35.
    Function(u8),
    /// Escape.
    Escape,
    /// Tab.
    Tab,
    /// Backspace.
    Backspace,
    /// Enter or Return.
    Enter,
    /// Space.
    Space,
    /// Insert.
    Insert,
    /// Delete.
    Delete,
    /// Home.
    Home,
    /// End.
    End,
    /// Page Up.
    PageUp,
    /// Page Down.
    PageDown,
    /// Left arrow.
    ArrowLeft,
    /// Right arrow.
    ArrowRight,
    /// Up arrow.
    ArrowUp,
    /// Down arrow.
    ArrowDown,
    /// Slash.
    Slash,
    /// Question mark.
    QuestionMark,
}

impl ShortcutKey {
    fn egui(self) -> Option<egui::Key> {
        match self {
            Self::Character(character) => {
                let mut encoded = [0; 4];
                egui::Key::from_name(character.encode_utf8(&mut encoded))
            }
            Self::Function(number) => egui::Key::ALL
                .iter()
                .copied()
                .find(|key| key.name() == format!("F{number}")),
            Self::Escape => Some(egui::Key::Escape),
            Self::Tab => Some(egui::Key::Tab),
            Self::Backspace => Some(egui::Key::Backspace),
            Self::Enter => Some(egui::Key::Enter),
            Self::Space => Some(egui::Key::Space),
            Self::Insert => Some(egui::Key::Insert),
            Self::Delete => Some(egui::Key::Delete),
            Self::Home => Some(egui::Key::Home),
            Self::End => Some(egui::Key::End),
            Self::PageUp => Some(egui::Key::PageUp),
            Self::PageDown => Some(egui::Key::PageDown),
            Self::ArrowLeft => Some(egui::Key::ArrowLeft),
            Self::ArrowRight => Some(egui::Key::ArrowRight),
            Self::ArrowUp => Some(egui::Key::ArrowUp),
            Self::ArrowDown => Some(egui::Key::ArrowDown),
            Self::Slash => Some(egui::Key::Slash),
            Self::QuestionMark => Some(egui::Key::Questionmark),
        }
    }

    fn valid(self) -> bool {
        match self {
            Self::Character(character) => character.is_ascii_alphanumeric(),
            Self::Function(number) => (1..=35).contains(&number),
            _ => true,
        }
    }

    fn matches_text(self, text: &str) -> bool {
        let mut characters = text.chars();
        let Some(character) = characters.next() else {
            return false;
        };
        if characters.next().is_some() {
            return false;
        }
        match self {
            Self::Character(expected) => character.eq_ignore_ascii_case(&expected),
            Self::Slash => character == '/',
            Self::QuestionMark => character == '?',
            _ => false,
        }
    }
}

/// One platform-neutral keyboard accelerator.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Shortcut {
    modifiers: ShortcutModifiers,
    key: ShortcutKey,
}

impl Shortcut {
    /// Construct an accelerator from its portable modifiers and logical key.
    #[must_use]
    pub const fn new(modifiers: ShortcutModifiers, key: ShortcutKey) -> Self {
        Self { modifiers, key }
    }

    /// Construct a primary-modifier letter or digit accelerator.
    #[must_use]
    pub const fn primary(character: char) -> Self {
        Self::new(
            ShortcutModifiers::PRIMARY,
            ShortcutKey::Character(character),
        )
    }

    /// Construct a mnemonic Alt+letter or Alt+digit accelerator.
    #[must_use]
    pub const fn mnemonic(character: char) -> Self {
        Self::new(ShortcutModifiers::ALT, ShortcutKey::Character(character))
    }

    /// Modifier set.
    #[must_use]
    pub const fn modifiers(self) -> ShortcutModifiers {
        self.modifiers
    }

    /// Logical key.
    #[must_use]
    pub const fn key(self) -> ShortcutKey {
        self.key
    }

    /// Platform-native chord label according to the current egui viewport.
    #[must_use]
    pub fn label(self, ctx: &egui::Context) -> String {
        let key = self
            .key
            .egui()
            .unwrap_or_else(|| panic!("invalid shortcut key {:?}", self.key));
        let formatted =
            ctx.format_shortcut(&egui::KeyboardShortcut::new(self.modifiers.egui(), key));
        let name = key.name();
        let symbol = key.symbol_or_name();
        if let Some(prefix) = formatted.strip_suffix(name) {
            format!("{prefix}{symbol}")
        } else {
            formatted
        }
    }

    fn valid(self) -> bool {
        self.modifiers.valid() && self.key.valid()
    }

    fn collides(self, other: Self) -> bool {
        if self.key.egui() != other.key.egui() {
            return false;
        }
        [false, true].into_iter().any(|mac| {
            let left = self.modifiers.portable_projection(mac);
            let right = other.modifiers.portable_projection(mac);
            left == right
                || (self.key == ShortcutKey::QuestionMark && left & !(1 << 3) == right & !(1 << 3))
        })
    }

    fn matches(self, key: egui::Key, mut held: egui::Modifiers) -> bool {
        if self.key.egui() != Some(key) {
            return false;
        }
        if self.key == ShortcutKey::QuestionMark && !self.modifiers.shift {
            held.shift = false;
        }
        held.matches_exact(self.modifiers.egui())
    }
}

/// Application context in which a command exists.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CommandScope<S> {
    /// Command is visible in every application context.
    Global,
    /// Command belongs to one application-defined context.
    Context(S),
}

/// Whether a command may supersede a focused text editor.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TextFocusPolicy {
    /// Leave the keystroke to the text editor.
    #[default]
    Defer,
    /// The application command lawfully owns the keystroke even while editing.
    Capture,
}

/// Whether held-key repeats may invoke a command repeatedly.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum RepeatPolicy {
    /// Consume repeats without invoking the command again.
    #[default]
    Reject,
    /// Invoke once for each input frame containing one or more repeats.
    Allow,
}

/// Immutable declaration of one application command.
#[derive(Clone, Copy, Debug)]
pub struct CommandSpec<C, S: 'static> {
    command: C,
    id: &'static str,
    label: &'static str,
    detail: &'static str,
    scope: CommandScope<S>,
    default_shortcuts: &'static [Shortcut],
    mnemonic: Option<char>,
    text_focus: TextFocusPolicy,
    repeat: RepeatPolicy,
}

impl<C, S> CommandSpec<C, S> {
    /// Declare one typed command and its stable configuration identity.
    pub const fn new(
        command: C,
        id: &'static str,
        label: &'static str,
        scope: CommandScope<S>,
    ) -> Self {
        Self {
            command,
            id,
            label,
            detail: "",
            scope,
            default_shortcuts: &[],
            mnemonic: None,
            text_focus: TextFocusPolicy::Defer,
            repeat: RepeatPolicy::Reject,
        }
    }

    /// Explain the command's result rather than restating its label.
    #[must_use]
    pub const fn with_detail(mut self, detail: &'static str) -> Self {
        self.detail = detail;
        self
    }

    /// Install default accelerators in presentation order.
    #[must_use]
    pub const fn with_default_shortcuts(mut self, shortcuts: &'static [Shortcut]) -> Self {
        self.default_shortcuts = shortcuts;
        self
    }

    /// Add an Alt mnemonic whose glyph is visibly underlined in the label.
    #[must_use]
    pub const fn with_mnemonic(mut self, mnemonic: char) -> Self {
        self.mnemonic = Some(mnemonic);
        self
    }

    /// Choose whether the command may supersede focused text entry.
    #[must_use]
    pub const fn with_text_focus(mut self, policy: TextFocusPolicy) -> Self {
        self.text_focus = policy;
        self
    }

    /// Choose whether held-key repeats reinvoke the command.
    #[must_use]
    pub const fn with_repeat(mut self, policy: RepeatPolicy) -> Self {
        self.repeat = policy;
        self
    }

    /// Typed application command.
    pub const fn command(&self) -> C
    where
        C: Copy,
    {
        self.command
    }

    /// Stable, application-namespaced configuration identity.
    pub const fn id(&self) -> &'static str {
        self.id
    }

    /// Visible action label.
    pub const fn label(&self) -> &'static str {
        self.label
    }

    /// Visible consequence description.
    pub const fn detail(&self) -> &'static str {
        self.detail
    }

    /// Application scope.
    pub const fn scope(&self) -> CommandScope<S>
    where
        S: Copy,
    {
        self.scope
    }

    /// Default accelerators.
    pub const fn default_shortcuts(&self) -> &'static [Shortcut] {
        self.default_shortcuts
    }

    /// Alt mnemonic, when one exists.
    pub const fn mnemonic_key(&self) -> Option<char> {
        self.mnemonic
    }

    /// Focus ownership law.
    pub const fn text_focus_policy(&self) -> TextFocusPolicy {
        self.text_focus
    }

    /// Held-key repeat law.
    pub const fn repeat_policy(&self) -> RepeatPolicy {
        self.repeat
    }

    /// Typographic label with the declared Alt mnemonic underlined.
    pub fn widget_text(&self, ui: &egui::Ui) -> egui::WidgetText {
        self.widget_text_with_font(ui, &egui::TextStyle::Button.resolve(ui.style()))
    }

    pub(crate) fn widget_text_with_font(
        &self,
        ui: &egui::Ui,
        font: &egui::FontId,
    ) -> egui::WidgetText {
        self.mnemonic.map_or_else(
            || egui::RichText::new(self.label).font(font.clone()).into(),
            |mnemonic| {
                brass_poolrooms::chrome::MnemonicText::new(self.label, mnemonic)
                    .widget_text_with_font(ui, font.clone())
            },
        )
    }

    fn bindings(&self) -> impl Iterator<Item = Shortcut> + '_ {
        self.default_shortcuts
            .iter()
            .copied()
            .chain(self.mnemonic.map(Shortcut::mnemonic))
    }
}

/// Generated command-button response with exact, non-bleeding key ownership.
///
/// It dereferences to the underlying egui response for geometry and focus.
/// [`Self::clicked`] admits pointer, touch, accessibility, or a fresh
/// unmodified Enter/Space activation only.
#[derive(Debug)]
pub struct CommandButtonResponse {
    response: egui::Response,
    activated: bool,
}

impl CommandButtonResponse {
    /// Whether this button received one lawful activation.
    #[must_use]
    pub const fn clicked(&self) -> bool {
        self.activated
    }

    /// Discard refined activation semantics and return the raw egui response.
    #[must_use]
    pub fn into_response(self) -> egui::Response {
        self.response
    }
}

impl Deref for CommandButtonResponse {
    type Target = egui::Response;

    fn deref(&self) -> &Self::Target {
        &self.response
    }
}

/// Per-frame command availability.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum CommandStatus<'reason> {
    /// Visible and invokable.
    #[default]
    Enabled,
    /// Visible but unavailable for the stated reason.
    Disabled(&'reason str),
    /// Absent from routing and contextual guidance.
    Hidden,
}

/// Result of consuming one command accelerator.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommandDispatch<'reason, C> {
    /// Invoke this application-owned command exactly once.
    Invoke(C),
    /// The command owned the keystroke but refused execution.
    Refused {
        /// Command that refused execution.
        command: C,
        /// Human-readable reason suitable for immediate feedback.
        reason: &'reason str,
    },
}

/// Validated single source of truth for one application's commands.
#[derive(Debug)]
pub struct CommandCanon<C: 'static, S: 'static> {
    specs: &'static [CommandSpec<C, S>],
}

impl<C, S> CommandCanon<C, S>
where
    C: Copy + Debug + Eq + 'static,
    S: Copy + Debug + Eq + 'static,
{
    /// Validate and retain a static command declaration.
    ///
    /// Configuration errors are programmer faults and fail immediately. IDs
    /// must be lowercase dotted names. Commands, IDs, mnemonics, and bindings
    /// must be unique wherever their scopes can coexist.
    pub fn new(specs: &'static [CommandSpec<C, S>]) -> Self {
        validate(specs);
        Self { specs }
    }

    /// Complete declaration in stable presentation order.
    #[must_use]
    pub const fn specs(&self) -> &'static [CommandSpec<C, S>] {
        self.specs
    }

    /// Find one declaration by its typed command value.
    pub fn spec(&self, command: C) -> &'static CommandSpec<C, S> {
        self.specs
            .iter()
            .find(|spec| spec.command == command)
            .unwrap_or_else(|| panic!("undeclared command {command:?}"))
    }

    /// Effective non-mnemonic accelerators for one command.
    ///
    /// This is the future custom-keymap seam. It returns declaration defaults
    /// today through a slice borrowed from the canon; routing, button legends,
    /// and the guide all consult this projection rather than reading defaults
    /// independently. A future canon may therefore own merged bindings without
    /// changing consumers.
    pub fn shortcuts(&self, command: C) -> &[Shortcut] {
        self.spec(command).default_shortcuts
    }

    /// Render a standard button from the canon's declaration and bindings.
    ///
    /// This is an accelerator surface only: the returned click remains a typed
    /// application action, and keyboard dispatch still flows through [`Self::route`].
    pub fn button(&self, command: C, ui: &mut egui::Ui) -> CommandButtonResponse {
        let spec = self.spec(command);
        let mut button = egui::Button::new(spec.widget_text(ui));
        if let Some(shortcut) = self.bindings(spec).next() {
            button = button.shortcut_text(shortcut.label(ui.ctx()));
        }
        let response = ui.add(button);
        let activated = brass_poolrooms::chrome::exact_activation(ui, &response);
        if activated {
            response.request_focus();
        }
        CommandButtonResponse {
            response,
            activated,
        }
    }

    /// Consume at most one exact accelerator and return its typed consequence.
    ///
    /// Contexts are ordered from most specific to least specific. Contextual
    /// bindings beat global bindings; a hidden command relinquishes its chord.
    /// A preceding modal layer suspends routing and retains every key.
    pub fn route<'reason>(
        &self,
        ctx: &egui::Context,
        contexts: &[S],
        status: impl Fn(C) -> CommandStatus<'reason>,
    ) -> Option<CommandDispatch<'reason, C>> {
        if ctx.memory(|memory| memory.top_modal_layer().is_some()) {
            return None;
        }
        self.route_unchecked(ctx, contexts, status)
    }

    /// Consume at most one accelerator while an application-owned modal layer
    /// is topmost.
    ///
    /// This admits the modal's own command context without letting it pierce a
    /// later modal above it. Ordinary application surfaces should use
    /// [`Self::route`].
    pub fn route_in_modal<'reason>(
        &self,
        ctx: &egui::Context,
        layer: egui::LayerId,
        contexts: &[S],
        status: impl Fn(C) -> CommandStatus<'reason>,
    ) -> Option<CommandDispatch<'reason, C>> {
        if ctx.memory(egui::Memory::top_modal_layer) != Some(layer) {
            return None;
        }
        self.route_unchecked(ctx, contexts, status)
    }

    fn route_unchecked<'reason>(
        &self,
        ctx: &egui::Context,
        contexts: &[S],
        status: impl Fn(C) -> CommandStatus<'reason>,
    ) -> Option<CommandDispatch<'reason, C>> {
        for context in contexts {
            for spec in self
                .specs
                .iter()
                .filter(|spec| spec.scope == CommandScope::Context(*context))
            {
                match route_spec(ctx, spec, self.bindings(spec), status(spec.command)) {
                    Route::Miss => {}
                    Route::Consumed => return None,
                    Route::Dispatch(dispatch) => return Some(dispatch),
                }
            }
        }
        for spec in self
            .specs
            .iter()
            .filter(|spec| spec.scope == CommandScope::Global)
        {
            match route_spec(ctx, spec, self.bindings(spec), status(spec.command)) {
                Route::Miss => {}
                Route::Consumed => return None,
                Route::Dispatch(dispatch) => return Some(dispatch),
            }
        }
        None
    }

    fn bindings<'canon>(
        &'canon self,
        spec: &'canon CommandSpec<C, S>,
    ) -> impl Iterator<Item = Shortcut> + 'canon {
        self.shortcuts(spec.command)
            .iter()
            .copied()
            .chain(spec.mnemonic.map(Shortcut::mnemonic))
    }
}

enum Route<'reason, C> {
    Miss,
    Consumed,
    Dispatch(CommandDispatch<'reason, C>),
}

fn route_spec<'reason, C: Copy, S>(
    ctx: &egui::Context,
    spec: &CommandSpec<C, S>,
    bindings: impl Iterator<Item = Shortcut>,
    status: CommandStatus<'reason>,
) -> Route<'reason, C> {
    if status == CommandStatus::Hidden
        || (spec.text_focus == TextFocusPolicy::Defer && ctx.text_edit_focused())
    {
        return Route::Miss;
    }
    for shortcut in bindings {
        let stroke = take(ctx, shortcut);
        if stroke == Stroke::None {
            continue;
        }
        if stroke == Stroke::Repeat && spec.repeat == RepeatPolicy::Reject {
            return Route::Consumed;
        }
        return match status {
            CommandStatus::Enabled => Route::Dispatch(CommandDispatch::Invoke(spec.command)),
            CommandStatus::Disabled(reason) => Route::Dispatch(CommandDispatch::Refused {
                command: spec.command,
                reason,
            }),
            CommandStatus::Hidden => Route::Miss,
        };
    }
    Route::Miss
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Stroke {
    None,
    Fresh,
    Repeat,
}

pub(crate) fn take(ctx: &egui::Context, shortcut: Shortcut) -> Stroke {
    if shortcut.key.egui().is_none() {
        return Stroke::None;
    }
    ctx.input_mut(|input| {
        let mut fresh = false;
        let mut repeated = false;
        input.events.retain(|event| {
            let egui::Event::Key {
                key: candidate,
                pressed: true,
                repeat,
                modifiers,
                ..
            } = event
            else {
                return true;
            };
            let matched = shortcut.matches(*candidate, *modifiers);
            if matched {
                fresh |= !*repeat;
                repeated |= *repeat;
            }
            !matched
        });
        if fresh || repeated {
            input.events.retain(
                |event| !matches!(event, egui::Event::Text(text) if shortcut.key.matches_text(text)),
            );
        }
        if fresh {
            Stroke::Fresh
        } else if repeated {
            Stroke::Repeat
        } else {
            Stroke::None
        }
    })
}

fn validate<C, S>(specs: &[CommandSpec<C, S>])
where
    C: Copy + Debug + Eq,
    S: Copy + Debug + Eq,
{
    for (index, spec) in specs.iter().enumerate() {
        assert!(valid_id(spec.id), "invalid command ID '{}'", spec.id);
        assert!(
            !spec.label.trim().is_empty(),
            "command labels cannot be empty"
        );
        if let Some(mnemonic) = spec.mnemonic {
            assert!(
                mnemonic.is_ascii_alphanumeric(),
                "invalid mnemonic '{mnemonic}'"
            );
            assert!(
                spec.label
                    .chars()
                    .any(|candidate| candidate.eq_ignore_ascii_case(&mnemonic)),
                "mnemonic '{mnemonic}' does not occur in '{}'",
                spec.label
            );
        }
        for shortcut in spec.bindings() {
            assert!(shortcut.valid(), "invalid shortcut in '{}'", spec.id);
            assert!(
                RESERVED.iter().all(|reserved| !reserved.collides(shortcut)),
                "shortcut {shortcut:?} in '{}' is reserved by the shared interaction grammar",
                spec.id
            );
        }
        for prior in &specs[..index] {
            assert!(
                prior.command != spec.command,
                "duplicate command {:?}",
                spec.command
            );
            assert!(prior.id != spec.id, "duplicate command ID '{}'", spec.id);
            if scopes_overlap(prior.scope, spec.scope) {
                for left in prior.bindings() {
                    for right in spec.bindings() {
                        assert!(
                            !left.collides(right),
                            "shortcut collision between '{}' and '{}'",
                            prior.id,
                            spec.id
                        );
                    }
                }
            }
        }
        let bindings = spec.bindings().collect::<Vec<_>>();
        for (position, binding) in bindings.iter().enumerate() {
            assert!(
                !bindings[..position]
                    .iter()
                    .any(|prior| prior.collides(*binding)),
                "duplicate shortcut in '{}'",
                spec.id
            );
        }
    }
}

fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && !id.starts_with('.')
        && !id.ends_with('.')
        && id.split('.').all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
        })
}

fn scopes_overlap<S: Eq>(left: CommandScope<S>, right: CommandScope<S>) -> bool {
    match (left, right) {
        (CommandScope::Global, _) | (_, CommandScope::Global) => true,
        (CommandScope::Context(left), CommandScope::Context(right)) => left == right,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum Command {
        Save,
        Rename,
        Search,
        Next,
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum Scope {
        Library,
        Viewer,
    }

    const SAVE: [Shortcut; 1] = [Shortcut::primary('S')];
    const RENAME: [Shortcut; 1] = [Shortcut::new(
        ShortcutModifiers::NONE,
        ShortcutKey::Function(2),
    )];
    const SEARCH: [Shortcut; 1] = [Shortcut::new(ShortcutModifiers::NONE, ShortcutKey::Slash)];
    const NEXT: [Shortcut; 1] = [Shortcut::new(
        ShortcutModifiers::ALT,
        ShortcutKey::ArrowRight,
    )];
    const SPECS: [CommandSpec<Command, Scope>; 4] = [
        CommandSpec::new(Command::Save, "document.save", "Save", CommandScope::Global)
            .with_default_shortcuts(&SAVE),
        CommandSpec::new(
            Command::Rename,
            "library.rename",
            "Rename",
            CommandScope::Context(Scope::Library),
        )
        .with_default_shortcuts(&RENAME)
        .with_mnemonic('R'),
        CommandSpec::new(
            Command::Search,
            "library.search",
            "Search",
            CommandScope::Context(Scope::Library),
        )
        .with_default_shortcuts(&SEARCH),
        CommandSpec::new(
            Command::Next,
            "viewer.next",
            "Next",
            CommandScope::Context(Scope::Viewer),
        )
        .with_default_shortcuts(&NEXT)
        .with_repeat(RepeatPolicy::Allow),
    ];

    fn key(modifiers: egui::Modifiers, key: egui::Key, repeat: bool) -> egui::RawInput {
        egui::RawInput {
            events: vec![
                egui::Event::ModifiersChanged(modifiers),
                egui::Event::Key {
                    key,
                    physical_key: Some(key),
                    pressed: true,
                    repeat,
                    modifiers,
                },
            ],
            ..egui::RawInput::default()
        }
    }

    #[test]
    fn exact_routing_owns_both_physical_and_textual_projections() {
        let canon = CommandCanon::new(&SPECS);
        let ctx = egui::Context::default();
        let primary = egui::Modifiers::CTRL.plus(egui::Modifiers::COMMAND);
        let overmodified = primary.plus(egui::Modifiers::SHIFT);
        let mut dispatch = None;
        ctx.run_ui(key(overmodified, egui::Key::S, false), |ui| {
            dispatch = canon.route(ui.ctx(), &[Scope::Library], |_| CommandStatus::Enabled);
        })
        .drop_without_applying_deltas();
        assert_eq!(dispatch, None);
        assert!(ctx.input(|input| input.key_pressed(egui::Key::S)));

        let ctx = egui::Context::default();
        let mut input = key(egui::Modifiers::ALT, egui::Key::R, false);
        input.events.push(egui::Event::Text("r".to_owned()));
        ctx.run_ui(input, |ui| {
            dispatch = canon.route(ui.ctx(), &[Scope::Library], |_| CommandStatus::Enabled);
        })
        .drop_without_applying_deltas();
        assert_eq!(dispatch, Some(CommandDispatch::Invoke(Command::Rename)));
        assert!(ctx.input(|input| {
            input
                .events
                .iter()
                .all(|event| !matches!(event, egui::Event::Key { .. } | egui::Event::Text(_)))
        }));
    }

    #[test]
    fn dynamic_routing_preserves_disabled_repeat_and_text_owners() {
        let canon = CommandCanon::new(&SPECS);
        let ctx = egui::Context::default();
        let mut dispatch = None;
        ctx.run_ui(key(egui::Modifiers::NONE, egui::Key::F2, false), |ui| {
            dispatch = canon.route(ui.ctx(), &[Scope::Library], |command| {
                if command == Command::Rename {
                    CommandStatus::Disabled("select an item first")
                } else {
                    CommandStatus::Enabled
                }
            });
        })
        .drop_without_applying_deltas();
        assert_eq!(
            dispatch,
            Some(CommandDispatch::Refused {
                command: Command::Rename,
                reason: "select an item first",
            })
        );

        ctx.run_ui(key(egui::Modifiers::NONE, egui::Key::F2, true), |ui| {
            dispatch = canon.route(ui.ctx(), &[Scope::Library], |_| CommandStatus::Enabled);
        })
        .drop_without_applying_deltas();
        assert_eq!(dispatch, None);
        assert!(!ctx.input(|input| input.key_pressed(egui::Key::F2)));

        ctx.run_ui(
            key(egui::Modifiers::ALT, egui::Key::ArrowRight, true),
            |ui| {
                dispatch = canon.route(ui.ctx(), &[Scope::Viewer], |_| CommandStatus::Enabled);
            },
        )
        .drop_without_applying_deltas();
        assert_eq!(dispatch, Some(CommandDispatch::Invoke(Command::Next)));

        let ctx = egui::Context::default();
        let mut text = String::new();
        ctx.run_ui(egui::RawInput::default(), |ui| {
            ui.text_edit_singleline(&mut text).request_focus();
        })
        .drop_without_applying_deltas();
        ctx.run_ui(key(egui::Modifiers::NONE, egui::Key::Slash, false), |ui| {
            dispatch = canon.route(ui.ctx(), &[Scope::Library], |_| CommandStatus::Enabled);
            let _editor = ui.text_edit_singleline(&mut text);
        })
        .drop_without_applying_deltas();
        assert_eq!(dispatch, None);
        assert!(ctx.input(|input| input.key_pressed(egui::Key::Slash)));
    }

    #[test]
    fn command_routing_cannot_pierce_a_modal_layer() {
        let canon = CommandCanon::new(&SPECS);
        let ctx = egui::Context::default();
        let modal_id = egui::Id::new("command-barrier");
        let modal_layer = egui::LayerId::new(egui::Order::Foreground, modal_id);
        let modal = || egui::Modal::new(modal_id);
        ctx.run_ui(egui::RawInput::default(), |ui| {
            let _modal = modal().show(ui.ctx(), |ui| ui.label("modal"));
        })
        .drop_without_applying_deltas();

        let primary = egui::Modifiers::CTRL.plus(egui::Modifiers::COMMAND);
        let mut dispatch = None;
        ctx.run_ui(key(primary, egui::Key::S, false), |ui| {
            dispatch = canon.route(ui.ctx(), &[Scope::Library], |_| CommandStatus::Enabled);
            let _modal = modal().show(ui.ctx(), |ui| ui.label("modal"));
        })
        .drop_without_applying_deltas();

        assert_eq!(dispatch, None);
        assert!(ctx.input(|input| input.key_pressed(egui::Key::S)));
        assert_eq!(ctx.memory(egui::Memory::top_modal_layer), Some(modal_layer));
        ctx.run_ui(
            egui::RawInput {
                events: vec![egui::Event::Key {
                    key: egui::Key::S,
                    physical_key: Some(egui::Key::S),
                    pressed: false,
                    repeat: false,
                    modifiers: primary,
                }],
                ..egui::RawInput::default()
            },
            |ui| {
                let _modal = modal().show(ui.ctx(), |ui| ui.label("modal"));
            },
        )
        .drop_without_applying_deltas();

        ctx.run_ui(key(primary, egui::Key::S, false), |ui| {
            dispatch = canon.route_in_modal(ui.ctx(), modal_layer, &[Scope::Library], |_| {
                CommandStatus::Enabled
            });
            let _modal = modal().show(ui.ctx(), |ui| ui.label("modal"));
        })
        .drop_without_applying_deltas();
        assert_eq!(dispatch, Some(CommandDispatch::Invoke(Command::Save)));
    }

    #[test]
    fn canon_rejects_ambiguous_or_reserved_bindings() {
        fn rejection(case: impl FnOnce() + std::panic::UnwindSafe, needle: &str) {
            let panic = std::panic::catch_unwind(case).expect_err("invalid canon was admitted");
            let message = panic
                .downcast_ref::<String>()
                .map(String::as_str)
                .or_else(|| panic.downcast_ref::<&str>().copied())
                .unwrap_or("");
            assert!(message.contains(needle), "unexpected panic: {message}");
        }

        rejection(
            || {
                const COLLISION: [CommandSpec<Command, Scope>; 2] = [
                    CommandSpec::new(Command::Save, "document.save", "Save", CommandScope::Global)
                        .with_default_shortcuts(&SAVE),
                    CommandSpec::new(
                        Command::Search,
                        "library.search",
                        "Search",
                        CommandScope::Context(Scope::Library),
                    )
                    .with_default_shortcuts(&SAVE),
                ];
                let _canon = CommandCanon::new(&COLLISION);
            },
            "shortcut collision",
        );
        rejection(
            || {
                const CASED: [Shortcut; 2] = [Shortcut::primary('S'), Shortcut::primary('s')];
                const COLLISION: [CommandSpec<Command, Scope>; 1] = [CommandSpec::new(
                    Command::Save,
                    "document.save",
                    "Save",
                    CommandScope::Global,
                )
                .with_default_shortcuts(&CASED)];
                let _canon = CommandCanon::new(&COLLISION);
            },
            "duplicate shortcut",
        );
        rejection(
            || {
                const STOLEN: [Shortcut; 1] = [Shortcut::new(
                    ShortcutModifiers::NONE,
                    ShortcutKey::ArrowRight,
                )];
                const COLLISION: [CommandSpec<Command, Scope>; 1] = [CommandSpec::new(
                    Command::Next,
                    "viewer.next",
                    "Next",
                    CommandScope::Context(Scope::Viewer),
                )
                .with_default_shortcuts(&STOLEN)];
                let _canon = CommandCanon::new(&COLLISION);
            },
            "reserved by the shared interaction grammar",
        );
    }
}
