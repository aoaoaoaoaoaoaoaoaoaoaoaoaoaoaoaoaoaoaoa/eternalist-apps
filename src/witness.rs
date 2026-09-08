//! Application-level semantic Target publication.

#![deny(missing_docs)]

use std::fmt::{self, Display, Formatter, Write as _};

/// Shared semantic Targets owned by Eternalist application composition.
///
/// Products own their domain Targets. These variants are reserved for the
/// application header, Command Guide, and Settings Sheet rendered by this
/// crate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApplicationTarget<'a> {
    /// Complete Application Header geometry.
    Header,
    /// Application name in the header.
    Name,
    /// Header Help actuator.
    Help,
    /// Header Settings actuator.
    SettingsOpen,
    /// Settings close actuator.
    SettingsClose,
    /// Complete Settings Sheet body.
    SettingsBody,
    /// Configuration path displayed by the Settings Sheet.
    SettingsPath,
    /// Configuration fault card.
    SettingsFault,
    /// Configuration reload actuator.
    SettingsReload,
    /// One application-defined setting row.
    Setting(&'a str),
    /// Complete Command Guide body.
    CommandGuideBody,
}

impl Display for ApplicationTarget<'_> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Header => formatter.write_str("eternalist.application.header"),
            Self::Name => formatter.write_str("eternalist.application.name"),
            Self::Help => formatter.write_str("eternalist.application.help"),
            Self::SettingsOpen => formatter.write_str("eternalist.settings.open"),
            Self::SettingsClose => formatter.write_str("eternalist.settings.close"),
            Self::SettingsBody => formatter.write_str("eternalist.settings.body"),
            Self::SettingsPath => formatter.write_str("eternalist.settings.path"),
            Self::SettingsFault => formatter.write_str("eternalist.settings.fault"),
            Self::SettingsReload => formatter.write_str("eternalist.settings.reload"),
            Self::Setting(id) => {
                formatter.write_str("eternalist.settings.entry/")?;
                write_identity(formatter, id)
            }
            Self::CommandGuideBody => formatter.write_str("eternalist.command-guide.body"),
        }
    }
}

fn write_identity(formatter: &mut Formatter<'_>, identity: &str) -> fmt::Result {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    for byte in identity.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            formatter.write_char(char::from(byte))?;
        } else {
            formatter.write_char('%')?;
            formatter.write_char(char::from(HEX[usize::from(byte >> 4)]))?;
            formatter.write_char(char::from(HEX[usize::from(byte & 0x0f)]))?;
        }
    }
    Ok(())
}

/// Publish an egui response as one semantic Target in observational builds.
///
/// Ordinary builds erase publication completely. This façade keeps the
/// feature boundary and focus-preserving response projection out of products;
/// egui-tester-witness remains the recording authority.
#[inline]
pub fn response(ui: &egui::Ui, target: impl Display, response: &egui::Response) {
    #[cfg(all(
        feature = "egui-test",
        any(target_os = "linux", target_os = "macos", target_os = "windows")
    ))]
    egui_tester_witness::egui::record_response(ui, target.to_string(), response);
    #[cfg(not(all(
        feature = "egui-test",
        any(target_os = "linux", target_os = "macos", target_os = "windows")
    )))]
    {
        let _ = (ui, response);
        drop(target);
    }
}

/// Publish a rectangle allocated by `ui` as one semantic Target in observational builds.
///
/// Like [`rect`], this projection carries no keyboard-focus evidence.
#[inline]
pub fn anchor(ui: &egui::Ui, target: impl Display, rect: egui::Rect) {
    #[cfg(all(
        feature = "egui-test",
        any(target_os = "linux", target_os = "macos", target_os = "windows")
    ))]
    egui_tester_witness::egui::record(ui, target.to_string(), rect);
    #[cfg(not(all(
        feature = "egui-test",
        any(target_os = "linux", target_os = "macos", target_os = "windows")
    )))]
    {
        let _ = (ui, rect);
        drop(target);
    }
}

/// Publish painter-owned geometry as one semantic Target in observational builds.
///
/// Unlike [`response`], this projection carries no keyboard-focus evidence.
#[inline]
pub fn rect(ctx: &egui::Context, target: impl Display, rect: egui::Rect) {
    #[cfg(all(
        feature = "egui-test",
        any(target_os = "linux", target_os = "macos", target_os = "windows")
    ))]
    egui_tester_witness::egui::record_rect(ctx, target.to_string(), rect);
    #[cfg(not(all(
        feature = "egui-test",
        any(target_os = "linux", target_os = "macos", target_os = "windows")
    )))]
    {
        let _ = (ctx, rect);
        drop(target);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_targets_obey_the_fleet_wire_grammar() {
        assert_eq!(
            ApplicationTarget::CommandGuideBody.to_string(),
            "eternalist.command-guide.body"
        );
        assert_eq!(
            ApplicationTarget::Setting("remote/source 1").to_string(),
            "eternalist.settings.entry/remote%2Fsource%201"
        );
    }
}
