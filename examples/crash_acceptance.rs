//! Black-box crash, restart, consent, network, and stored-object acceptance.

#![expect(
    unused_crate_dependencies,
    reason = "this acceptance target shares the package manifest with its native specimen"
)]

#[cfg(all(target_os = "linux", feature = "egui-test"))]
use std::{
    collections::BTreeSet,
    path::PathBuf,
    process::Command,
    thread,
    time::{Duration, Instant},
};

#[cfg(all(target_os = "linux", feature = "egui-test"))]
use anyhow::{Context as _, Result, bail, ensure};
#[cfg(all(target_os = "linux", feature = "egui-test"))]
use egui_tester::{AppCommand, Button, Network, Probe, Testbed, WindowQuery};

#[cfg(all(target_os = "linux", feature = "egui-test"))]
const WAIT: Duration = Duration::from_secs(30);
#[cfg(all(target_os = "linux", feature = "egui-test"))]
const STATE: &str = "crash-state";
#[cfg(all(target_os = "linux", feature = "egui-test"))]
const CAPSULE: &str = "crash-state/crash-report-v1.json";

#[cfg(all(target_os = "linux", feature = "egui-test"))]
fn main() -> Result<()> {
    let mut arguments = std::env::args_os().skip(1);
    let binary = arguments
        .next()
        .map(PathBuf::from)
        .context("usage: crash_acceptance SPECIMEN INTAKE_URL REPORTS_BUCKET")?;
    let endpoint = arguments
        .next()
        .context("missing intake URL")?
        .into_string()
        .map_err(|_| anyhow::anyhow!("intake URL is not UTF-8"))?;
    let bucket = arguments
        .next()
        .context("missing reports bucket")?
        .into_string()
        .map_err(|_| anyhow::anyhow!("reports bucket is not UTF-8"))?;
    ensure!(
        arguments.next().is_none(),
        "usage: crash_acceptance SPECIMEN INTAKE_URL REPORTS_BUCKET"
    );
    ensure!(binary.is_file(), "specimen not found: {}", binary.display());

    let before = report_keys(&bucket)?;
    let testbed = Testbed::raise().context("raise hermetic X11 testbed")?;
    let crash = testbed.launch(specimen_command(&binary, &endpoint).arg("--detonate"))?;
    let exit = crash.wait(WAIT).context("specimen to detonate")?;
    ensure!(!exit.success(), "deliberate crash unexpectedly succeeded");
    crash
        .terminate()
        .context("collect crashed specimen cgroup")?;
    let capsule = testbed.read_private(CAPSULE)?;
    let captured: serde_json::Value = serde_json::from_slice(&capsule)?;
    ensure!(
        captured["fault"]["kind"] == "panic",
        "capsule did not record a panic"
    );

    let app = testbed.launch(
        specimen_command(&binary, &endpoint)
            .witness("probes/crash.observations")
            .network(Network::Host),
    )?;
    let session = testbed.x11_session(
        &app,
        WindowQuery::title_exact("Eternalist · crash-path specimen"),
        WAIT,
    )?;
    session.focus()?;
    let mut probe: Probe<bool> = app.witness()?.typed();
    let _presented = probe.wait_surface_presented(&app, WAIT)?;
    let send = probe.wait_anchor(&app, "eternalist.crash-report.send", WAIT)?;
    let (x, y) = send.center();
    let _click = session.click(x, y, Button::Primary)?;
    let _clicked = probe.wait_anchor(
        &app,
        "eternalist.crash-report.send-clicked",
        Duration::from_secs(5),
    )?;
    if let Err(error) = app.wait_until(WAIT, "accepted report capsule to retire", || {
        Ok(!testbed.private_path(CAPSULE)?.exists())
    }) {
        let stderr = std::fs::read_to_string(app.stderr_path())
            .unwrap_or_else(|read| format!("<could not read application stderr: {read}>"));
        bail!("{error}; application stderr:\n{stderr}");
    }

    let key = wait_for_report(&app, &bucket, &before)?;
    let stored = read_report(&bucket, &key)?;
    ensure!(
        stored == captured,
        "stored report differs from the consented capsule"
    );
    delete_report(&bucket, &key)?;

    let _close = session.close()?;
    let exit = app.wait(WAIT).context("specimen to close")?;
    ensure!(exit.success(), "restarted specimen failed: {exit:#?}");
    app.terminate()
        .context("collect restarted specimen cgroup")?;
    Ok(())
}

#[cfg(all(target_os = "linux", feature = "egui-test"))]
fn specimen_command(binary: &PathBuf, endpoint: &str) -> AppCommand {
    AppCommand::new(binary)
        .private_env("ETERNALIST_CRASH_STATE", STATE)
        .env("ETERNALIST_CRASH_INTAKE", endpoint)
        .runtime(Duration::from_mins(1))
}

#[cfg(all(target_os = "linux", feature = "egui-test"))]
fn wait_for_report(
    app: &egui_tester::Application<'_>,
    bucket: &str,
    before: &BTreeSet<String>,
) -> Result<String> {
    let deadline = Instant::now() + WAIT;
    loop {
        app.ensure_running("stored crash report")?;
        let new: Vec<_> = report_keys(bucket)?.difference(before).cloned().collect();
        if let [key] = new.as_slice() {
            return Ok(key.clone());
        }
        ensure!(
            new.is_empty(),
            "acceptance created multiple reports: {new:#?}"
        );
        ensure!(
            Instant::now() < deadline,
            "timed out waiting for stored crash report"
        );
        thread::sleep(Duration::from_millis(250));
    }
}

#[cfg(all(target_os = "linux", feature = "egui-test"))]
fn report_keys(bucket: &str) -> Result<BTreeSet<String>> {
    let output = Command::new("aws")
        .args([
            "s3api",
            "list-objects-v2",
            "--bucket",
            bucket,
            "--prefix",
            "reports/v1/",
            "--query",
            "Contents[].Key",
            "--output",
            "text",
        ])
        .output()
        .context("list stored reports")?;
    ensure!(
        output.status.success(),
        "list reports failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(String::from_utf8(output.stdout)?
        .split_whitespace()
        .map(str::to_owned)
        .collect())
}

#[cfg(all(target_os = "linux", feature = "egui-test"))]
fn read_report(bucket: &str, key: &str) -> Result<serde_json::Value> {
    let output = Command::new("aws")
        .args(["s3", "cp", &format!("s3://{bucket}/{key}"), "-"])
        .output()
        .context("read stored report")?;
    ensure!(
        output.status.success(),
        "read report failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).context("parse stored report")
}

#[cfg(all(target_os = "linux", feature = "egui-test"))]
fn delete_report(bucket: &str, key: &str) -> Result<()> {
    let status = Command::new("aws")
        .args(["s3api", "delete-object", "--bucket", bucket, "--key", key])
        .status()
        .context("delete acceptance report")?;
    ensure!(status.success(), "delete acceptance report failed");
    Ok(())
}

#[cfg(not(all(target_os = "linux", feature = "egui-test")))]
fn main() {}
