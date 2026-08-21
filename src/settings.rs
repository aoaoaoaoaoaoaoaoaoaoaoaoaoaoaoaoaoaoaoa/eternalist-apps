//! Modal application settings with one shared interaction and presentation law.

#![deny(missing_docs)]

use std::{ops::RangeInclusive, path::Path};

use brass_poolrooms::{
    chrome::{
        self, Checkbox, MechanismSize, Monoglyph, MonoglyphFinish, MonoglyphResponse, NumberInput,
        ScrewScroll, Symbol,
    },
    water::Surface,
};

use crate::{
    commands::{SETTINGS_SHORTCUTS, Stroke, take},
    modal::ModalShell,
};

const NAME_SIZE: f32 = 15.0;
const DETAIL_SIZE: f32 = 14.0;
const FAULT: egui::Color32 = egui::Color32::from_rgb(214, 92, 46);

/// Stable application-owned metadata for one configurable preference.
#[derive(Clone, Copy, Debug)]
pub struct SettingSpec {
    id: &'static str,
    name: &'static str,
    detail: &'static str,
}

impl SettingSpec {
    /// Declare one setting independently of its current value or storage.
    #[must_use]
    pub const fn new(id: &'static str, name: &'static str, detail: &'static str) -> Self {
        Self { id, name, detail }
    }

    /// Stable machine-facing setting identity.
    #[must_use]
    pub const fn id(self) -> &'static str {
        self.id
    }

    /// User-facing setting name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        self.name
    }

    /// User-facing consequence of changing the setting.
    #[must_use]
    pub const fn detail(self) -> &'static str {
        self.detail
    }
}

/// Current configuration-file condition projected into the settings surface.
#[derive(Clone, Copy, Debug)]
pub struct SettingsFile<'a> {
    path: &'a Path,
    fault: Option<&'a str>,
    reload_pending: bool,
    reloadable: bool,
}

impl<'a> SettingsFile<'a> {
    /// Describe a valid configuration file.
    #[must_use]
    pub const fn ready(path: &'a Path) -> Self {
        Self {
            path,
            fault: None,
            reload_pending: false,
            reloadable: true,
        }
    }

    /// Describe a configuration file that cannot currently govern the app.
    #[must_use]
    pub const fn fault(path: &'a Path, message: &'a str) -> Self {
        Self {
            path,
            fault: Some(message),
            reload_pending: false,
            reloadable: true,
        }
    }

    /// Mark an in-flight reload without changing the last known condition.
    #[must_use]
    pub const fn reloading(mut self, pending: bool) -> Self {
        self.reload_pending = pending;
        self
    }

    /// Admit or withhold explicit disk reload while application changes settle.
    #[must_use]
    pub const fn reloadable(mut self, reloadable: bool) -> Self {
        self.reloadable = reloadable;
        self
    }

    const fn enabled(self) -> bool {
        self.fault.is_none() && !self.reload_pending
    }
}

/// Actions emitted by one settings presentation.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SettingsResponse {
    reload_requested: bool,
}

impl SettingsResponse {
    /// Whether the user asked to reread the configuration file from disk.
    #[must_use]
    pub const fn reload_requested(self) -> bool {
        self.reload_requested
    }
}

/// Stateful central settings surface.
#[derive(Debug, Default)]
pub struct SettingsSheet {
    shell: ModalShell,
}

impl SettingsSheet {
    /// Whether the settings sheet is open.
    #[must_use]
    pub const fn is_open(&self) -> bool {
        self.shell.is_open()
    }

    /// Geometry occupied by the settings card in its most recent open pass.
    #[must_use]
    pub const fn rect(&self) -> Option<egui::Rect> {
        self.shell.rect()
    }

    /// Open settings and remember the current focus restoration target.
    pub fn open(&mut self, ctx: &egui::Context) {
        self.shell.open(ctx);
    }

    /// Open settings when a preflight condition needs the user's attention.
    pub fn require_attention(&mut self, ctx: &egui::Context) {
        self.shell.open(ctx);
    }

    /// Close settings and restore the prior focus target when possible.
    pub fn close(&mut self, ctx: &egui::Context) {
        self.shell.close(ctx);
    }

    /// Consume the shared F2 or platform-familiar settings accelerator and
    /// toggle the sheet.
    ///
    /// Call this before ordinary application layout. While open, the sheet
    /// quarantines wheel input and returns it only to its own scroll surface.
    pub fn take_shortcut(&mut self, ctx: &egui::Context) -> bool {
        self.shell.prepare(ctx);
        let mut invoked = false;
        for shortcut in SETTINGS_SHORTCUTS {
            invoked |= take(ctx, shortcut) == Stroke::Fresh;
        }
        if invoked {
            self.shell.toggle(ctx);
        }
        self.shell.quarantine_wheel(ctx);
        invoked
    }

    /// Show the persistent settings actuator.
    pub fn activator(&mut self, ui: &mut egui::Ui, needs_attention: bool) -> MonoglyphResponse {
        self.shell.prepare(ui.ctx());
        let mut actuator = Monoglyph::symbol(Symbol::Settings).size(MechanismSize::Medium);
        if needs_attention {
            actuator = actuator.finish(MonoglyphFinish::Danger);
        }
        let hint = if needs_attention {
            format!(
                "Settings need attention · {} or {}",
                SETTINGS_SHORTCUTS[0].label(ui.ctx()),
                SETTINGS_SHORTCUTS[1].label(ui.ctx())
            )
        } else {
            format!(
                "Settings · {} or {}",
                SETTINGS_SHORTCUTS[0].label(ui.ctx()),
                SETTINGS_SHORTCUTS[1].label(ui.ctx())
            )
        };
        let response = actuator.show(ui).on_hover_text(hint);
        record(ui, "eternalist.settings.open", response.rect);
        if response.clicked() {
            self.shell.toggle(ui.ctx());
        }
        response
    }

    /// Render settings above the completed application UI.
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        water: &mut Surface,
        file: SettingsFile<'_>,
        add_settings: impl FnOnce(&mut SettingsUi<'_>),
    ) -> SettingsResponse {
        if !self.shell.begin_present(ctx) {
            return SettingsResponse::default();
        }
        let width = (ctx.content_rect().width() - 48.0).clamp(380.0, 680.0);
        let body_height = (ctx.content_rect().height() - 220.0).clamp(220.0, 520.0);
        let mut close = false;
        let mut reload_requested = false;
        let focus_close = self.shell.focus_close();
        let modal = egui::Modal::new(egui::Id::new("eternalist-settings"))
            .frame(
                egui::Frame::new()
                    .fill(chrome::SURFACE)
                    .stroke(egui::Stroke::new(1.5_f32, chrome::EDGE_STRONG))
                    .corner_radius(2)
                    .inner_margin(egui::Margin::same(14)),
            )
            .backdrop_color(egui::Color32::from_black_alpha(176))
            .show(ctx, |ui| {
                ui.set_width(width);
                let _header = ui.horizontal(|ui| {
                    let _title = ui.label(chrome::title("SETTINGS"));
                    let _close =
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let response = Monoglyph::symbol(Symbol::Remove)
                                .size(MechanismSize::Small)
                                .show(ui)
                                .on_hover_text("Close settings · Escape");
                            if focus_close {
                                response.request_focus();
                            }
                            record(ui, "eternalist.settings.close", response.rect);
                            close |= response.clicked();
                        });
                });
                let _hint = ui.label(chrome::muted(format!(
                    "{} or {} toggles settings",
                    SETTINGS_SHORTCUTS[0].label(ui.ctx()),
                    SETTINGS_SHORTCUTS[1].label(ui.ctx())
                )));
                ui.add_space(10.0);
                reload_requested |= settings_body(ui, water, file, body_height, add_settings);
                ui.min_rect()
            });
        self.shell
            .finish_present(ctx, modal.inner, close || modal.should_close());
        SettingsResponse { reload_requested }
    }
}

fn settings_body(
    ui: &mut egui::Ui,
    water: &mut Surface,
    file: SettingsFile<'_>,
    max_height: f32,
    add_settings: impl FnOnce(&mut SettingsUi<'_>),
) -> bool {
    let mut reload_requested = false;
    if let Some(fault) = file.fault {
        fault_card(
            ui,
            water,
            fault,
            file.reload_pending,
            file.reloadable,
            &mut reload_requested,
        );
        ui.add_space(10.0);
    }
    let _body = ScrewScroll::vertical()
        .id_salt("eternalist-settings-body")
        .min_scrolled_height(190.0)
        .max_height(max_height)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            let mut settings = SettingsUi {
                ui,
                water,
                enabled: file.enabled(),
            };
            add_settings(&mut settings);
            settings.ui.add_space(12.0);
            let _source = settings.ui.horizontal(|ui| {
                let _label = ui.label(chrome::eyebrow("CONFIGURATION FILE"));
                if file.fault.is_none() {
                    let _reload =
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            reload_requested |= reload_actuator(
                                ui,
                                settings.water,
                                file.reload_pending,
                                file.reloadable,
                                false,
                            );
                        });
                }
            });
            settings.ui.add_space(3.0);
            let _path = settings.ui.add(
                egui::Label::new(
                    egui::RichText::new(file.path.display().to_string())
                        .monospace()
                        .size(13.0)
                        .color(chrome::MUTED),
                )
                .selectable(true)
                .wrap(),
            );
            settings.ui.add_space(8.0);
        });
    reload_requested
}

/// Restricted settings-layout surface supplied to an application.
pub struct SettingsUi<'a> {
    ui: &'a mut egui::Ui,
    water: &'a mut Surface,
    enabled: bool,
}

impl SettingsUi<'_> {
    /// Begin a named settings group.
    pub fn section(&mut self, title: impl Into<String>) {
        let _title = self.ui.label(chrome::eyebrow(title));
        self.ui.add_space(4.0);
    }

    /// Render one boolean preference and return whether it changed.
    pub fn boolean(&mut self, spec: SettingSpec, value: &mut bool) -> bool {
        setting_row(self.ui, self.water, self.enabled, spec, |ui, water| {
            let control = Checkbox::without_text(value)
                .size(MechanismSize::Small)
                .show(ui);
            water.checkbox(&control);
            (control.rect, control.changed())
        })
    }

    /// Render one bounded floating-point preference and return whether it changed.
    pub fn number(
        &mut self,
        spec: SettingSpec,
        value: &mut f64,
        range: RangeInclusive<f64>,
        step: f64,
        precision: usize,
    ) -> bool {
        setting_row(self.ui, self.water, self.enabled, spec, |ui, water| {
            let control = NumberInput::new(value, range, step, precision).show(ui);
            water.number_input(&control);
            (control.rect, control.changed())
        })
    }
}

fn setting_row(
    ui: &mut egui::Ui,
    water: &mut Surface,
    enabled: bool,
    spec: SettingSpec,
    control: impl FnOnce(&mut egui::Ui, &mut Surface) -> (egui::Rect, bool),
) -> bool {
    let mut changed = false;
    let _row = ui.add_enabled_ui(enabled, |ui| {
        let _contents = ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), 48.0),
            egui::Layout::right_to_left(egui::Align::Center),
            |ui| {
                let (rect, control_changed) = control(ui, water);
                record(ui, format!("eternalist.settings.{}", spec.id()), rect);
                changed = control_changed;
                ui.add_space(12.0);
                let _copy =
                    ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
                        let _name = ui.label(
                            egui::RichText::new(spec.name())
                                .size(NAME_SIZE)
                                .color(chrome::TEXT),
                        );
                        let _detail = ui.label(
                            egui::RichText::new(spec.detail())
                                .size(DETAIL_SIZE)
                                .color(chrome::MUTED),
                        );
                    });
            },
        );
    });
    changed
}

fn fault_card(
    ui: &mut egui::Ui,
    water: &mut Surface,
    message: &str,
    reload_pending: bool,
    reloadable: bool,
    reload_requested: &mut bool,
) {
    let _card = egui::Frame::new()
        .fill(egui::Color32::from_rgba_unmultiplied(80, 27, 12, 176))
        .stroke(egui::Stroke::new(1.5_f32, FAULT))
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            let _row = ui.horizontal(|ui| {
                let _copy = ui.vertical(|ui| {
                    let _title = ui.label(
                        egui::RichText::new("CONFIGURATION NEEDS ATTENTION")
                            .size(14.0)
                            .strong()
                            .color(FAULT),
                    );
                    let _detail =
                        ui.label(egui::RichText::new(message).size(13.0).color(chrome::TEXT));
                });
                let _reload =
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        *reload_requested |=
                            reload_actuator(ui, water, reload_pending, reloadable, true);
                    });
            });
        });
}

fn reload_actuator(
    ui: &mut egui::Ui,
    water: &mut Surface,
    pending: bool,
    reloadable: bool,
    labeled: bool,
) -> bool {
    let response = ui
        .add_enabled_ui(reloadable && !pending, |ui| {
            Monoglyph::symbol(Symbol::Restore)
                .size(MechanismSize::Small)
                .show(ui)
        })
        .inner
        .on_hover_text(if pending {
            "Reading the configuration file"
        } else if reloadable {
            "Reread the configuration file from disk"
        } else {
            "Wait for application changes to finish saving"
        });
    water.monoglyph(&response);
    record(ui, "eternalist.settings.reload", response.rect);
    if labeled {
        let _label = ui.label(chrome::section_title(if pending {
            "READING"
        } else {
            "RELOAD"
        }));
    }
    response.clicked()
}

#[inline]
fn record(ui: &egui::Ui, name: impl Into<String>, rect: egui::Rect) {
    #[cfg(all(
        feature = "egui-test",
        any(target_os = "linux", target_os = "macos", target_os = "windows")
    ))]
    egui_tester_witness::egui::record(ui, name, rect);
    #[cfg(not(all(
        feature = "egui-test",
        any(target_os = "linux", target_os = "macos", target_os = "windows")
    )))]
    let _ = (ui, name, rect);
}
