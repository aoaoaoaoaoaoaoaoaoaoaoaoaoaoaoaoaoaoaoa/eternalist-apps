//! Persistent application identity and universal actions.

#![deny(missing_docs)]

use brass_poolrooms::{chrome::MonoglyphResponse, water::Surface};

use crate::{command_guide::CommandGuide, settings::SettingsSheet};

/// Application identity and universal controls placed above a persistent
/// control surface.
#[derive(Clone, Copy, Debug)]
pub struct ApplicationHeader<'a> {
    name: &'a str,
    settings_attention: bool,
}

impl<'a> ApplicationHeader<'a> {
    /// Name one application header.
    #[must_use]
    pub const fn new(name: &'a str) -> Self {
        Self {
            name,
            settings_attention: false,
        }
    }

    /// Give the settings actuator its fault finish.
    #[must_use]
    pub const fn settings_attention(mut self, attention: bool) -> Self {
        self.settings_attention = attention;
        self
    }

    /// Present application identity opposite right-justified Help and Settings.
    pub fn show(
        self,
        ui: &mut egui::Ui,
        guide: &mut CommandGuide,
        settings: &mut SettingsSheet,
        water: &mut Surface,
    ) -> ApplicationHeaderResponse {
        let row = ui.horizontal(|ui| {
            let title = ui.label(brass_poolrooms::chrome::title(self.name).size(18.0));
            ui.add_space(3.0);
            let actions = ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let settings = settings.activator(ui, self.settings_attention);
                water.monoglyph(&settings);
                let help = guide.activator(ui);
                water.monoglyph(&help);
                (help, settings)
            });
            let (help, settings) = actions.inner;
            (title, help, settings)
        });
        let (title, help, settings) = row.inner;
        record(ui, "eternalist.application.header", row.response.rect);
        record(ui, "eternalist.application.name", title.rect);
        record(ui, "eternalist.application.help", help.rect);
        ApplicationHeaderResponse {
            title,
            help,
            settings,
            rect: row.response.rect,
        }
    }
}

/// Responses from one application-header presentation.
pub struct ApplicationHeaderResponse {
    /// Selectable application name.
    pub title: egui::Response,
    /// Persistent Help actuator.
    pub help: MonoglyphResponse,
    /// Persistent Settings actuator.
    pub settings: MonoglyphResponse,
    /// Complete header geometry.
    pub rect: egui::Rect,
}

#[cfg(feature = "egui-test")]
fn record(ui: &egui::Ui, name: impl Into<String>, rect: egui::Rect) {
    egui_tester_witness::egui::record(ui, name, rect);
}

#[cfg(not(feature = "egui-test"))]
fn record(_ui: &egui::Ui, _name: impl Into<String>, _rect: egui::Rect) {}
