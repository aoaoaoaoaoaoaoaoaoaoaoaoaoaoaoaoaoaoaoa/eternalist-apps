//! Native capsule-filesystem and TLS acceptance without GUI automation.

#![expect(
    unused_crate_dependencies,
    reason = "this acceptance target shares the package manifest with the native host"
)]

#[cfg(all(
    feature = "egui-test",
    any(target_os = "linux", target_os = "macos", target_os = "windows")
))]
fn main() -> anyhow::Result<()> {
    let mut arguments = std::env::args().skip(1);
    let endpoint = arguments
        .next()
        .ok_or_else(|| anyhow::anyhow!("usage: crash_native_acceptance INTAKE_URL"))?;
    anyhow::ensure!(
        arguments.next().is_none(),
        "usage: crash_native_acceptance INTAKE_URL"
    );
    eternalist_apps::native_crash_acceptance(&endpoint).map_err(anyhow::Error::msg)
}

#[cfg(not(all(
    feature = "egui-test",
    any(target_os = "linux", target_os = "macos", target_os = "windows")
)))]
fn main() {}
