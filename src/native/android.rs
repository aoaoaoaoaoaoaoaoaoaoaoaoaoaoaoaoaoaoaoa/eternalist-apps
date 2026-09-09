//! Android platform facts: the `NativeActivity` event loop, system-bar
//! insets read over JNI, a deeper swapchain for older Adreno consumers, and
//! system-trace sections.

use super::{SafeArea, Spark};
use crate::Ingress;
use anyhow::{Context as _, Result};
use jni::{JavaVM, refs::Global};
use winit::{
    event_loop::{ActiveEventLoop, EventLoop},
    platform::android::{
        ActiveEventLoopExtAndroid as _, EventLoopBuilderExtAndroid as _, WindowExtAndroid as _,
        activity::AndroidApp,
    },
    window::Window,
};

#[allow(
    elided_lifetimes_in_paths,
    reason = "jni binding macro generates lifetime-elided implementation paths"
)]
mod sdk {
    jni::bind_java_type! {
        pub(super) Activity => "android.app.Activity",
        type_map {
            AndroidWindow => "android.view.Window",
        },
        methods {
            fn get_window() -> AndroidWindow,
        }
    }

    jni::bind_java_type! {
        pub(super) AndroidWindow => "android.view.Window",
        type_map {
            AndroidView => "android.view.View",
        },
        methods {
            fn get_decor_view() -> AndroidView,
        }
    }

    jni::bind_java_type! {
        pub(super) AndroidView => "android.view.View",
        type_map {
            AndroidWindowInsets => "android.view.WindowInsets",
        },
        methods {
            fn get_root_window_insets() -> AndroidWindowInsets,
        }
    }

    jni::bind_java_type! {
        pub(super) AndroidWindowInsets => "android.view.WindowInsets",
        type_map {
            AndroidInsets => "android.graphics.Insets",
        },
        methods {
            fn get_insets(type_mask: i32) -> AndroidInsets,
            fn get_system_window_inset_left() -> i32,
            fn get_system_window_inset_right() -> i32,
            fn get_system_window_inset_top() -> i32,
            fn get_system_window_inset_bottom() -> i32,
        }
    }

    jni::bind_java_type! {
        pub(super) AndroidWindowInsetsType => "android.view.WindowInsets$Type",
        methods {
            static fn system_bars() -> i32,
        }
    }

    jni::bind_java_type! {
        pub(super) AndroidInsets => "android.graphics.Insets",
        fields {
            left: i32,
            right: i32,
            top: i32,
            bottom: i32,
        }
    }
}

use sdk::{Activity, AndroidWindowInsetsType};

/// wgpu maps two frames of latency to three swapchain images, keeping older
/// Adreno `BufferQueue` consumers from starving acquisition.
pub const FRAME_LATENCY: u32 = 2;

/// System-bar insets in physical pixels, once Android has reported them.
#[derive(Clone, Copy, Debug)]
pub struct Bars(Option<InsetsPx>);

impl Bars {
    pub const fn unread() -> Self {
        Self(None)
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct InsetsPx {
    left: i32,
    right: i32,
    top: i32,
    bottom: i32,
}

/// One RAII section in Android's system trace, near zero cost while tracing
/// is inactive.
pub struct Section(Option<ndk::trace::Section>);

impl Section {
    pub fn begin(name: &'static str) -> Self {
        let section = ndk::trace::is_trace_enabled()
            .then(|| ndk::trace::Section::new(name).ok())
            .flatten();
        Self(section)
    }
}

impl Drop for Section {
    fn drop(&mut self) {
        drop(self.0.take());
    }
}

/// Emulator Vulkan drivers may advertise debug-utils support but fault while
/// naming objects. Validation remains enabled independently.
pub fn temper_instance(descriptor: &mut wgpu::InstanceDescriptor) {
    descriptor.flags.remove(wgpu::InstanceFlags::DEBUG);
    descriptor
        .flags
        .insert(wgpu::InstanceFlags::DISCARD_HAL_LABELS);
}

pub fn event_loop(ingress: Ingress) -> Result<EventLoop<Spark>> {
    let Ingress::Android(android) = ingress else {
        anyhow::bail!("Android host entered without its NativeActivity");
    };
    let mut builder = EventLoop::<Spark>::with_user_event();
    let _builder = builder.with_android_app(android);
    builder.build().context("build Android event loop")
}

#[allow(
    clippy::cast_precision_loss,
    reason = "physical display insets are exactly representable throughout practical screen geometry"
)]
pub fn safe_area(
    ctx: &egui::Context,
    window: &Window,
    event_loop: &ActiveEventLoop,
    bars: &mut Bars,
) -> Result<SafeArea> {
    if bars.0.is_none() {
        bars.0 = system_bar_insets(event_loop.android_app())
            .context("read Android system-bar insets")?;
    }
    let content = window.content_rect();
    let surface = window.inner_size();
    let pixels_per_point = egui_winit::pixels_per_point(ctx, window);
    let content = if content.right > content.left && content.bottom > content.top {
        let width = i32::try_from(surface.width).unwrap_or(i32::MAX);
        let height = i32::try_from(surface.height).unwrap_or(i32::MAX);
        InsetsPx {
            left: content.left.max(0),
            right: width.saturating_sub(content.right).max(0),
            top: content.top.max(0),
            bottom: height.saturating_sub(content.bottom).max(0),
        }
    } else {
        InsetsPx::default()
    };
    let system = bars.0.unwrap_or_default();
    let margin = egui::epaint::MarginF32 {
        left: content.left.max(system.left) as f32 / pixels_per_point,
        right: content.right.max(system.right) as f32 / pixels_per_point,
        top: content.top.max(system.top) as f32 / pixels_per_point,
        bottom: content.bottom.max(system.bottom) as f32 / pixels_per_point,
    };
    Ok(SafeArea {
        insets: Some(egui::SafeAreaInsets(margin)),
        unsettled: bars.0.is_none(),
    })
}

#[allow(
    unsafe_code,
    reason = "Android exposes its VM and borrowed Activity only through raw JNI pointers"
)]
fn system_bar_insets(android: &AndroidApp) -> jni::errors::Result<Option<InsetsPx>> {
    // SAFETY: Android owns the process VM for the lifetime of `android`, and the
    // Activity pointer is an unowned global reference valid for that same lifetime.
    let vm = unsafe { JavaVM::from_raw(android.vm_as_ptr().cast()) };
    vm.attach_current_thread(|env| {
        let raw_activity = android.activity_as_ptr() as jni::sys::jobject;
        // SAFETY: The cast borrows Android's global Activity reference without
        // assuming ownership or extending it beyond this JNI call.
        let activity = unsafe { env.as_cast_raw::<Global<Activity<'_>>>(&raw_activity)? };
        let window = activity.as_ref().get_window(env)?;
        let decor = window.get_decor_view(env)?;
        let insets = decor.get_root_window_insets(env)?;
        if insets.is_null() {
            return Ok(None);
        }
        let system = if AndroidApp::sdk_version() >= 30 {
            let type_mask = AndroidWindowInsetsType::system_bars(env)?;
            let system = insets.get_insets(env, type_mask)?;
            InsetsPx {
                left: system.left(env)?,
                right: system.right(env)?,
                top: system.top(env)?,
                bottom: system.bottom(env)?,
            }
        } else {
            InsetsPx {
                left: insets.get_system_window_inset_left(env)?,
                right: insets.get_system_window_inset_right(env)?,
                top: insets.get_system_window_inset_top(env)?,
                bottom: insets.get_system_window_inset_bottom(env)?,
            }
        };
        Ok(Some(system))
    })
}
