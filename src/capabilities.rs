//! Facts the host declares about one run.
//!
//! Applications consult one fact and never an operating system. The host
//! installs the set before the application is built; a context without a host
//! (tests, editors, the web atelier) reads the desktop set.

use egui::Id;

/// Facts about the surroundings of one run, declared by the host.
///
/// Every field is a fact, never a policy. Policies live where they are
/// applied: a Mechanism steps up when `touch` holds, water runs only while
/// `power_unconstrained` holds, and a shortcut hint appears only while
/// `keyboard` holds.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "each field is one independent fact, consulted by name"
)]
pub struct Capabilities {
    /// A hovering precise pointer exists. Tension, tooltips, and the Inspector
    /// actuator require it.
    pub pointer: bool,
    /// Direct multi-touch manipulation exists. Pinch and swipe are meaningful,
    /// and mechanisms step up to a fingertip.
    pub touch: bool,
    /// A physical keyboard with shortcuts is expected.
    pub keyboard: bool,
    /// The host may spend the GPU continuously on ornament: water, radiators,
    /// and tension. Handhelds never grant it.
    pub power_unconstrained: bool,
    /// A human-edited configuration file exists beside the application's
    /// settings.
    pub configuration: bool,
    /// The operating system may suspend or destroy the process at will. Close
    /// never exits, and the application checkpoints on suspension.
    pub retirement: bool,
}

impl Capabilities {
    /// A desktop workstation.
    pub const DESKTOP: Self = Self {
        pointer: true,
        touch: false,
        keyboard: true,
        power_unconstrained: true,
        configuration: true,
        retirement: false,
    };

    /// A handheld: a phone or a tablet held in the hand.
    pub const HANDHELD: Self = Self {
        pointer: false,
        touch: true,
        keyboard: false,
        power_unconstrained: false,
        configuration: false,
        retirement: true,
    };

    const ID: &str = "eternalist-capabilities";

    /// Declare the facts for every consumer of this context.
    pub fn install(self, ctx: &egui::Context) {
        ctx.data_mut(|data| {
            let _prior = data.insert_temp(Id::new(Self::ID), self);
        });
    }

    /// The facts declared for this context, or the desktop set without a host.
    #[must_use]
    pub fn of(ctx: &egui::Context) -> Self {
        ctx.data(|data| data.get_temp(Id::new(Self::ID)))
            .unwrap_or(Self::DESKTOP)
    }
}

impl Default for Capabilities {
    fn default() -> Self {
        Self::DESKTOP
    }
}
