//! Hermetic native acceptance for the command and keyboard Atelier tab.

#![expect(
    unused_crate_dependencies,
    reason = "this acceptance target shares the package manifest with the Atelier product target"
)]

#[cfg(all(target_os = "linux", feature = "egui-test"))]
use std::{
    path::{Path, PathBuf},
    thread,
    time::Duration,
};

#[cfg(all(target_os = "linux", feature = "egui-test"))]
use anyhow::{Context as _, Result, ensure};
#[cfg(all(target_os = "linux", feature = "egui-test"))]
use egui_tester::{
    AppCommand, Button, Key, Modifiers, PixelRegion, Probe, Testbed, Wheel, WindowQuery,
};
#[cfg(all(target_os = "linux", feature = "egui-test"))]
use serde::Deserialize;

#[cfg(all(target_os = "linux", feature = "egui-test"))]
const WAIT: Duration = Duration::from_secs(5);
#[cfg(all(target_os = "linux", feature = "egui-test"))]
const STARTUP_WAIT: Duration = Duration::from_secs(30);
#[cfg(all(target_os = "linux", feature = "egui-test"))]
const CLIPBOARD_PROBE: &str = "--clipboard-probe";

#[cfg(all(target_os = "linux", feature = "egui-test"))]
#[derive(Debug, Deserialize)]
struct Observation {
    page: String,
    guide_open: bool,
    status: String,
    selected: bool,
    filter: String,
    density: u16,
    inspector_expanded: bool,
    inspector_extent: f32,
    settings: SettingsObservation,
}

#[cfg(all(target_os = "linux", feature = "egui-test"))]
#[derive(Debug, Deserialize)]
struct SettingsObservation {
    open: bool,
    fault: bool,
}

#[cfg(all(target_os = "linux", feature = "egui-test"))]
fn main() -> Result<()> {
    let mut arguments = std::env::args_os().skip(1);
    let first = arguments
        .next()
        .context("usage: atelier_acceptance PATH_TO_ATELIER [ARTIFACT_DIRECTORY]")?;
    if first == CLIPBOARD_PROBE {
        ensure!(arguments.next().is_none(), "usage: {CLIPBOARD_PROBE}");
        let mut clipboard = arboard::Clipboard::new().context("open native clipboard")?;
        print!("{}", clipboard.get_text().context("read native clipboard")?);
        return Ok(());
    }
    let binary = PathBuf::from(first);
    let artifacts = arguments.next().map(PathBuf::from);
    ensure!(
        arguments.next().is_none(),
        "usage: atelier_acceptance PATH_TO_ATELIER [ARTIFACT_DIRECTORY]"
    );
    ensure!(
        binary.is_file(),
        "Atelier binary not found: {}",
        binary.display()
    );

    let testbed = Testbed::raise().context("raise hermetic X11 testbed")?;
    let app = testbed
        .launch(
            AppCommand::new(&binary)
                .witness("probes/atelier.observations")
                .runtime(Duration::from_mins(1)),
        )
        .context("launch instrumented Atelier")?;
    let session = testbed
        .x11_session(
            &app,
            WindowQuery::title_exact("Eternalist · application primitive atelier"),
            Duration::from_secs(15),
        )
        .context("find Atelier window")?;
    session.focus().context("focus Atelier window")?;
    let mut probe: Probe<Observation> = app.witness()?.typed();
    let _presented = probe.wait_surface_presented(&app, STARTUP_WAIT)?;
    command_story(&testbed, &session, &app, &mut probe, artifacts.as_deref())?;
    settings_story(&session, &app, &mut probe, artifacts.as_deref())?;
    visible_background_water_story(&testbed, &binary, &session, &app, &mut probe)?;

    let _close = session.close()?;
    let exit = app.wait(WAIT).context("Atelier to honor native close")?;
    ensure!(exit.success(), "Atelier close failed: {exit:#?}");
    app.terminate().context("collect Atelier cgroup")?;
    short_modal_story(&testbed, &binary, artifacts.as_deref())?;
    Ok(())
}

#[cfg(all(target_os = "linux", feature = "egui-test"))]
fn short_modal_story(testbed: &Testbed, binary: &Path, artifacts: Option<&Path>) -> Result<()> {
    let app = testbed.launch(
        AppCommand::new(binary)
            .arg("--short-window")
            .witness("probes/atelier-short.observations")
            .runtime(Duration::from_mins(1)),
    )?;
    let session = testbed.x11_session(
        &app,
        WindowQuery::title_exact("Eternalist · application primitive atelier"),
        Duration::from_secs(15),
    )?;
    session.focus()?;
    let mut probe: Probe<Observation> = app.witness()?.typed();
    let _presented = probe.wait_surface_presented(&app, STARTUP_WAIT)?;

    let _ready = probe.wait(&app, WAIT, "short Settings sheet", |frame| {
        frame.state.page == "settings" && frame.state.settings.open
    })?;
    let _painted = probe.wait_fresh(&app, WAIT)?;
    capture_optional(&session, artifacts, "settings-ready-short.png")?;
    assert_modal_containment(
        &session,
        &app,
        &mut probe,
        "atelier.settings.sheet",
        "eternalist.settings.body",
    )?;
    reveal_settings_target(&session, &app, &mut probe, "eternalist.settings.path", 6)?;

    let _escape = session.key(Key::Escape)?;
    let _closed = probe.wait(&app, WAIT, "close short Settings", |frame| {
        !frame.state.settings.open
    })?;
    click_target(&session, &app, &mut probe, "atelier.settings.fault")?;
    let _faulted = probe.wait(&app, WAIT, "faulted short Settings", |frame| {
        frame.state.settings.fault && frame.state.settings.open
    })?;
    assert_modal_containment(
        &session,
        &app,
        &mut probe,
        "atelier.settings.sheet",
        "eternalist.settings.body",
    )?;
    reveal_settings_target(&session, &app, &mut probe, "eternalist.settings.fault", 8)?;
    reveal_settings_target(&session, &app, &mut probe, "eternalist.settings.path", 10)?;
    capture_optional(&session, artifacts, "settings-fault-short.png")?;

    let _escape = session.key(Key::Escape)?;
    let _closed = probe.wait(&app, WAIT, "close faulted short Settings", |frame| {
        !frame.state.settings.open
    })?;
    click_target(&session, &app, &mut probe, "atelier.tab.commands")?;
    let _commands = probe.wait(&app, WAIT, "short Commands tab", |frame| {
        frame.state.page == "commands"
    })?;
    let _help = session.key(Key::Function(1))?;
    let _guide = probe.wait(&app, WAIT, "short Help guide", |frame| {
        frame.state.guide_open
    })?;
    let _painted = probe.wait_fresh(&app, WAIT)?;
    capture_optional(&session, artifacts, "help-short.png")?;
    assert_modal_containment(
        &session,
        &app,
        &mut probe,
        "atelier.commands.guide",
        "eternalist.command-guide.body",
    )?;
    let body = probe.wait_anchor(&app, "eternalist.command-guide.body", WAIT)?;
    let before_scroll = session.capture()?;
    let (x, y) = body.center();
    let _scroll = session.wheel(x, y, 8, Wheel::default())?;
    let _scrolled = probe.wait_fresh(&app, WAIT)?;
    let after_scroll = session.capture()?;
    let changed = before_scroll.difference_region(&after_scroll, PixelRegion::anchor(&body), 2)?;
    ensure!(
        changed > 0.04,
        "mouse wheel changed only {changed:.4} of the command-guide body pixels"
    );
    assert_modal_containment(
        &session,
        &app,
        &mut probe,
        "atelier.commands.guide",
        "eternalist.command-guide.body",
    )?;

    let _close = session.close()?;
    let exit = app
        .wait(WAIT)
        .context("short Atelier to honor native close")?;
    ensure!(exit.success(), "short Atelier close failed: {exit:#?}");
    app.terminate().context("collect short Atelier cgroup")?;
    Ok(())
}

#[cfg(all(target_os = "linux", feature = "egui-test"))]
fn reveal_settings_target(
    session: &egui_tester::X11Session<'_, '_>,
    app: &egui_tester::Application<'_>,
    probe: &mut Probe<Observation>,
    target: &str,
    magnitude: i32,
) -> Result<()> {
    ensure!(magnitude > 0, "settings reveal magnitude must be positive");
    for _attempt in 0..3 {
        let body = probe.wait_anchor(app, "eternalist.settings.body", WAIT)?;
        let target = probe.wait_anchor(app, target, WAIT)?;
        if ensure_anchor_contained(&body, &target).is_ok() {
            return Ok(());
        }
        let (x, y) = body.center();
        let direction = if target.rect[3] > body.rect[3] { 1 } else { -1 };
        let _scroll = session.wheel(x, y, direction * magnitude, Wheel::default())?;
        let _scrolled = probe.wait_fresh(app, WAIT)?;
    }
    let body = probe.wait_anchor(app, "eternalist.settings.body", WAIT)?;
    let target = probe.wait_anchor(app, target, WAIT)?;
    ensure_anchor_contained(&body, &target)
}

#[cfg(all(target_os = "linux", feature = "egui-test"))]
fn assert_modal_containment(
    session: &egui_tester::X11Session<'_, '_>,
    app: &egui_tester::Application<'_>,
    probe: &mut Probe<Observation>,
    card: &str,
    body: &str,
) -> Result<()> {
    let card = probe.wait_anchor(app, card, WAIT)?;
    let body = probe.wait_anchor(app, body, WAIT)?;
    ensure_anchor_contained(&card, &body)?;
    let frame = session.capture()?;
    let [left, top, right, bottom] = card.rect;
    ensure!(
        left >= -0.5
            && top >= -0.5
            && f64::from(right) <= f64::from(frame.width()) + 0.5
            && f64::from(bottom) <= f64::from(frame.height()) + 0.5,
        "modal card {:?} escaped {}×{} client pixels",
        card.rect,
        frame.width(),
        frame.height(),
    );
    Ok(())
}

#[cfg(all(target_os = "linux", feature = "egui-test"))]
fn ensure_anchor_contained(outer: &egui_tester::Anchor, inner: &egui_tester::Anchor) -> Result<()> {
    let [left, top, right, bottom] = outer.rect;
    let [inner_left, inner_top, inner_right, inner_bottom] = inner.rect;
    ensure!(
        inner_left >= left - 0.5
            && inner_top >= top - 0.5
            && inner_right <= right + 0.5
            && inner_bottom <= bottom + 0.5,
        "witness `{}` {:?} escaped `{}` {:?}",
        inner.name,
        inner.rect,
        outer.name,
        outer.rect,
    );
    Ok(())
}

#[cfg(all(target_os = "linux", feature = "egui-test"))]
fn capture_optional(
    session: &egui_tester::X11Session<'_, '_>,
    artifacts: Option<&Path>,
    name: &str,
) -> Result<()> {
    if let Some(directory) = artifacts {
        std::fs::create_dir_all(directory).context("create acceptance artifact directory")?;
        session.capture()?.save_png(directory.join(name))?;
    }
    Ok(())
}

#[cfg(all(target_os = "linux", feature = "egui-test"))]
fn settings_story(
    session: &egui_tester::X11Session<'_, '_>,
    app: &egui_tester::Application<'_>,
    probe: &mut Probe<Observation>,
    artifacts: Option<&Path>,
) -> Result<()> {
    click_target(session, app, probe, "atelier.tab.settings")?;
    let _opened = probe.wait(app, WAIT, "Settings tab to present its sheet", |frame| {
        frame.state.page == "settings" && frame.state.settings.open
    })?;
    if let Some(directory) = artifacts {
        thread::sleep(Duration::from_millis(250));
        let _settled = probe.wait_fresh(app, WAIT)?;
        std::fs::create_dir_all(directory).context("create acceptance artifact directory")?;
        session
            .capture()?
            .save_png(directory.join("settings-ready.png"))?;
    }
    let _escape = session.key(Key::Escape)?;
    let _closed = probe.wait(app, WAIT, "Escape to close settings", |frame| {
        !frame.state.settings.open
    })?;
    click_target(session, app, probe, "eternalist.application.help")?;
    let _guide = probe.wait(app, WAIT, "application header to open Help", |frame| {
        frame.state.guide_open
    })?;
    let _escape = session.key(Key::Escape)?;
    let _closed = probe.wait(app, WAIT, "Escape to close Help", |frame| {
        !frame.state.guide_open
    })?;
    click_target(session, app, probe, "atelier.settings.fault")?;
    let _fault = probe.wait(
        app,
        WAIT,
        "configuration fault to summon settings",
        |frame| frame.state.settings.fault && frame.state.settings.open,
    )?;
    if let Some(directory) = artifacts {
        thread::sleep(Duration::from_millis(250));
        let _settled = probe.wait_fresh(app, WAIT)?;
        session
            .capture()?
            .save_png(directory.join("settings-fault.png"))?;
    }
    Ok(())
}

#[cfg(all(target_os = "linux", feature = "egui-test"))]
fn visible_background_water_story(
    testbed: &Testbed,
    binary: &Path,
    session: &egui_tester::X11Session<'_, '_>,
    app: &egui_tester::Application<'_>,
    probe: &mut Probe<Observation>,
) -> Result<()> {
    let sentinel = testbed.launch(
        AppCommand::new(binary)
            .arg("--focus-sentinel")
            .runtime(Duration::from_mins(1)),
    )?;
    let sentinel_session = testbed.x11_session(
        &sentinel,
        WindowQuery::title_exact("Eternalist · focus sentinel"),
        STARTUP_WAIT,
    )?;

    session.focus()?;
    let tab = probe.wait_anchor(app, "atelier.tab.commands", WAIT)?;
    let (x, y) = tab.center();
    let _hover = session.move_to(x, y)?;
    let _armed = probe.wait_fresh(app, WAIT)?;
    let _departure = session.leave()?;
    let _departed = probe.wait_fresh(app, WAIT)?;
    sentinel_session.focus()?;
    // Drain every present that could have been queued before FocusOut. The
    // next witness must therefore be a continuation owned by visible
    // background presentation, not stale foreground work.
    thread::sleep(Duration::from_millis(120));
    let _background_baseline = probe.read()?;
    let _water_continues = probe
        .wait_fresh(app, Duration::from_millis(400))
        .context("visible water to continue after pointer departure and focus loss")?;

    let _close = sentinel_session.close()?;
    let exit = sentinel.wait(WAIT)?;
    ensure!(exit.success(), "second Atelier close failed: {exit:#?}");
    sentinel.terminate()?;
    session.focus()?;
    Ok(())
}

#[cfg(all(target_os = "linux", feature = "egui-test"))]
fn command_story(
    testbed: &Testbed,
    session: &egui_tester::X11Session<'_, '_>,
    app: &egui_tester::Application<'_>,
    probe: &mut Probe<Observation>,
    artifacts: Option<&Path>,
) -> Result<()> {
    click_target(session, app, probe, "atelier.tab.commands")?;
    let _commands = probe.wait(app, WAIT, "Commands tab", |frame| {
        frame.state.page == "commands"
    })?;
    inspector_story(session, app, probe)?;

    let _mnemonic = session.chord(Modifiers::ALT, Key::Character('o'))?;
    let _opened = probe.wait(app, WAIT, "Alt+O command", |frame| {
        frame.state.status == "opened archive"
    })?;

    click_target(session, app, probe, "atelier.commands.open")?;
    wait_focus(probe, app, "atelier.commands.open")?;
    let _tab = session.key(Key::Tab)?;
    wait_focus(probe, app, "atelier.commands.save")?;
    let _backward = session.chord(Modifiers::SHIFT, Key::Tab)?;
    wait_focus(probe, app, "atelier.commands.open")?;
    let _forward = session.key(Key::Tab)?;
    wait_focus(probe, app, "atelier.commands.save")?;
    let _space = session.key(Key::Space)?;
    let _saved = probe.wait(app, WAIT, "Space activates focused Save", |frame| {
        frame.state.status == "saved archive"
    })?;

    let _next_panel = session.chord(Modifiers::CTRL, Key::Tab)?;
    wait_focus(probe, app, "atelier.commands.panel.selection")?;
    let _tab = session.key(Key::Tab)?;
    wait_focus(probe, app, "atelier.commands.selected")?;
    let _space = session.key(Key::Space)?;
    let _cleared = probe.wait(app, WAIT, "Space toggles focused checkbox", |frame| {
        !frame.state.selected
    })?;

    let _refused = session.chord(Modifiers::ALT, Key::Character('r'))?;
    let _reason = probe.wait(app, WAIT, "disabled mnemonic refusal", |frame| {
        frame.state.status == "refused: select an item first"
    })?;
    let _focus_search = session.chord(Modifiers::ALT, Key::Character('f'))?;
    let _search = probe.wait(app, WAIT, "focus-search command", |frame| {
        frame.state.status == "focused search"
    })?;
    wait_focus(probe, app, "atelier.commands.search")
        .context("Alt+F to transfer focus into search")?;
    let _typed = session.type_text("ore")?;
    let _filter = probe.wait(app, WAIT, "search receives text", |frame| {
        frame.state.filter == "ore"
    })?;
    let _save = session.chord(Modifiers::CTRL, Key::Character('s'))?;
    let _captured = probe.wait(app, WAIT, "Save owns Ctrl+S during text entry", |frame| {
        frame.state.status == "saved archive" && frame.state.filter == "ore"
    })?;

    guide_story(session, app, probe, artifacts)?;

    let _next_panel = session.chord(Modifiers::CTRL, Key::Tab)?;
    wait_focus(probe, app, "atelier.commands.panel.density")?;
    let _tab = session.key(Key::Tab)?;
    wait_focus(probe, app, "atelier.commands.density")?;
    let _right = session.key(Key::Right)?;
    let _keyboard_adjusted = probe.wait(app, WAIT, "Right adjusts focused rail", |frame| {
        frame.state.density == 5
    })?;
    let rail = probe.wait_anchor(app, "atelier.commands.density", WAIT)?;
    let (x, y) = rail.center();
    let _wheel = session.wheel(x, y, -1, Wheel::default())?;
    let _wheel_adjusted = probe.wait(app, WAIT, "wheel adjusts hovered rail", |frame| {
        frame.state.density == 6
    })?;
    clipboard_story(testbed, session, app, probe)?;

    Ok(())
}

#[cfg(all(target_os = "linux", feature = "egui-test"))]
fn inspector_story(
    session: &egui_tester::X11Session<'_, '_>,
    app: &egui_tester::Application<'_>,
    probe: &mut Probe<Observation>,
) -> Result<()> {
    let boundary = probe.wait_anchor(app, "atelier.commands.inspector-boundary", WAIT)?;
    let (boundary_x, boundary_y) = boundary.center();
    let _hover = session.move_to(boundary_x, boundary_y)?;
    let actuator = probe.wait_anchor(app, "atelier.commands.inspector-actuator", WAIT)?;
    let deployed = session.capture()?;
    let (actuator_x, actuator_y) = actuator.center();
    let _hide = session.click(actuator_x, actuator_y, Button::Primary)?;
    let _concealed = probe.wait(
        app,
        WAIT,
        "border actuator conceals the inspector",
        |frame| !frame.state.inspector_expanded && frame.state.inspector_extent <= 0.5,
    )?;
    let inspector_region = PixelRegion::new(0, 82, 240, 780);
    let _concealed = session
        .wait_changed_region(&deployed, inspector_region, 0.02, 4, WAIT)
        .context("concealing the inspector did not materially reclaim its rendered region")?;

    let _hidden_edge = session.move_to(2, boundary_y)?;
    let hidden_actuator = probe.wait_anchor(app, "atelier.commands.inspector-actuator", WAIT)?;
    let (hidden_x, hidden_y) = hidden_actuator.center();
    let _show = session.click(hidden_x, hidden_y, Button::Primary)?;
    let _deployed = probe.wait(app, WAIT, "edge actuator reveals the inspector", |frame| {
        frame.state.inspector_expanded
            && frame.state.inspector_extent >= eternalist_apps::inspector::WIDTH - 0.5
    })?;

    let _hide_with_key = session.key(Key::Function(9))?;
    let _hidden_with_key = probe.wait(app, WAIT, "F9 conceals the inspector", |frame| {
        !frame.state.inspector_expanded && frame.state.inspector_extent <= 0.5
    })?;
    let _show_with_key = session.key(Key::Function(9))?;
    let _restored_with_key = probe.wait(app, WAIT, "F9 reveals the inspector", |frame| {
        frame.state.inspector_expanded
            && frame.state.inspector_extent >= eternalist_apps::inspector::WIDTH - 0.5
    })?;
    Ok(())
}

#[cfg(all(target_os = "linux", feature = "egui-test"))]
fn clipboard_story(
    testbed: &Testbed,
    session: &egui_tester::X11Session<'_, '_>,
    app: &egui_tester::Application<'_>,
    probe: &mut Probe<Observation>,
) -> Result<()> {
    let label = probe.wait_anchor(app, "atelier.commands.copy", WAIT)?;
    let (x, y) = label.center();
    let _selected = session.click(x, y, Button::Primary)?;
    let _selection_frame = probe.wait_fresh(app, WAIT)?;
    let _copy = session.chord(Modifiers::CTRL, Key::Character('c'))?;
    let _copy_frame = probe.wait_fresh(app, WAIT)?;

    let probe = std::env::current_exe().context("resolve clipboard probe")?;
    let clipboard = testbed.launch(AppCommand::new(probe).arg(CLIPBOARD_PROBE).runtime(WAIT))?;
    let exit = clipboard.wait(WAIT)?;
    ensure!(
        exit.success(),
        "clipboard probe could not read the native clipboard: {exit:#?}"
    );
    ensure!(
        exit.stdout.trim() == "COPY CAPABILITY SENTINEL",
        "native clipboard contains {:?}",
        exit.stdout,
    );
    clipboard.terminate().context("collect clipboard probe")?;
    Ok(())
}

#[cfg(all(target_os = "linux", feature = "egui-test"))]
fn guide_story(
    session: &egui_tester::X11Session<'_, '_>,
    app: &egui_tester::Application<'_>,
    probe: &mut Probe<Observation>,
    artifacts: Option<&Path>,
) -> Result<()> {
    let before_help = session.capture()?;
    let _help = session.key(Key::Function(1))?;
    let _opened = probe.wait(app, WAIT, "F1 opens generated help", |frame| {
        frame.state.guide_open
    })?;
    let guide = probe.wait_anchor(app, "atelier.commands.guide", WAIT)?;
    let guide_region = PixelRegion::anchor(&guide);
    let with_help = session.wait_changed_region(&before_help, guide_region, 0.55, 2, WAIT)?;
    if let Some(directory) = artifacts {
        std::fs::create_dir_all(directory).context("create acceptance artifact directory")?;
        before_help.save_png(directory.join("before-help.png"))?;
        with_help.save_png(directory.join("with-help.png"))?;
    }
    let help_difference = before_help.difference_region(&with_help, guide_region, 2)?;
    ensure!(
        help_difference > 0.55,
        "the witnessed guide changed only {help_difference:.4} of its own card pixels"
    );
    let _blocked_panel = session.chord(Modifiers::CTRL, Key::Tab)?;
    let blocked = probe.wait_fresh(app, WAIT)?;
    ensure!(
        blocked.state.guide_open,
        "Control+Tab escaped through the open command guide"
    );
    let _blocked_command = session.chord(Modifiers::ALT, Key::Character('o'))?;
    let blocked = probe.wait_fresh(app, WAIT)?;
    ensure!(
        blocked.state.guide_open,
        "Alt+O escaped through the open command guide"
    );
    let _blocked_inspector = session.key(Key::Function(9))?;
    let blocked = probe.wait_fresh(app, WAIT)?;
    ensure!(
        blocked.state.guide_open && blocked.state.inspector_expanded,
        "F9 escaped through the open command guide"
    );
    let _escape = session.key(Key::Escape)?;
    let closed = probe.wait(app, WAIT, "Escape closes only the guide", |frame| {
        !frame.state.guide_open
    })?;
    ensure!(
        closed.state.status == "saved archive",
        "a background command escaped through the guide: {}",
        closed.state.status
    );
    wait_focus(probe, app, "atelier.commands.search")
        .context("Escape to restore focus into search")?;
    Ok(())
}

#[cfg(all(target_os = "linux", feature = "egui-test"))]
fn click_target(
    session: &egui_tester::X11Session<'_, '_>,
    app: &egui_tester::Application<'_>,
    probe: &mut Probe<Observation>,
    target: &str,
) -> Result<()> {
    let anchor = probe.wait_anchor(app, target, WAIT)?;
    let (x, y) = anchor.center();
    let _click = session.click(x, y, Button::Primary)?;
    Ok(())
}

#[cfg(all(target_os = "linux", feature = "egui-test"))]
fn wait_focus(
    probe: &mut Probe<Observation>,
    app: &egui_tester::Application<'_>,
    target: &str,
) -> Result<()> {
    let _focused = probe.wait_focus(app, target, WAIT)?;
    Ok(())
}

#[cfg(not(all(target_os = "linux", feature = "egui-test")))]
fn main() {}
