//! The operating-system entry token handed to the native host.

use crate::Capabilities;
use std::path::PathBuf;

#[cfg(target_os = "android")]
pub use winit::platform::android::activity::AndroidApp;

/// Environment variable overriding the desktop facts, for proving the handheld
/// projection under a desktop testbed. Its only admitted value is `handheld`.
pub const CAPABILITIES_ENV: &str = "ETERNALIST_CAPABILITIES";

/// How the process entered: bare on a desktop, or through Android's
/// `NativeActivity` with its application handle.
#[derive(Clone, Debug)]
pub enum Ingress {
    /// A desktop process started from its command line.
    Desktop,
    /// An Android `NativeActivity` with the handle winit and storage require.
    #[cfg(target_os = "android")]
    Android(AndroidApp),
}

impl Ingress {
    /// The facts this entry declares.
    #[must_use]
    pub fn capabilities(&self) -> Capabilities {
        match self {
            Self::Desktop => {
                if std::env::var_os(CAPABILITIES_ENV).is_some_and(|value| value == "handheld") {
                    Capabilities::HANDHELD
                } else {
                    Capabilities::DESKTOP
                }
            }
            #[cfg(target_os = "android")]
            Self::Android(_) => Capabilities::HANDHELD,
        }
    }

    /// The private storage root the platform hands the application, when the
    /// platform owns storage instead of the user's home directory.
    #[must_use]
    pub fn private_root(&self) -> Option<PathBuf> {
        match self {
            Self::Desktop => None,
            #[cfg(target_os = "android")]
            Self::Android(android) => android.internal_data_path(),
        }
    }
}
