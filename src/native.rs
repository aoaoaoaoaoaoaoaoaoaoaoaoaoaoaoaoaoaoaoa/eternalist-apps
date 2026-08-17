//! One-window native winit, egui, wgpu, water, and witness lifecycle.

use crate::{crash_reports::CrashReports, responsiveness};
use anyhow::{Context as _, Result, bail};
use brass_poolrooms::water::{Engine, Frame as WaterFrame};
use egui_wgpu::{
    RenderState, Renderer, RendererOptions, ScreenDescriptor, WgpuConfiguration, WgpuSetup,
};
#[cfg(feature = "egui-test")]
use serde::Serialize;
use std::{
    cell::Cell,
    fmt,
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicU8, Ordering},
    },
    time::{Duration, Instant},
};
#[cfg(target_os = "linux")]
use winit::platform::x11::{ActiveEventLoopExtX11 as _, WindowAttributesExtX11 as _, WindowType};
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, PhysicalSize},
    event::{StartCause, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy},
    window::{Window, WindowAttributes},
};

macro_rules! main_phase {
    ($name:literal, $body:expr) => {{
        let _phase = tracing::info_span!(target: "eternalist::main", $name).entered();
        $body
    }};
}

/// Stable top-level window identity and initial geometry.
#[derive(Clone, Copy, Debug)]
pub struct WindowSpec {
    /// Initial and fallback window title.
    pub title: &'static str,
    /// Initial logical width and height in points.
    pub initial_size: [f64; 2],
    /// Whether X11 window managers should treat the window as a floating utility.
    pub floating: bool,
}

/// Main-thread wall-time obligations owned by the native product.
#[derive(Clone, Copy, Debug)]
pub struct ResponsivenessSpec {
    /// Maximum ordinary main-thread work admitted by one frame.
    pub frame: Duration,
}

/// Product policy for a window-manager close request.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum CloseDisposition {
    /// End the native application.
    #[default]
    Exit,
    /// Keep the application resident when the native window can be hidden and
    /// later revealed; otherwise end the application.
    HideOrExit,
}

impl ResponsivenessSpec {
    /// Admit at most 40 ms of main-thread work per ordinary frame.
    #[must_use]
    pub const fn interactive() -> Self {
        Self {
            frame: Duration::from_millis(40),
        }
    }
}

impl WindowSpec {
    /// Define a non-floating window with its initial logical size in points.
    #[must_use]
    pub const fn new(title: &'static str, initial_size: [f64; 2]) -> Self {
        Self {
            title,
            initial_size,
            floating: false,
        }
    }

    /// Ask X11 window managers to treat this application as a floating utility.
    #[must_use]
    pub const fn floating(mut self) -> Self {
        self.floating = true;
        self
    }
}

/// The narrow product seam admitted by the native host.
pub trait NativeApp {
    /// Initial top-level window identity and geometry.
    const WINDOW: WindowSpec;
    /// Main-thread frame-work obligation checked by the host instrumentation.
    const RESPONSIVENESS: ResponsivenessSpec = ResponsivenessSpec::interactive();

    /// Opt this product into local crash recovery and explicit report consent.
    ///
    /// The host never transmits a report without a fresh user gesture. Storage
    /// placement remains a product decision because it is part of the
    /// product's filesystem contract.
    #[must_use]
    fn crash_reports() -> Option<crate::CrashReportSpec> {
        None
    }

    /// Current top-level window identity.
    fn window_title(&self) -> String {
        Self::WINDOW.title.to_owned()
    }

    /// Build one ordinary product UI frame on the native event-loop thread.
    fn draw(&mut self, ui: &mut egui::Ui);

    /// Decide what a window-manager close request means for this product.
    fn close_requested(&mut self) -> CloseDisposition {
        CloseDisposition::Exit
    }

    /// Consume an application-owned request to reveal a window previously hidden by
    /// [`CloseDisposition::HideOrExit`].
    ///
    /// External controllers must call [`NativeWake::wake`] after raising this signal
    /// so the sleeping native event loop observes it.
    fn take_reveal_request(&mut self) -> bool {
        false
    }

    /// Consume an application-owned request to hide the window after the frame
    /// carrying that request has been presented.
    ///
    /// The host publishes any acceptance witness before committing the
    /// concealment. Applications must use this seam instead of issuing an egui
    /// viewport visibility command during [`Self::draw`].
    fn take_conceal_request(&mut self) -> bool {
        false
    }

    /// Return the next application-service deadline, independent of rendering.
    ///
    /// Use this for semantic clocks such as retries, publication surveys, and
    /// persistence settlement. Visual motion belongs in egui or Poolrooms
    /// repaint requests instead. The host services this deadline even while the
    /// window is concealed, without creating a frame.
    fn service_deadline(&self, _now: Instant) -> Option<Instant> {
        None
    }

    /// Service a reached [`Self::service_deadline`] on the event-loop thread.
    ///
    /// This callback must remain small and nonblocking. Return `true` when the
    /// visible projection changed. Before returning, retire every deadline at
    /// or before `now`; retaining a reached deadline is a lifecycle violation.
    fn service_deadline_reached(&mut self, _now: Instant) -> bool {
        false
    }

    /// Report an explicit application exit requested outside the native window.
    fn exit_requested(&self) -> bool {
        false
    }

    /// Commit work deliberately deferred until a successful surface present.
    ///
    /// The host calls this exactly once after each successful present and never
    /// after an acquisition or rendering failure. Return `true` to request a
    /// follow-up frame.
    fn after_present(&mut self) -> bool;

    /// Seal the Poolrooms water composition for the frame just drawn.
    ///
    /// `pixels_per_point` is the physical-to-logical scale for this render.
    /// `tooltip_rects` contains final-pass logical rectangles that must remain
    /// optically above the water surface.
    fn water(
        &mut self,
        ctx: &egui::Context,
        pixels_per_point: f32,
        tooltip_rects: &[egui::Rect],
    ) -> WaterFrame;

    /// Install application-owned wgpu callback resources before the first frame.
    ///
    /// The host invokes this once after constructing its renderer. Registered
    /// resources must use the supplied device and target format.
    fn register_gpu(renderer: &mut Renderer, device: &wgpu::Device, format: wgpu::TextureFormat);

    /// Minimal one-way state projected to native acceptance stories.
    #[cfg(feature = "egui-test")]
    type Observation: Serialize + Send + 'static;

    /// Project the smallest useful one-way acceptance observation.
    #[cfg(feature = "egui-test")]
    fn observe(&self, text_edit_focused: bool) -> Self::Observation;
}

#[derive(Clone, Copy, Debug)]
enum Spark {
    Wake,
    RepaintAfter(Duration),
    ForegroundRepaintAfter(Duration),
}

/// A reliable cross-thread signal into the native event loop.
///
/// Obtain the handle while constructing an application with
/// [`Self::from_context`], then clone it into workers or platform callbacks.
/// [`Self::request_foreground_repaint`] publishes streaming visible changes,
/// [`Self::request_repaint`] publishes finite changes that warrant a background
/// frame, and [`Self::wake`] publishes nonvisual control signals such as reveal
/// or exit.
/// Unlike `egui::Context::request_repaint`, these methods are not coalesced
/// behind egui's internal outstanding-repaint state.
#[derive(Clone, Default)]
pub struct NativeWake {
    proxy: Arc<Mutex<Option<EventLoopProxy<Spark>>>>,
}

impl NativeWake {
    /// Return the one native wake handle associated with an egui context.
    #[must_use]
    pub fn from_context(ctx: &egui::Context) -> Self {
        ctx.data_mut(|data| {
            data.get_temp_mut_or_default::<Self>(egui::Id::new("eternalist-native-wake"))
                .clone()
        })
    }

    /// Wake the event loop without requesting a product frame.
    ///
    /// Returns `false` before the native host is armed or after it retires.
    #[must_use]
    pub fn wake(&self) -> bool {
        self.send(Spark::Wake)
    }

    /// Wake the event loop and request one externally caused product frame.
    ///
    /// Presentation policy still suppresses the frame while the window is
    /// concealed and admits at most one frame while it is unfocused.
    /// Returns `false` before the native host is armed or after it retires.
    #[must_use]
    pub fn request_repaint(&self) -> bool {
        self.request_repaint_after(Duration::ZERO)
    }

    /// Wake the event loop and request one externally caused product frame no
    /// later than `delay` from now.
    ///
    /// Multiple requests are collapsed by the host to their earliest deadline.
    /// Presentation policy may still suppress the frame.
    #[must_use]
    pub fn request_repaint_after(&self, delay: Duration) -> bool {
        self.send(Spark::RepaintAfter(delay))
    }

    /// Wake the event loop and request a frame only while the application is
    /// foreground-focused.
    ///
    /// Use this for streams of progress, tiles, thumbnails, or other visible
    /// worker results whose consumption may mint more work. Suppressing these
    /// frames while unfocused lets bounded channels apply backpressure and
    /// prevents a producer/result/repaint cycle from becoming background
    /// animation. Focus restoration supplies the catch-up frame.
    #[must_use]
    pub fn request_foreground_repaint(&self) -> bool {
        self.request_foreground_repaint_after(Duration::ZERO)
    }

    /// Wake the event loop and request a foreground-only frame no later than
    /// `delay` from now.
    ///
    /// Multiple requests are collapsed by the host to their earliest deadline.
    #[must_use]
    pub fn request_foreground_repaint_after(&self, delay: Duration) -> bool {
        self.send(Spark::ForegroundRepaintAfter(delay))
    }

    fn send(&self, spark: Spark) -> bool {
        lock_native_wake(self)
            .as_ref()
            .is_some_and(|proxy| proxy.send_event(spark).is_ok())
    }

    fn arm(&self, proxy: EventLoopProxy<Spark>) {
        *lock_native_wake(self) = Some(proxy);
    }

    fn disarm(&self) {
        *lock_native_wake(self) = None;
    }
}

impl fmt::Debug for NativeWake {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NativeWake")
            .field("armed", &lock_native_wake(self).is_some())
            .finish()
    }
}

fn lock_native_wake(wake: &NativeWake) -> MutexGuard<'_, Option<EventLoopProxy<Spark>>> {
    match wake.proxy.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

type Alarm = Arc<Mutex<Option<Instant>>>;

// A Context is shared with workers, so origin must be thread-local: a worker
// repaint concurrent with UI rendering is external, while egui's own callback
// on the event-loop thread is part of the frame that caused it.
thread_local! {
    static IN_FRAME: Cell<bool> = const { Cell::new(false) };
}

struct FrameScope;

impl FrameScope {
    fn enter() -> Self {
        let nested = IN_FRAME.replace(true);
        debug_assert!(!nested, "native frames must not nest");
        Self
    }
}

impl Drop for FrameScope {
    fn drop(&mut self) {
        IN_FRAME.set(false);
    }
}

#[derive(Clone, Copy, Debug)]
enum RepaintOrigin {
    External,
    ForegroundExternal,
    Frame,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
enum Presentation {
    Concealed,
    Background,
    Foreground,
}

#[derive(Clone, Copy, Debug)]
#[repr(u8)]
enum Concealment {
    Occluded = 1 << 0,
    ZeroSized = 1 << 1,
    Hidden = 1 << 2,
    Minimized = 1 << 3,
}

#[derive(Clone, Copy, Debug, Default)]
struct Concealments(u8);

impl Concealments {
    fn set(&mut self, cause: Concealment, active: bool) {
        if active {
            self.0 |= cause as u8;
        } else {
            self.0 &= !(cause as u8);
        }
    }

    const fn any(self) -> bool {
        self.0 != 0
    }
}

#[derive(Clone, Copy, Debug, Default)]
enum Focus {
    Background,
    #[default]
    Foreground,
}

#[derive(Clone, Copy, Debug, Default)]
struct WindowStanding {
    focus: Focus,
    concealments: Concealments,
}

impl WindowStanding {
    const fn presentation(self) -> Presentation {
        if self.concealments.any() {
            Presentation::Concealed
        } else {
            match self.focus {
                Focus::Background => Presentation::Background,
                Focus::Foreground => Presentation::Foreground,
            }
        }
    }
}

#[derive(Clone, Debug)]
struct RepaintGovernor(Arc<AtomicU8>);

impl RepaintGovernor {
    fn new() -> Self {
        Self(Arc::new(AtomicU8::new(Presentation::Foreground as u8)))
    }

    fn presentation(&self) -> Presentation {
        match self.0.load(Ordering::Acquire) {
            value if value == Presentation::Concealed as u8 => Presentation::Concealed,
            value if value == Presentation::Background as u8 => Presentation::Background,
            value if value == Presentation::Foreground as u8 => Presentation::Foreground,
            _ => unreachable!("repaint governor admitted an invalid presentation state"),
        }
    }

    fn set(&self, presentation: Presentation) -> Presentation {
        let prior = self.0.swap(presentation as u8, Ordering::AcqRel);
        match prior {
            value if value == Presentation::Concealed as u8 => Presentation::Concealed,
            value if value == Presentation::Background as u8 => Presentation::Background,
            value if value == Presentation::Foreground as u8 => Presentation::Foreground,
            _ => unreachable!("repaint governor admitted an invalid presentation state"),
        }
    }

    fn delay(&self, requested: Duration, origin: RepaintOrigin) -> Option<Duration> {
        match (self.presentation(), origin) {
            (Presentation::Concealed, _)
            | (
                Presentation::Background,
                RepaintOrigin::ForegroundExternal | RepaintOrigin::Frame,
            ) => None,
            (Presentation::Background, RepaintOrigin::External) | (Presentation::Foreground, _) => {
                Some(requested)
            }
        }
    }
}

/// Run one native application until its sole top-level window closes.
///
/// # Errors
///
/// Returns the first event-loop, window, GPU, rendering, tracing, or witness
/// failure. The host does not continue after a corrupt frame path.
pub fn run<A: NativeApp>(ctx: egui::Context, app: A) -> Result<()> {
    let crash_reports = CrashReports::arm(A::crash_reports(), &ctx);
    run_armed(ctx, app, crash_reports)
}

/// Construct and run one native application inside the recoverable panic boundary.
///
/// Prefer this entry point when application construction performs fallible
/// platform or storage work. The crash hook is armed before `build` runs.
///
/// # Errors
///
/// Returns the first application-construction, event-loop, window, GPU,
/// rendering, tracing, or witness failure.
pub fn run_with<A, F>(ctx: egui::Context, build: F) -> Result<()>
where
    A: NativeApp,
    F: FnOnce(&egui::Context) -> Result<A>,
{
    let crash_reports = CrashReports::arm(A::crash_reports(), &ctx);
    let app = build(&ctx)?;
    run_armed(ctx, app, crash_reports)
}

fn run_armed<A: NativeApp>(ctx: egui::Context, app: A, crash_reports: CrashReports) -> Result<()> {
    let event_loop = EventLoop::<Spark>::with_user_event()
        .build()
        .context("build event loop")?;
    event_loop.set_control_flow(ControlFlow::Wait);
    let alarm = Alarm::default();
    let governor = RepaintGovernor::new();
    let wake = NativeWake::from_context(&ctx);
    let proxy = event_loop.create_proxy();
    wake.arm(proxy.clone());
    arm_repaints(&ctx, Arc::clone(&alarm), governor.clone(), proxy);
    #[cfg(feature = "egui-test")]
    let witness: Option<egui_tester_witness::Publisher<A::Observation>> =
        egui_tester_witness::Publisher::from_env().context("arm egui-tester witness")?;
    #[cfg(feature = "egui-test")]
    if witness.is_some() {
        install_witness(&ctx);
    }
    let mut shell = Shell {
        ctx,
        app,
        alarm,
        governor,
        rig: None,
        force_redraw: false,
        surface_occlusion_retries: 0,
        window_standing: WindowStanding::default(),
        window_title: A::WINDOW.title.to_owned(),
        fault: None,
        crash_reports,
        trace_deadline: responsiveness::deadline()?,
        #[cfg(feature = "egui-test")]
        witness,
    };
    let event_result = event_loop.run_app(&mut shell).context("run event loop");
    wake.disarm();
    event_result?;
    #[cfg(feature = "egui-test")]
    if let Some(witness) = &shell.witness {
        witness.flush().context("flush egui-tester witness")?;
    }
    shell.fault.map_or(Ok(()), Err)
}

fn arm_repaints(
    ctx: &egui::Context,
    alarm: Alarm,
    governor: RepaintGovernor,
    proxy: EventLoopProxy<Spark>,
) {
    ctx.set_request_repaint_callback(move |info| {
        let origin = IN_FRAME.with(|in_frame| {
            if in_frame.get() {
                RepaintOrigin::Frame
            } else {
                RepaintOrigin::External
            }
        });
        let admitted = governor.delay(info.delay, origin);
        let _repaint = tracing::info_span!(
            target: "eternalist::main",
            "repaint.callback",
            ?origin,
            presentation = ?governor.presentation(),
            admitted = admitted.is_some(),
        )
        .entered();
        if let Some(delay) = admitted {
            advance_alarm(&alarm, Instant::now() + delay);
        }
        let _woken = proxy.send_event(Spark::Wake);
    });
}

fn schedule_repaint(
    governor: &RepaintGovernor,
    alarm: &Alarm,
    window: &Window,
    delay: Duration,
    origin: RepaintOrigin,
) {
    let admitted = governor.delay(delay, origin);
    let _repaint = tracing::info_span!(
        target: "eternalist::main",
        "repaint.schedule",
        ?origin,
        presentation = ?governor.presentation(),
        admitted = admitted.is_some(),
    )
    .entered();
    let Some(delay) = admitted else {
        return;
    };
    if delay.is_zero() {
        window.request_redraw();
    } else if let Some(when) = Instant::now().checked_add(delay) {
        advance_alarm(alarm, when);
    }
}

fn advance_alarm(alarm: &Alarm, when: Instant) {
    let mut alarm = lock_alarm(alarm);
    if alarm.is_none_or(|set| when < set) {
        *alarm = Some(when);
    }
}

fn lock_alarm(alarm: &Alarm) -> MutexGuard<'_, Option<Instant>> {
    match alarm.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

struct Shell<A: NativeApp> {
    ctx: egui::Context,
    app: A,
    alarm: Alarm,
    governor: RepaintGovernor,
    rig: Option<Rig>,
    force_redraw: bool,
    surface_occlusion_retries: usize,
    window_standing: WindowStanding,
    window_title: String,
    fault: Option<anyhow::Error>,
    crash_reports: CrashReports,
    trace_deadline: Option<Instant>,
    #[cfg(feature = "egui-test")]
    witness: Option<egui_tester_witness::Publisher<A::Observation>>,
}

impl<A: NativeApp> Shell<A> {
    #[allow(
        clippy::too_many_lines,
        reason = "the canonical native frame is one ordered presentation transaction"
    )]
    fn paint(&mut self, event_loop: &ActiveEventLoop) -> Result<()> {
        let presentation = self.governor.presentation();
        match presentation {
            Presentation::Concealed => return Ok(()),
            Presentation::Background | Presentation::Foreground => {}
        }
        let Some(rig) = self.rig.as_mut() else {
            return Ok(());
        };
        let begun = Instant::now();
        let frame = self.ctx.cumulative_frame_nr();
        let frame_span = tracing::info_span!(
            target: "eternalist::main",
            "frame",
            frame,
            primitives = tracing::field::Empty,
            pixels_per_point = tracing::field::Empty,
            presented = tracing::field::Empty,
            ?presentation,
        );
        let _frame = frame_span.enter();
        let _frame_scope = FrameScope::enter();
        #[cfg(feature = "egui-test")]
        let pulse = self
            .witness
            .as_ref()
            .map(|_| egui_tester_witness::FramePulse::begin());
        let raw_input = main_phase!("frame.input", rig.input.take_egui_input(&rig.window));
        let mut output = main_phase!(
            "frame.ui",
            self.ctx.run_ui(raw_input, |ui| {
                self.crash_reports.quarantine_input(ui.ctx());
                self.app.draw(ui);
                self.crash_reports.restore_input(ui.ctx());
                self.crash_reports.show(ui.ctx());
            })
        );
        frame_span.record("pixels_per_point", output.pixels_per_point);
        let title = self.app.window_title();
        if title != self.window_title {
            rig.window.set_title(&title);
            self.window_title = title;
        }
        main_phase!(
            "frame.platform_output",
            rig.handle_platform_output(event_loop, output.platform_output)?
        );
        if let Some(viewport) = output.viewport_output.get(&egui::ViewportId::ROOT) {
            main_phase!(
                "frame.viewport_commands",
                rig.process_viewport_commands(&self.ctx, viewport.commands.iter().cloned())
            );
        }
        let primitives = main_phase!(
            "frame.tessellate",
            self.ctx.tessellate(output.shapes, output.pixels_per_point)
        );
        frame_span.record("primitives", primitives.len());
        let tooltip_rects = main_phase!("frame.tooltip_geometry", tooltip_rects(&self.ctx));
        let water = main_phase!(
            "frame.water",
            self.app
                .water(&self.ctx, output.pixels_per_point, &tooltip_rects)
        );
        if water.wants_repaint() {
            schedule_repaint(
                &self.governor,
                &self.alarm,
                &rig.window,
                Duration::ZERO,
                RepaintOrigin::Frame,
            );
        }
        #[cfg(feature = "egui-test")]
        let pending = pulse
            .map(|pulse| {
                stage_witness(
                    &self.ctx,
                    pulse,
                    self.ctx.cumulative_frame_nr(),
                    output.pixels_per_point,
                    self.app.observe(self.ctx.text_edit_focused()),
                )
            })
            .transpose()
            .context("stage egui-tester witness")?;
        let rendered = main_phase!(
            "frame.render",
            rig.render(
                &primitives,
                &output.textures_delta,
                output.pixels_per_point,
                &water,
            )
        );
        // Egui's delta is a one-shot transaction. The renderer has now either
        // applied it or deliberately retired it on a surface failure.
        output.textures_delta.clear();
        let rendered = rendered?;
        let RenderOutcome::Presented { repaint } = rendered else {
            frame_span.record("presented", false);
            match rendered {
                RenderOutcome::Retry => {
                    schedule_repaint(
                        &self.governor,
                        &self.alarm,
                        &rig.window,
                        Duration::ZERO,
                        RepaintOrigin::Frame,
                    );
                }
                RenderOutcome::Occluded => {
                    *lock_alarm(&self.alarm) = None;
                    if let Some(delay) = SURFACE_OCCLUSION_RETRY_DELAYS
                        .get(self.surface_occlusion_retries)
                        .copied()
                    {
                        self.surface_occlusion_retries += 1;
                        schedule_repaint(
                            &self.governor,
                            &self.alarm,
                            &rig.window,
                            delay,
                            RepaintOrigin::Frame,
                        );
                    }
                }
                RenderOutcome::Presented { .. } => unreachable!(),
            }
            warn_frame_overrun(begun.elapsed(), A::RESPONSIVENESS.frame);
            return Ok(());
        };
        self.surface_occlusion_retries = 0;
        frame_span.record("presented", true);
        if repaint {
            schedule_repaint(
                &self.governor,
                &self.alarm,
                &rig.window,
                Duration::ZERO,
                RepaintOrigin::Frame,
            );
        }
        #[cfg(feature = "egui-test")]
        if let (Some(publisher), Some(pending)) = (&mut self.witness, pending) {
            let surface_presented = egui_tester_witness::ProductInstant::now();
            let _surface_sequence = publisher
                .surface_present_at(pending, surface_presented)
                .context("publish egui-tester witness")?;
        }
        self.force_redraw |= main_phase!("frame.after_present", self.app.after_present());
        if let Some(viewport) = output.viewport_output.get(&egui::ViewportId::ROOT) {
            schedule_repaint(
                &self.governor,
                &self.alarm,
                &rig.window,
                viewport.repaint_delay,
                RepaintOrigin::Frame,
            );
        }
        let conceal = main_phase!(
            "frame.take_conceal_request",
            self.app.take_conceal_request()
        );
        if conceal && !self.conceal() {
            bail!("application requested concealment on a platform without window visibility");
        }
        warn_frame_overrun(begun.elapsed(), A::RESPONSIVENESS.frame);
        Ok(())
    }

    fn tend_alarm(&self) {
        if self.governor.presentation() == Presentation::Concealed {
            return;
        }
        let Some(rig) = &self.rig else {
            return;
        };
        let fire = {
            let mut alarm = lock_alarm(&self.alarm);
            let fire = alarm.is_some_and(|when| when <= Instant::now());
            if fire {
                *alarm = None;
            }
            fire
        };
        if fire {
            rig.window.request_redraw();
        }
    }

    fn tend_service(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        if self
            .app
            .service_deadline(now)
            .is_none_or(|deadline| deadline > now)
        {
            return;
        }
        let changed = main_phase!(
            "app.service_deadline",
            self.app.service_deadline_reached(now)
        );
        let finished = Instant::now();
        if let Some(deadline) = self
            .app
            .service_deadline(finished)
            .filter(|deadline| *deadline <= finished)
        {
            self.abort(
                event_loop,
                anyhow::anyhow!(
                    "application retained a reached service deadline ({:?} overdue)",
                    finished.saturating_duration_since(deadline)
                ),
            );
            return;
        }
        if changed && let Some(rig) = &self.rig {
            schedule_repaint(
                &self.governor,
                &self.alarm,
                &rig.window,
                Duration::ZERO,
                RepaintOrigin::External,
            );
        }
    }

    fn reconcile_presentation(&mut self) {
        if let Some(rig) = &self.rig {
            self.window_standing.concealments.set(
                Concealment::Minimized,
                rig.window.is_minimized().unwrap_or(false),
            );
        }
        let presentation = self.window_standing.presentation();
        let prior = self.governor.set(presentation);
        if presentation == Presentation::Foreground && presentation != prior {
            self.surface_occlusion_retries = 0;
        }
        if presentation != prior
            && matches!(
                presentation,
                Presentation::Concealed | Presentation::Background
            )
        {
            *lock_alarm(&self.alarm) = None;
        }
        if presentation != prior
            && (prior == Presentation::Concealed || presentation == Presentation::Foreground)
            && presentation != Presentation::Concealed
            && let Some(rig) = &self.rig
        {
            rig.window.request_redraw();
        }
    }

    fn reveal(&mut self) {
        self.window_standing
            .concealments
            .set(Concealment::Hidden, false);
        self.window_standing
            .concealments
            .set(Concealment::Occluded, false);
        self.window_standing
            .concealments
            .set(Concealment::Minimized, false);
        if let Some(rig) = &self.rig {
            rig.window.set_visible(true);
            rig.window.set_minimized(false);
            rig.window.focus_window();
        }
        self.reconcile_presentation();
    }

    fn conceal(&mut self) -> bool {
        let can_hide = self
            .rig
            .as_ref()
            .is_some_and(|rig| rig.window.is_visible().is_some());
        if !can_hide {
            return false;
        }
        self.window_standing
            .concealments
            .set(Concealment::Hidden, true);
        self.reconcile_presentation();
        if let Some(rig) = &self.rig {
            rig.window.set_visible(false);
        }
        true
    }

    fn abort(&mut self, event_loop: &ActiveEventLoop, error: anyhow::Error) {
        if self.fault.is_none() {
            self.fault = Some(error);
        }
        event_loop.exit();
    }
}

fn warn_frame_overrun(elapsed: Duration, budget: Duration) {
    if elapsed <= budget {
        return;
    }
    let micros = |duration: Duration| u64::try_from(duration.as_micros()).unwrap_or(u64::MAX);
    tracing::warn!(
        target: "eternalist::latency",
        operation = "frame",
        elapsed_us = micros(elapsed),
        budget_us = micros(budget),
        "main-thread budget exceeded"
    );
}

fn tooltip_rects(ctx: &egui::Context) -> Vec<egui::Rect> {
    ctx.memory(|memory| {
        memory
            .layer_ids()
            .filter(|layer| layer.order == egui::Order::Tooltip && memory.areas().is_visible(layer))
            .filter_map(|layer| memory.area_rect(layer.id))
            .filter(egui::Rect::is_positive)
            .collect()
    })
}

impl<A: NativeApp> ApplicationHandler<Spark> for Shell<A> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.rig.is_some() {
            return;
        }
        match Rig::raise::<A>(event_loop, &self.ctx) {
            Ok(rig) => self.rig = Some(rig),
            Err(error) => self.abort(event_loop, error.context("raise native window")),
        }
    }

    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) {
        if matches!(cause, StartCause::ResumeTimeReached { .. }) {
            self.tend_alarm();
            self.tend_service(event_loop);
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: Spark) {
        if self.app.take_reveal_request() {
            self.reveal();
        } else {
            self.reconcile_presentation();
        }
        if let Some(rig) = &self.rig {
            match event {
                Spark::RepaintAfter(delay) => schedule_repaint(
                    &self.governor,
                    &self.alarm,
                    &rig.window,
                    delay,
                    RepaintOrigin::External,
                ),
                Spark::ForegroundRepaintAfter(delay) => schedule_repaint(
                    &self.governor,
                    &self.alarm,
                    &rig.window,
                    delay,
                    RepaintOrigin::ForegroundExternal,
                ),
                Spark::Wake => {}
            }
        }
        self.tend_alarm();
        self.tend_service(event_loop);
        if self.app.exit_requested() {
            event_loop.exit();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let event_name = match &event {
            WindowEvent::RedrawRequested => "redraw",
            WindowEvent::CursorMoved { .. } => "cursor_moved",
            WindowEvent::MouseWheel { .. } => "mouse_wheel",
            WindowEvent::MouseInput { .. } => "mouse_input",
            WindowEvent::KeyboardInput { .. } => "keyboard_input",
            WindowEvent::Resized(_) => "resized",
            WindowEvent::Focused(_) => "focused",
            WindowEvent::Occluded(_) => "occluded",
            WindowEvent::CloseRequested => "close_requested",
            _ => "other",
        };
        let active = match &event {
            WindowEvent::Focused(active) | WindowEvent::Occluded(active) => Some(*active),
            _ => None,
        };
        let _event = tracing::info_span!(
            target: "eternalist::main",
            "window.event",
            kind = event_name,
            active = ?active,
        )
        .entered();
        match &event {
            WindowEvent::CloseRequested => {
                match self.app.close_requested() {
                    CloseDisposition::Exit => event_loop.exit(),
                    CloseDisposition::HideOrExit => {
                        if !self.conceal() {
                            event_loop.exit();
                        }
                    }
                }
                return;
            }
            WindowEvent::RedrawRequested => {
                if let Err(error) = self.paint(event_loop) {
                    self.abort(event_loop, error);
                }
                return;
            }
            WindowEvent::Resized(size) => {
                self.surface_occlusion_retries = 0;
                self.window_standing
                    .concealments
                    .set(Concealment::ZeroSized, size.width == 0 || size.height == 0);
                self.reconcile_presentation();
                if let Some(rig) = &mut self.rig {
                    rig.resize(*size);
                }
            }
            WindowEvent::Focused(focused) => {
                self.window_standing.focus = if *focused {
                    Focus::Foreground
                } else {
                    Focus::Background
                };
                self.reconcile_presentation();
            }
            WindowEvent::Occluded(occluded) => {
                self.window_standing
                    .concealments
                    .set(Concealment::Occluded, *occluded);
                self.reconcile_presentation();
            }
            _ => {}
        }
        let Some(rig) = &mut self.rig else {
            return;
        };
        let response = rig.input.on_window_event(&rig.window, &event);
        if response.repaint {
            schedule_repaint(
                &self.governor,
                &self.alarm,
                &rig.window,
                Duration::ZERO,
                RepaintOrigin::External,
            );
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self
            .trace_deadline
            .is_some_and(|deadline| deadline <= Instant::now())
        {
            event_loop.exit();
            return;
        }
        if std::mem::take(&mut self.force_redraw)
            && let Some(rig) = &self.rig
        {
            schedule_repaint(
                &self.governor,
                &self.alarm,
                &rig.window,
                Duration::ZERO,
                RepaintOrigin::Frame,
            );
        }
        self.tend_alarm();
        self.tend_service(event_loop);
        let now = Instant::now();
        let deadline = (*lock_alarm(&self.alarm))
            .into_iter()
            .chain(self.app.service_deadline(now))
            .chain(self.trace_deadline)
            .min();
        event_loop.set_control_flow(deadline.map_or(ControlFlow::Wait, ControlFlow::WaitUntil));
    }
}

struct Rig {
    window: Arc<Window>,
    input: egui_winit::State,
    #[cfg(target_os = "linux")]
    cursor_foundry: crate::native_cursor::X11CursorFoundry,
    surface: wgpu::Surface<'static>,
    gpu: RenderState,
    config: wgpu::SurfaceConfiguration,
    water: Engine,
}

#[derive(Clone, Copy, Debug)]
enum RenderOutcome {
    Presented { repaint: bool },
    Retry,
    Occluded,
}

const SURFACE_OCCLUSION_RETRY_DELAYS: [Duration; 4] = [
    Duration::from_millis(16),
    Duration::from_millis(32),
    Duration::from_millis(64),
    Duration::from_millis(128),
];

impl Rig {
    fn handle_platform_output(
        &mut self,
        event_loop: &ActiveEventLoop,
        platform_output: egui::PlatformOutput,
    ) -> Result<()> {
        #[cfg(target_os = "linux")]
        {
            let cursor_image = platform_output.cursor_image.clone();
            if event_loop.is_x11() {
                self.input
                    .handle_platform_output(&self.window, platform_output);
            } else {
                self.input.handle_platform_output_with_event_loop(
                    &self.window,
                    event_loop,
                    platform_output,
                );
            }
            self.cursor_foundry
                .apply(cursor_image.as_ref())
                .context("apply X11 custom cursor")?;
        }
        #[cfg(not(target_os = "linux"))]
        self.input.handle_platform_output_with_event_loop(
            &self.window,
            event_loop,
            platform_output,
        );
        Ok(())
    }

    #[allow(
        clippy::cast_possible_truncation,
        reason = "winit reports DPI as f64 while egui's scale contract is f32"
    )]
    fn raise<A: NativeApp>(event_loop: &ActiveEventLoop, ctx: &egui::Context) -> Result<Self> {
        let [width, height] = A::WINDOW.initial_size;
        let attributes = WindowAttributes::default()
            .with_title(A::WINDOW.title)
            .with_inner_size(LogicalSize::new(width, height));
        #[cfg(target_os = "linux")]
        let attributes = if A::WINDOW.floating {
            attributes.with_x11_window_type(vec![WindowType::Dialog])
        } else {
            attributes
        };
        let window = Arc::new(
            event_loop
                .create_window(attributes)
                .context("create window")?,
        );
        let input = egui_winit::State::new(
            ctx.clone(),
            egui::ViewportId::ROOT,
            window.as_ref(),
            Some(window.scale_factor() as f32),
            window.theme(),
            None,
        );
        #[cfg(target_os = "linux")]
        let cursor_foundry = crate::native_cursor::X11CursorFoundry::bind(&window)
            .context("bind X11 cursor foundry")?;
        let mut configuration = WgpuConfiguration::default();
        if let WgpuSetup::CreateNew(setup) = &mut configuration.wgpu_setup {
            let inherited = Arc::clone(&setup.device_descriptor);
            setup.device_descriptor = Arc::new(move |adapter| {
                let mut descriptor = inherited(adapter);
                descriptor.memory_hints = wgpu::MemoryHints::MemoryUsage;
                descriptor
            });
        }
        let instance = pollster::block_on(configuration.wgpu_setup.new_instance());
        let surface = instance
            .create_surface(Arc::clone(&window))
            .context("create surface")?;
        let gpu = pollster::block_on(RenderState::create(
            &configuration,
            &instance,
            Some(&surface),
            RendererOptions::default(),
        ))
        .context("create wgpu render state")?;
        A::register_gpu(&mut gpu.renderer.write(), &gpu.device, gpu.target_format);
        let size = window.inner_size();
        let mut config = surface
            .get_default_config(&gpu.adapter, size.width.max(1), size.height.max(1))
            .context("surface is unsupported by the adapter")?;
        config.format = gpu.target_format;
        config.present_mode = wgpu::PresentMode::AutoVsync;
        config.desired_maximum_frame_latency = 1;
        config.view_formats = vec![gpu.target_format];
        surface.configure(&gpu.device, &config);
        let mut water = Engine::new(&gpu.device, gpu.target_format);
        water.resize(&gpu.device, config.width, config.height);
        Ok(Self {
            window,
            input,
            #[cfg(target_os = "linux")]
            cursor_foundry,
            surface,
            gpu,
            config,
            water,
        })
    }

    fn resize(&mut self, size: PhysicalSize<u32>) {
        if size.width == 0 || size.height == 0 {
            return;
        }
        self.config.width = size.width;
        self.config.height = size.height;
        self.surface.configure(&self.gpu.device, &self.config);
        self.water.resize(&self.gpu.device, size.width, size.height);
    }

    fn process_viewport_commands(
        &mut self,
        ctx: &egui::Context,
        commands: impl IntoIterator<Item = egui::ViewportCommand>,
    ) {
        let viewport = self
            .input
            .egui_input_mut()
            .viewports
            .entry(egui::ViewportId::ROOT)
            .or_default();
        let mut actions = Vec::new();
        egui_winit::process_viewport_commands(ctx, viewport, commands, &self.window, &mut actions);
    }

    #[allow(
        clippy::too_many_lines,
        reason = "the canonical Poolrooms render graph is one ordered GPU transaction"
    )]
    fn render(
        &mut self,
        primitives: &[egui::ClippedPrimitive],
        delta: &egui::TexturesDelta,
        pixels_per_point: f32,
        water: &WaterFrame,
    ) -> Result<RenderOutcome> {
        let screen = ScreenDescriptor {
            size_in_pixels: [self.config.width, self.config.height],
            pixels_per_point,
        };
        let mut encoder = main_phase!(
            "render.encoder",
            self.gpu
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("native-app-shell"),
                })
        );
        let user_commands = main_phase!("render.prepare", {
            let mut renderer = self.gpu.renderer.write();
            for (id, image_deltas) in &delta.set {
                for image_delta in image_deltas {
                    renderer.update_texture(&self.gpu.device, &self.gpu.queue, *id, image_delta);
                }
            }
            renderer.update_buffers(
                &self.gpu.device,
                &self.gpu.queue,
                &mut encoder,
                primitives,
                &screen,
            )
        });
        let frame = main_phase!(
            "render.acquire_surface",
            match self.surface.get_current_texture() {
                wgpu::CurrentSurfaceTexture::Success(frame)
                | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
                wgpu::CurrentSurfaceTexture::Timeout => {
                    self.free_textures(delta);
                    return Ok(RenderOutcome::Retry);
                }
                wgpu::CurrentSurfaceTexture::Occluded => {
                    self.free_textures(delta);
                    return Ok(RenderOutcome::Occluded);
                }
                wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                    self.free_textures(delta);
                    self.surface.configure(&self.gpu.device, &self.config);
                    return Ok(RenderOutcome::Retry);
                }
                wgpu::CurrentSurfaceTexture::Validation => {
                    self.free_textures(delta);
                    bail!("surface texture validation failure");
                }
            }
        );
        let surface_view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        if water.dry() {
            self.water.becalm(&self.gpu.queue);
        }
        let frosted = water.live() && self.water.scene_view().is_some();
        main_phase!("render.egui_pass", {
            let target = if frosted {
                self.water.scene_view().unwrap_or(&surface_view)
            } else {
                &surface_view
            };
            let mut pass = encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("native-app-egui"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: target,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                })
                .forget_lifetime();
            self.gpu
                .renderer
                .read()
                .render(&mut pass, primitives, &screen);
        });
        if frosted {
            main_phase!(
                "render.water_compose",
                self.water.compose(
                    &self.gpu.device,
                    &self.gpu.queue,
                    &mut encoder,
                    &surface_view,
                    water,
                )
            );
        }
        let _submission = main_phase!(
            "render.submit",
            self.gpu
                .queue
                .submit(user_commands.into_iter().chain([encoder.finish()]))
        );
        let repaint = main_phase!(
            "render.water_after_submit",
            self.water
                .after_submit(&self.gpu.device, &self.gpu.queue, water)
        );
        main_phase!("render.free_textures", self.free_textures(delta));
        self.window.pre_present_notify();
        main_phase!("render.present", self.gpu.queue.present(frame));
        let _maintained = main_phase!(
            "render.maintain",
            self.gpu.device.poll(wgpu::PollType::Poll)
        );
        Ok(RenderOutcome::Presented { repaint })
    }

    fn free_textures(&self, delta: &egui::TexturesDelta) {
        let mut renderer = self.gpu.renderer.write();
        for id in &delta.free {
            renderer.free_texture(id);
        }
    }
}

#[cfg(feature = "egui-test")]
fn install_witness(ctx: &egui::Context) {
    egui_tester_witness::egui::install(ctx);
    ctx.on_begin_pass(
        "clear poolrooms witness anchors",
        Arc::new(|ui| {
            drop(brass_poolrooms::instrumentation::take(ui.ctx()));
        }),
    );
}

#[cfg(feature = "egui-test")]
fn stage_witness<T: Serialize>(
    ctx: &egui::Context,
    pulse: egui_tester_witness::FramePulse,
    frame: u64,
    pixels_per_point: f32,
    state: T,
) -> egui_tester_witness::Result<egui_tester_witness::PendingFrame<T>> {
    use egui_tester_witness::Anchor;

    let anchors = egui_tester_witness::egui::take(ctx, pixels_per_point)?;
    let poolrooms = brass_poolrooms::instrumentation::take(ctx)
        .into_iter()
        .map(|anchor| {
            Anchor::logical(
                anchor.name,
                [
                    anchor.rect.min.x,
                    anchor.rect.min.y,
                    anchor.rect.max.x,
                    anchor.rect.max.y,
                ],
                pixels_per_point,
            )
        })
        .collect::<egui_tester_witness::Result<Vec<_>>>()?;
    let observed = pulse.observe();
    egui_tester_witness::PendingFrame::forge_at(
        observed,
        frame,
        pixels_per_point,
        anchors.into_iter().chain(poolrooms),
        state,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repaint_authority_distinguishes_control_results_from_streams() {
        let governor = RepaintGovernor::new();
        let delay = Duration::from_millis(7);
        for origin in [
            RepaintOrigin::External,
            RepaintOrigin::ForegroundExternal,
            RepaintOrigin::Frame,
        ] {
            assert_eq!(governor.delay(delay, origin), Some(delay));
        }

        let _prior = governor.set(Presentation::Background);
        assert_eq!(governor.delay(delay, RepaintOrigin::External), Some(delay));
        assert_eq!(
            governor.delay(delay, RepaintOrigin::ForegroundExternal),
            None
        );
        assert_eq!(governor.delay(delay, RepaintOrigin::Frame), None);

        let _prior = governor.set(Presentation::Concealed);
        for origin in [
            RepaintOrigin::External,
            RepaintOrigin::ForegroundExternal,
            RepaintOrigin::Frame,
        ] {
            assert_eq!(governor.delay(delay, origin), None);
        }
    }
}
