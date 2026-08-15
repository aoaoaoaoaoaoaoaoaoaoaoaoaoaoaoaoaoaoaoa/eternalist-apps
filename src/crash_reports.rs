//! Private crash-capsule recovery and explicit-consent delivery.

use std::{
    backtrace::Backtrace,
    fs::{self, File, OpenOptions},
    io::{Read as _, Write as _},
    path::{Path, PathBuf},
    sync::{Arc, mpsc},
    thread,
    time::{Duration, SystemTime},
};

use brass_poolrooms::chrome;
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use ureq::tls::{RootCerts, TlsConfig, TlsProvider};

use crate::NativeWake;

const SCHEMA: u8 = 1;
const CAPSULE_NAME: &str = "crash-report-v1.json";
const MAX_CAPSULE_BYTES: u64 = 16 * 1024;
const MAX_STACK_FRAMES: usize = 32;
const MAX_SYMBOL_BYTES: usize = 240;
const SEND_TIMEOUT: Duration = Duration::from_secs(5);

// Armed only after the intake stack exists and its ordinary admission path has
// passed the release acceptance. There is deliberately no runtime override.
const PRODUCTION_INTAKE_URL: Option<&str> = Some("https://faults.eternalist.moe/v1/report");

/// Closed product identity admitted by the Eternalist crash intake.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CrashProduct {
    /// HRRR weather viewer.
    Hrrr,
    /// Adequate Trailgen.
    Trailgen,
    /// Adequate Booru Viewer.
    BooruViewer,
}

/// One product's crash-report identity and local storage boundary.
#[derive(Clone, Debug)]
pub struct CrashReportSpec {
    product: CrashProduct,
    release: &'static str,
    state_dir: PathBuf,
    endpoint: Option<String>,
}

impl CrashReportSpec {
    /// Declare crash recovery for one released product.
    #[must_use]
    pub fn new(
        product: CrashProduct,
        release: &'static str,
        state_dir: impl Into<PathBuf>,
    ) -> Self {
        Self {
            product,
            release,
            state_dir: state_dir.into(),
            endpoint: PRODUCTION_INTAKE_URL.map(str::to_owned),
        }
    }

    /// Bind an isolated stack to the black-box crash-path acceptance.
    ///
    /// This constructor does not exist in ordinary product builds. Production
    /// products cannot redirect reports through environment or configuration.
    #[cfg(feature = "egui-test")]
    #[doc(hidden)]
    #[must_use]
    pub fn acceptance(
        product: CrashProduct,
        release: &'static str,
        state_dir: impl Into<PathBuf>,
        endpoint: impl Into<String>,
    ) -> Self {
        Self {
            product,
            release,
            state_dir: state_dir.into(),
            endpoint: Some(endpoint.into()),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct CrashReport {
    schema: u8,
    product: CrashProduct,
    release: String,
    platform: Platform,
    fault: Fault,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Platform {
    os: String,
    arch: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Fault {
    kind: FaultKind,
    site: Option<FaultSite>,
    stack: Vec<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum FaultKind {
    Panic,
    HostFailure,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct FaultSite {
    file: String,
    line: u32,
    column: u32,
}

struct Recorder {
    spec: CrashReportSpec,
    capsule: PathBuf,
}

impl Recorder {
    fn capture(&self, kind: FaultKind, site: Option<FaultSite>) -> std::io::Result<()> {
        if self.capsule.exists() {
            return Ok(());
        }
        let report = CrashReport {
            schema: SCHEMA,
            product: self.spec.product,
            release: self.spec.release.to_owned(),
            platform: Platform {
                os: std::env::consts::OS.to_owned(),
                arch: std::env::consts::ARCH.to_owned(),
            },
            fault: Fault {
                kind,
                site,
                stack: capture_stack(),
            },
        };
        persist_once(&self.capsule, &report)
    }
}

type PanicHook = dyn Fn(&std::panic::PanicHookInfo<'_>) + Send + Sync + 'static;

struct HookGuard {
    previous: Arc<PanicHook>,
}

impl Drop for HookGuard {
    fn drop(&mut self) {
        if thread::panicking() {
            return;
        }
        let previous = Arc::clone(&self.previous);
        std::panic::set_hook(Box::new(move |panic| previous(panic)));
    }
}

enum Delivery {
    Idle,
    Sending(mpsc::Receiver<bool>),
    Failed,
}

#[derive(Default)]
struct QuarantinedInput {
    events: Vec<egui::Event>,
    keys_down: std::collections::HashSet<egui::Key>,
    modifiers: egui::Modifiers,
    smooth_scroll_delta: egui::Vec2,
}

/// Per-run crash recovery state installed by the native host.
pub(crate) struct CrashReports {
    recorder: Option<Arc<Recorder>>,
    pending: Option<CrashReport>,
    hook: Option<HookGuard>,
    wake: NativeWake,
    delivery: Delivery,
    exact_open: bool,
    quarantined: Option<QuarantinedInput>,
}

pub(crate) struct HostFailureGuard {
    recorder: Option<Arc<Recorder>>,
    complete: bool,
}

impl HostFailureGuard {
    pub(crate) fn complete(&mut self) {
        self.complete = true;
    }
}

impl Drop for HostFailureGuard {
    fn drop(&mut self) {
        if !self.complete
            && let Some(recorder) = &self.recorder
        {
            let _captured = recorder.capture(FaultKind::HostFailure, None);
        }
    }
}

impl CrashReports {
    pub(crate) fn arm(spec: Option<CrashReportSpec>, ctx: &egui::Context) -> Self {
        let Some(spec) = spec else {
            return Self::inert(ctx);
        };
        let capsule = spec.state_dir.join(CAPSULE_NAME);
        let pending = load_pending(&capsule, spec.product);
        let recorder = Arc::new(Recorder { spec, capsule });
        let previous: Arc<PanicHook> = Arc::from(std::panic::take_hook());
        let hook_recorder = Arc::clone(&recorder);
        let hook_previous = Arc::clone(&previous);
        std::panic::set_hook(Box::new(move |panic| {
            let site = panic.location().map(|location| FaultSite {
                file: source_relative(location.file()),
                line: location.line(),
                column: location.column(),
            });
            let _captured = hook_recorder.capture(FaultKind::Panic, site);
            hook_previous(panic);
        }));
        Self {
            recorder: Some(recorder),
            pending,
            hook: Some(HookGuard { previous }),
            wake: NativeWake::from_context(ctx),
            delivery: Delivery::Idle,
            exact_open: false,
            quarantined: None,
        }
    }

    fn inert(ctx: &egui::Context) -> Self {
        Self {
            recorder: None,
            pending: None,
            hook: None,
            wake: NativeWake::from_context(ctx),
            delivery: Delivery::Idle,
            exact_open: false,
            quarantined: None,
        }
    }

    pub(crate) fn host_failure_guard(&self) -> HostFailureGuard {
        HostFailureGuard {
            recorder: self.recorder.clone(),
            complete: false,
        }
    }

    pub(crate) fn quarantine_input(&mut self, ctx: &egui::Context) {
        if self.pending.is_none() || self.quarantined.is_some() {
            return;
        }
        self.quarantined = Some(ctx.input_mut(|input| QuarantinedInput {
            events: std::mem::take(&mut input.events),
            keys_down: std::mem::take(&mut input.keys_down),
            modifiers: std::mem::take(&mut input.modifiers),
            smooth_scroll_delta: std::mem::take(&mut input.smooth_scroll_delta),
        }));
    }

    pub(crate) fn restore_input(&mut self, ctx: &egui::Context) {
        let Some(input) = self.quarantined.take() else {
            return;
        };
        ctx.input_mut(|state| {
            state.events = input.events;
            state.keys_down = input.keys_down;
            state.modifiers = input.modifiers;
            state.smooth_scroll_delta = input.smooth_scroll_delta;
        });
    }

    pub(crate) fn show(&mut self, ctx: &egui::Context) {
        self.settle_delivery();
        let Some(report) = self.pending.as_ref() else {
            return;
        };
        let mut discard = false;
        let mut send = false;
        let mut exact_open = self.exact_open;
        let sending = matches!(self.delivery, Delivery::Sending(_));
        let failed = matches!(self.delivery, Delivery::Failed);
        let delivery_armed = self
            .recorder
            .as_ref()
            .is_some_and(|recorder| recorder.spec.endpoint.is_some());
        let modal = egui::Modal::new(egui::Id::new("eternalist-crash-report-consent"))
            .frame(
                egui::Frame::new()
                    .fill(chrome::SURFACE)
                    .stroke(egui::Stroke::new(1.5, chrome::EDGE_STRONG))
                    .corner_radius(2)
                    .inner_margin(egui::Margin::same(16)),
            )
            .backdrop_color(egui::Color32::from_black_alpha(176))
            .show(ctx, |ui| {
                ui.set_width((ctx.content_rect().width() - 48.0).clamp(360.0, 600.0));
                ui.label(chrome::title("SEND A CRASH REPORT?"));
                ui.add_space(7.0);
                ui.label("The application closed unexpectedly. Nothing is sent unless you choose Send report.");
                ui.add_space(10.0);
                ui.label("The report contains only:");
                ui.label("• application and release");
                ui.label("• operating system and architecture");
                ui.label("• panic category and source location, when available");
                ui.label("• function names from the crash stack");
                ui.add_space(6.0);
                ui.label(chrome::muted(
                    "No files, settings, searches, map position, screenshots, or usage data.",
                ));
                ui.add_space(8.0);
                if ui
                    .selectable_label(exact_open, "View exact report")
                    .clicked()
                {
                    exact_open = !exact_open;
                }
                if exact_open {
                    let exact = serde_json::to_string_pretty(report)
                        .unwrap_or_else(|_| "Report could not be displayed.".to_owned());
                    egui::ScrollArea::vertical()
                        .id_salt("eternalist-crash-report-exact")
                        .max_height(220.0)
                        .show(ui, |ui| {
                            ui.add(egui::Label::new(egui::RichText::new(exact).monospace()).selectable(true));
                        });
                }
                if failed {
                    ui.add_space(8.0);
                    ui.colored_label(chrome::HOT, "The report was not accepted. It remains only on this computer.");
                }
                if !delivery_armed {
                    ui.add_space(8.0);
                    ui.label(chrome::muted("Crash delivery is not armed in this build."));
                }
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    let discard_response =
                        ui.add_enabled(!sending, egui::Button::new("DON'T SEND"));
                    witness_response(ui, "eternalist.crash-report.discard", &discard_response);
                    if discard_response.clicked() {
                        discard = true;
                    }
                    let label = if sending { "SENDING…" } else if failed { "RETRY" } else { "SEND REPORT" };
                    let send_response =
                        ui.add_enabled(!sending && delivery_armed, egui::Button::new(label));
                    if send_response.enabled() {
                        witness_response(ui, "eternalist.crash-report.send", &send_response);
                    }
                    if send_response.clicked() {
                        witness_response(ui, "eternalist.crash-report.send-clicked", &send_response);
                        send = true;
                    }
                });
            });
        self.exact_open = exact_open;
        if discard || modal.should_close() && !sending {
            self.discard();
        } else if send {
            self.send();
        }
    }

    fn settle_delivery(&mut self) {
        let Delivery::Sending(receiver) = &self.delivery else {
            return;
        };
        match receiver.try_recv() {
            Ok(true) => {
                self.remove_capsule();
                self.pending = None;
                self.delivery = Delivery::Idle;
            }
            Ok(false) | Err(mpsc::TryRecvError::Disconnected) => {
                self.delivery = Delivery::Failed;
            }
            Err(mpsc::TryRecvError::Empty) => {}
        }
    }

    fn send(&mut self) {
        let Some(report) = &self.pending else {
            return;
        };
        let Some(endpoint) = self
            .recorder
            .as_ref()
            .and_then(|recorder| recorder.spec.endpoint.clone())
        else {
            return;
        };
        let Ok(body) = serde_json::to_vec(report) else {
            self.delivery = Delivery::Failed;
            return;
        };
        let digest = format!("{:x}", Sha256::digest(&body));
        let (sender, receiver) = mpsc::sync_channel(1);
        let wake = self.wake.clone();
        let spawn = thread::Builder::new()
            .name("crash-report-delivery".to_owned())
            .spawn(move || {
                let accepted = match deliver(&endpoint, body, digest) {
                    Ok(202) => true,
                    Ok(status) => {
                        eprintln!("crash report delivery refused with HTTP {status}");
                        false
                    }
                    Err(error) => {
                        eprintln!("crash report delivery failed: {error}");
                        false
                    }
                };
                let _sent = sender.send(accepted);
                let _woken = wake.request_repaint();
            });
        if spawn.is_ok() {
            self.delivery = Delivery::Sending(receiver);
        } else {
            self.delivery = Delivery::Failed;
        }
    }

    fn discard(&mut self) {
        self.remove_capsule();
        self.pending = None;
        self.delivery = Delivery::Idle;
    }

    fn remove_capsule(&self) {
        if let Some(recorder) = &self.recorder {
            let _removed = fs::remove_file(&recorder.capsule);
        }
    }
}

fn deliver(endpoint: &str, body: Vec<u8>, digest: String) -> Result<u16, ureq::Error> {
    let agent = ureq::Agent::config_builder()
        .https_only(true)
        .max_redirects(0)
        .http_status_as_error(false)
        .tls_config(
            TlsConfig::builder()
                .provider(TlsProvider::NativeTls)
                .root_certs(RootCerts::PlatformVerifier)
                .build(),
        )
        .timeout_global(Some(SEND_TIMEOUT))
        .build()
        .new_agent();
    agent
        .post(endpoint)
        .content_type("application/json")
        .header("x-eternalist-content-sha256", digest)
        .send(body)
        .map(|response| response.status().as_u16())
}

/// Exercise the native crash filesystem and TLS seams without sending a report.
#[cfg(feature = "egui-test")]
#[doc(hidden)]
pub fn native_crash_acceptance(endpoint: &str) -> Result<(), String> {
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|error| format!("read system time: {error}"))?
        .as_nanos();
    let state = std::env::temp_dir().join(format!(
        "eternalist-crash-acceptance-{}-{nonce}",
        std::process::id()
    ));
    if state.exists() {
        return Err(format!(
            "acceptance state already exists: {}",
            state.display()
        ));
    }
    let result = (|| {
        let spec =
            CrashReportSpec::acceptance(CrashProduct::Hrrr, "0.0.0-acceptance", &state, endpoint);
        let capsule = state.join(CAPSULE_NAME);
        let recorder = Recorder { spec, capsule };
        recorder
            .capture(FaultKind::HostFailure, None)
            .map_err(|error| format!("persist capsule in new state directory: {error}"))?;
        load_pending(&recorder.capsule, CrashProduct::Hrrr)
            .ok_or_else(|| "reload the persisted capsule".to_owned())?;

        let body = b"{}".to_vec();
        let digest = format!("{:x}", Sha256::digest(&body));
        let status = deliver(endpoint, body, digest)
            .map_err(|error| format!("native TLS delivery probe: {error}"))?;
        if status != 400 {
            return Err(format!(
                "invalid delivery probe returned HTTP {status}, expected 400"
            ));
        }
        Ok(())
    })();
    if let Err(error) = fs::remove_dir_all(&state)
        && error.kind() != std::io::ErrorKind::NotFound
    {
        return Err(format!(
            "remove acceptance state {}: {error}",
            state.display()
        ));
    }
    result
}

impl Drop for CrashReports {
    fn drop(&mut self) {
        // Restore before dropping the recorder captured by the installed hook.
        drop(self.hook.take());
    }
}

fn load_pending(path: &Path, product: CrashProduct) -> Option<CrashReport> {
    let loaded = (|| {
        let file = File::open(path).ok()?;
        let length = file.metadata().ok()?.len();
        if length > MAX_CAPSULE_BYTES {
            return None;
        }
        let mut bytes = Vec::with_capacity(usize::try_from(length).unwrap_or(0));
        file.take(MAX_CAPSULE_BYTES + 1)
            .read_to_end(&mut bytes)
            .ok()?;
        let report: CrashReport = serde_json::from_slice(&bytes).ok()?;
        (report.schema == SCHEMA && report.product == product).then_some(report)
    })();
    if loaded.is_none() && path.exists() {
        let _removed = fs::remove_file(path);
    }
    loaded
}

#[cfg(feature = "egui-test")]
fn witness_response(ui: &egui::Ui, name: &'static str, response: &egui::Response) {
    egui_tester_witness::egui::record_response(ui, name, response);
}

#[cfg(not(feature = "egui-test"))]
fn witness_response(_ui: &egui::Ui, _name: &'static str, _response: &egui::Response) {}

fn persist_once(path: &Path, report: &CrashReport) -> std::io::Result<()> {
    if path.exists() {
        return Ok(());
    }
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    fs::create_dir_all(parent)?;
    let temporary = parent.join(format!(".{CAPSULE_NAME}.{}", std::process::id()));
    let _stale = fs::remove_file(&temporary);
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    let mut file = options.open(&temporary)?;
    serde_json::to_writer(&mut file, report)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    drop(file);
    if path.exists() {
        fs::remove_file(temporary)
    } else {
        fs::rename(temporary, path)
    }
}

fn capture_stack() -> Vec<String> {
    let mut symbols = Vec::new();
    for line in Backtrace::force_capture().to_string().lines() {
        let Some((index, symbol)) = line.trim().split_once(": ") else {
            continue;
        };
        if !index.bytes().all(|byte| byte.is_ascii_digit()) {
            continue;
        }
        let symbol = symbol.trim();
        if symbol.is_empty()
            || symbol.contains("crash_reports")
            || symbol.contains("std::panicking")
            || symbol.contains("core::panicking")
            || symbol.contains('/')
            || symbol.contains('\\')
        {
            continue;
        }
        let symbol = truncate_utf8(symbol, MAX_SYMBOL_BYTES).to_owned();
        if symbols.last() != Some(&symbol) {
            symbols.push(symbol);
        }
        if symbols.len() == MAX_STACK_FRAMES {
            break;
        }
    }
    symbols
}

fn source_relative(file: &str) -> String {
    let normalized = file.replace('\\', "/");
    for marker in ["/src/", "/examples/"] {
        if let Some(index) = normalized.rfind(marker) {
            return normalized[index + 1..].to_owned();
        }
    }
    if !normalized.starts_with('/')
        && !normalized.contains(':')
        && !normalized.split('/').any(|part| part == "..")
    {
        return truncate_utf8(&normalized, MAX_SYMBOL_BYTES).to_owned();
    }
    "<redacted>".to_owned()
}

fn truncate_utf8(value: &str, bytes: usize) -> &str {
    if value.len() <= bytes {
        return value;
    }
    let mut boundary = bytes;
    while !value.is_char_boundary(boundary) {
        boundary -= 1;
    }
    &value[..boundary]
}
