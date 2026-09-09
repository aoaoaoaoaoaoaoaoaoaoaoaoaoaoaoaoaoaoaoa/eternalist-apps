//! Desktop platform facts: the whole surface is safe, one frame of latency,
//! no system tracer, and a bare event loop.

use super::{SafeArea, Spark};
use crate::Ingress;
use anyhow::{Context as _, Result};
use winit::{
    event_loop::{ActiveEventLoop, EventLoop},
    window::Window,
};

/// Frames the swapchain may hold ahead of presentation.
pub const FRAME_LATENCY: u32 = 1;

/// Desktops report no system bars.
#[derive(Clone, Copy, Debug)]
pub struct Bars;

impl Bars {
    pub const fn unread() -> Self {
        Self
    }
}

/// A system-trace section; desktops trace through `tracing` alone.
pub struct Span;

impl Span {
    pub const fn begin(_name: &'static str) -> Self {
        Self
    }
}

pub fn temper_instance(_descriptor: &mut wgpu::InstanceDescriptor) {}

#[allow(
    clippy::needless_pass_by_value,
    reason = "the Android seam consumes the token; the desktop seam shares its signature"
)]
pub fn event_loop(ingress: Ingress) -> Result<EventLoop<Spark>> {
    let Ingress::Desktop = ingress;
    EventLoop::<Spark>::with_user_event()
        .build()
        .context("build event loop")
}

#[allow(
    clippy::unnecessary_wraps,
    reason = "the platform seam is fallible where insets cross into Java"
)]
pub fn safe_area(
    _ctx: &egui::Context,
    _window: &Window,
    _event_loop: &ActiveEventLoop,
    _bars: &mut Bars,
) -> Result<SafeArea> {
    Ok(SafeArea::default())
}
