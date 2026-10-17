//! Hermetic native acceptance: an image whose PNG outgrows one X11 request
//! survives a paste byte for byte.

#![expect(
    unused_crate_dependencies,
    reason = "this acceptance target shares the package manifest with the library"
)]

#[cfg(all(target_os = "linux", feature = "egui-test"))]
use std::{
    thread,
    time::{Duration, Instant},
};

#[cfg(all(target_os = "linux", feature = "egui-test"))]
use anyhow::{Context as _, Result, bail, ensure};
#[cfg(all(target_os = "linux", feature = "egui-test"))]
use egui_tester::{AppCommand, Testbed};
#[cfg(all(target_os = "linux", feature = "egui-test"))]
use eternalist_apps::{
    NativeWake,
    clipboard::{RgbaImage, Seizure},
};

/// Incompressible noise: its PNG outgrows a 16 MiB BIG-REQUESTS request, so
/// only an INCR transfer can deliver it.
#[cfg(all(target_os = "linux", feature = "egui-test"))]
const SIDE: usize = 2400;
#[cfg(all(target_os = "linux", feature = "egui-test"))]
const SEIZE: &str = "--seize";
#[cfg(all(target_os = "linux", feature = "egui-test"))]
const PASTE: &str = "--paste";
#[cfg(all(target_os = "linux", feature = "egui-test"))]
const WAIT: Duration = Duration::from_secs(30);

#[cfg(all(target_os = "linux", feature = "egui-test"))]
fn main() -> Result<()> {
    let mut arguments = std::env::args().skip(1);
    match (arguments.next().as_deref(), arguments.next()) {
        (None, None) => witness(),
        (Some(SEIZE), None) => seize(),
        (Some(PASTE), None) => paste(),
        _ => bail!("usage: clipboard_acceptance"),
    }
}

#[cfg(not(all(target_os = "linux", feature = "egui-test")))]
fn main() {}

#[cfg(all(target_os = "linux", feature = "egui-test"))]
fn noise() -> Vec<u8> {
    let mut state = 0x9E37_79B9_7F4A_7C15_u64;
    (0..SIDE * SIDE * 4)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state.to_le_bytes()[7]
        })
        .collect()
}

#[cfg(all(target_os = "linux", feature = "egui-test"))]
fn witness() -> Result<()> {
    let executable = std::env::current_exe().context("locate the acceptance executable")?;
    let testbed = Testbed::raise().context("raise hermetic X11 testbed")?;
    let owner = testbed
        .launch(AppCommand::new(&executable).arg(SEIZE))
        .context("launch the clipboard owner")?;
    let paster = testbed
        .launch(AppCommand::new(&executable).arg(PASTE))
        .context("launch the paste")?;
    let exit = paster.wait(WAIT).context("paste the oversized image")?;
    ensure!(exit.success(), "oversized image paste failed: {exit:#?}");
    paster.terminate().context("collect the paste")?;
    owner.terminate().context("collect the clipboard owner")?;
    Ok(())
}

#[cfg(all(target_os = "linux", feature = "egui-test"))]
fn seize() -> Result<()> {
    let image = RgbaImage::new(SIDE, SIDE, noise())?;
    let mut seizure = Seizure::image(image, NativeWake::default());
    let seized = loop {
        match seizure.try_settle() {
            Ok(seized) => break seized,
            Err(pending) => {
                seizure = pending;
                thread::sleep(Duration::from_millis(10));
            }
        }
    };
    seized.context("seize the clipboard")?;
    loop {
        thread::park();
    }
}

#[cfg(all(target_os = "linux", feature = "egui-test"))]
fn paste() -> Result<()> {
    let expected = noise();
    let deadline = Instant::now() + WAIT;
    let mut clipboard = arboard::Clipboard::new().context("open the clipboard")?;
    loop {
        match clipboard.get_image() {
            Ok(image) => {
                ensure!(
                    (image.width, image.height) == (SIDE, SIDE),
                    "pasted {}×{} instead of {SIDE}×{SIDE}",
                    image.width,
                    image.height
                );
                ensure!(
                    *image.bytes == *expected,
                    "pasted pixels differ from the copied image"
                );
                return Ok(());
            }
            Err(_) if Instant::now() < deadline => thread::sleep(Duration::from_millis(50)),
            Err(error) => return Err(error).context("paste the image"),
        }
    }
}
