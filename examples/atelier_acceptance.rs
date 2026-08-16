//! Hermetic native acceptance for the command and keyboard Atelier tab.

#![expect(
    unused_crate_dependencies,
    reason = "this acceptance target shares the package manifest with the Atelier product target"
)]

#[cfg(all(target_os = "linux", feature = "egui-test"))]
use std::{
    path::{Path, PathBuf},
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
#[derive(Debug, Deserialize)]
struct Observation {
    page: String,
    guide_open: bool,
    status: String,
    selected: bool,
    filter: String,
    density: u16,
}

#[cfg(all(target_os = "linux", feature = "egui-test"))]
fn main() -> Result<()> {
    let mut arguments = std::env::args_os().skip(1);
    let binary = arguments
        .next()
        .map(PathBuf::from)
        .context("usage: atelier_acceptance PATH_TO_ATELIER [ARTIFACT_DIRECTORY]")?;
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
            AppCommand::new(binary)
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

    let _close = session.close()?;
    let exit = app.wait(WAIT).context("Atelier to honor native close")?;
    ensure!(exit.success(), "Atelier close failed: {exit:#?}");
    app.terminate().context("collect Atelier cgroup")?;
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

    let clipboard = testbed.launch(
        AppCommand::new("/usr/bin/xclip")
            .args(["-selection", "clipboard", "-out"])
            .runtime(WAIT),
    )?;
    let exit = clipboard.wait(WAIT)?;
    ensure!(
        exit.success(),
        "xclip could not read the native clipboard: {exit:#?}"
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
