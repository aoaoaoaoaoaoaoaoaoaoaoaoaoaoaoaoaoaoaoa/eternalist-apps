use super::Exhibit;
use std::sync::{Arc, Mutex, MutexGuard};

use anyhow::{Context as _, Result};
use dwemer_poolrooms::egui_wgpu::WgpuSetup;
use dwemer_poolrooms::{
    chrome, egui,
    egui_wgpu::{RenderState, RendererOptions, ScreenDescriptor, WgpuConfiguration},
    water::{Domain, Engine, Floor, Surface, Wetness},
};
use web_time::Instant;
use winit::platform::web::EventLoopExtWebSys as _;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{StartCause, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy},
    window::{Window, WindowAttributes, WindowId},
};

enum Spark {
    Repaint,
    Forged(std::result::Result<Box<Rig>, String>),
}

type Alarm = Arc<Mutex<Option<Instant>>>;

pub fn run(app: impl Exhibit + 'static) -> Result<()> {
    console_error_panic_hook::set_once();
    let ctx = egui::Context::default();
    chrome::install(&ctx);
    let event_loop = EventLoop::<Spark>::with_user_event()
        .build()
        .context("build atelier event loop")?;
    event_loop.set_control_flow(ControlFlow::Wait);
    let alarm = Alarm::default();
    let proxy = event_loop.create_proxy();
    arm_repaints(&ctx, Arc::clone(&alarm), proxy.clone());
    let atelier = Atelier {
        ctx,
        exhibit: app,
        water: Surface::new(Wetness::Wet),
        alarm,
        rig: None,
        proxy,
        forging: false,
        proven: false,
    };
    event_loop.spawn_app(atelier);
    Ok(())
}

fn arm_repaints(ctx: &egui::Context, alarm: Alarm, proxy: EventLoopProxy<Spark>) {
    ctx.set_request_repaint_callback(move |info| {
        advance_alarm(&alarm, Instant::now() + info.delay);
        let _woken = proxy.send_event(Spark::Repaint);
    });
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

struct Atelier<A> {
    ctx: egui::Context,
    exhibit: A,
    water: Surface,
    alarm: Alarm,
    rig: Option<Rig>,
    proxy: EventLoopProxy<Spark>,
    forging: bool,
    proven: bool,
}

impl<A: Exhibit> Atelier<A> {
    fn paint(&mut self) {
        let Some(rig) = self.rig.as_mut() else {
            return;
        };
        let raw_input = rig.input.take_egui_input(&rig.window);
        let output = self.ctx.run_ui(raw_input, |ui| {
            let basin = ui.max_rect();
            self.water.begin(Domain::basin(basin));
            self.water.set_floor(Some(Floor::shallow(basin)));
            self.exhibit.ui(ui, &mut self.water);
        });
        rig.input
            .handle_platform_output(&rig.window, output.platform_output);
        let primitives = self.ctx.tessellate(output.shapes, output.pixels_per_point);
        let water = self
            .water
            .frame(&self.ctx, output.pixels_per_point, &[], None);
        if water.wants_repaint() {
            rig.window.request_redraw();
        }
        let presented = rig.render(
            &primitives,
            &output.textures_delta,
            output.pixels_per_point,
            &water,
        );
        if presented && !self.proven {
            self.proven = true;
            signal("ready", A::READY_MESSAGE);
        }
        if let Some(viewport) = output.viewport_output.get(&egui::ViewportId::ROOT) {
            if viewport.repaint_delay.is_zero() {
                rig.window.request_redraw();
            } else if let Some(when) = Instant::now().checked_add(viewport.repaint_delay) {
                advance_alarm(&self.alarm, when);
            }
        }
    }

    fn tend_alarm(&self) {
        let Some(rig) = &self.rig else {
            return;
        };
        let mut alarm = lock_alarm(&self.alarm);
        if alarm.is_some_and(|when| when <= Instant::now()) {
            *alarm = None;
            rig.window.request_redraw();
        }
    }
}

impl<A: Exhibit> ApplicationHandler<Spark> for Atelier<A> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.rig.is_some() || self.forging {
            return;
        }
        match Cradle::new::<A>(event_loop, &self.ctx) {
            Ok(cradle) => {
                self.forging = true;
                let proxy = self.proxy.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    let result = cradle
                        .forge()
                        .await
                        .map(Box::new)
                        .map_err(|err| format!("{err:#}"));
                    let _sent = proxy.send_event(Spark::Forged(result));
                });
            }
            Err(err) => {
                self.forging = true;
                signal("failed", &format!("Could not create canvas: {err:#}"));
            }
        }
    }

    fn new_events(&mut self, _event_loop: &ActiveEventLoop, cause: StartCause) {
        if matches!(cause, StartCause::ResumeTimeReached { .. }) {
            self.tend_alarm();
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: Spark) {
        match event {
            Spark::Repaint => self.tend_alarm(),
            Spark::Forged(result) => {
                self.forging = false;
                match result {
                    Ok(rig) => {
                        rig.window.request_redraw();
                        self.rig = Some(*rig);
                    }
                    Err(err) => {
                        self.forging = true;
                        signal("failed", &format!("Could not start WebGPU: {err}"));
                    }
                }
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match &event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
                return;
            }
            WindowEvent::RedrawRequested => {
                self.paint();
                return;
            }
            WindowEvent::Resized(size) => {
                if let Some(rig) = &mut self.rig {
                    rig.resize(*size);
                }
            }
            _ => {}
        }
        let Some(rig) = &mut self.rig else {
            return;
        };
        let response = rig.input.on_window_event(&rig.window, &event);
        if response.repaint {
            rig.window.request_redraw();
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.tend_alarm();
        event_loop.set_control_flow(match *lock_alarm(&self.alarm) {
            Some(when) => ControlFlow::WaitUntil(when),
            None => ControlFlow::Wait,
        });
    }
}

struct Rig {
    window: Arc<Window>,
    input: egui_winit::State,
    surface: wgpu::Surface<'static>,
    gpu: RenderState,
    config: wgpu::SurfaceConfiguration,
    water: Engine,
}

struct Cradle {
    window: Arc<Window>,
    input: egui_winit::State,
}

impl Cradle {
    fn new<A: Exhibit>(event_loop: &ActiveEventLoop, ctx: &egui::Context) -> Result<Self> {
        let attributes = WindowAttributes::default().with_title(A::TITLE);
        let attributes = {
            use wasm_bindgen::JsCast as _;
            use winit::platform::web::WindowAttributesExtWebSys as _;

            let canvas = web_sys::window()
                .and_then(|window| window.document())
                .and_then(|document| document.get_element_by_id(A::CANVAS_ID))
                .and_then(|element| element.dyn_into::<web_sys::HtmlCanvasElement>().ok())
                .with_context(|| format!("missing #{} canvas", A::CANVAS_ID))?;
            attributes
                .with_canvas(Some(canvas))
                .with_focusable(true)
                .with_prevent_default(true)
        };
        let window = Arc::new(
            event_loop
                .create_window(attributes)
                .context("create atelier window")?,
        );
        #[expect(
            clippy::cast_possible_truncation,
            reason = "egui records the platform's finite display scale as f32"
        )]
        let pixels_per_point = window.scale_factor() as f32;
        let input = egui_winit::State::new(
            ctx.clone(),
            egui::ViewportId::ROOT,
            window.as_ref(),
            Some(pixels_per_point),
            window.theme(),
            None,
        );
        Ok(Self { window, input })
    }

    async fn forge(self) -> Result<Rig> {
        let Self { window, mut input } = self;
        let configuration = {
            let mut configuration = WgpuConfiguration::default();
            if let WgpuSetup::CreateNew(setup) = &mut configuration.wgpu_setup {
                setup.instance_descriptor.backends = wgpu::Backends::BROWSER_WEBGPU;
            }
            configuration
        };
        let instance = configuration.wgpu_setup.new_instance().await;
        let surface = instance
            .create_surface(Arc::clone(&window))
            .context("create atelier surface")?;
        let gpu = RenderState::create(
            &configuration,
            &instance,
            Some(&surface),
            RendererOptions::default(),
        )
        .await
        .context("create atelier wgpu state")?;
        input.set_max_texture_side(gpu.device.limits().max_texture_dimension_2d as usize);
        let size = window.inner_size();
        let mut config = surface
            .get_default_config(&gpu.adapter, size.width.max(1), size.height.max(1))
            .context("atelier surface unsupported by adapter")?;
        config.format = gpu.target_format;
        config.present_mode = wgpu::PresentMode::AutoVsync;
        config.view_formats = vec![gpu.target_format];
        surface.configure(&gpu.device, &config);
        let mut water = Engine::new(&gpu.device, gpu.target_format);
        water.resize(&gpu.device, config.width, config.height);
        Ok(Rig {
            window,
            input,
            surface,
            gpu,
            config,
            water,
        })
    }
}

impl Rig {
    fn resize(&mut self, size: PhysicalSize<u32>) {
        if size.width == 0 || size.height == 0 {
            return;
        }
        self.config.width = size.width;
        self.config.height = size.height;
        self.surface.configure(&self.gpu.device, &self.config);
        self.water.resize(&self.gpu.device, size.width, size.height);
    }

    fn render(
        &mut self,
        primitives: &[egui::ClippedPrimitive],
        delta: &egui::TexturesDelta,
        pixels_per_point: f32,
        water: &dwemer_poolrooms::water::Frame,
    ) -> bool {
        let screen = ScreenDescriptor {
            size_in_pixels: [self.config.width, self.config.height],
            pixels_per_point,
        };
        let mut encoder = self
            .gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("eternalist-atelier"),
            });
        let user_commands = {
            let mut renderer = self.gpu.renderer.write();
            for (id, image_delta) in &delta.set {
                renderer.update_texture(&self.gpu.device, &self.gpu.queue, *id, image_delta);
            }
            renderer.update_buffers(
                &self.gpu.device,
                &self.gpu.queue,
                &mut encoder,
                primitives,
                &screen,
            )
        };
        let Some(frame) = self.acquire_frame() else {
            return false;
        };
        let surface_view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        if water.dry() {
            self.water.becalm(&self.gpu.queue);
        }
        let wet = water.live() && self.water.scene_view().is_some();
        {
            let target = if wet {
                self.water.scene_view().unwrap_or(&surface_view)
            } else {
                &surface_view
            };
            let mut pass = encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("eternalist-atelier-egui"),
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
        }
        if wet {
            self.water.compose(
                &self.gpu.device,
                &self.gpu.queue,
                &mut encoder,
                &surface_view,
                water,
            );
        }
        let _submission = self
            .gpu
            .queue
            .submit(user_commands.into_iter().chain([encoder.finish()]));
        if self
            .water
            .after_submit(&self.gpu.device, &self.gpu.queue, water)
        {
            self.window.request_redraw();
        }
        self.window.pre_present_notify();
        frame.present();
        let mut renderer = self.gpu.renderer.write();
        for id in &delta.free {
            renderer.free_texture(id);
        }
        true
    }

    fn acquire_frame(&mut self) -> Option<wgpu::SurfaceTexture> {
        match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => Some(frame),
            wgpu::CurrentSurfaceTexture::Timeout => {
                self.window.request_redraw();
                None
            }
            wgpu::CurrentSurfaceTexture::Occluded => None,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.surface.configure(&self.gpu.device, &self.config);
                self.window.request_redraw();
                None
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                eprintln!("atelier surface texture validation failure");
                None
            }
        }
    }
}

fn signal(state: &str, message: &str) {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return;
    };
    if let Some(root) = document.document_element() {
        let _marked = root.set_attribute("data-eternalist", state);
    }
    if let Some(status) = document.get_element_by_id("status") {
        status.set_text_content(Some(message));
    }
    if state == "failed" {
        web_sys::console::error_1(&message.into());
    }
}
