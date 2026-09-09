//! Platform application directories derived from a product identity.

use crate::{Ingress, ProductIdentity};
use anyhow::{Context as _, Result};
use directories::ProjectDirs;
use std::path::{Path, PathBuf};

/// A product's platform-correct directories.
///
/// `state` is `XDG_STATE_HOME` on Linux; macOS and Windows fold state beneath
/// `local_data/state`. `runtime` exists only where the platform supplies a
/// per-session runtime directory.
#[derive(Clone, Debug)]
pub struct ApplicationPaths {
    pub config: PathBuf,
    pub data: PathBuf,
    pub local_data: PathBuf,
    pub cache: PathBuf,
    pub state: PathBuf,
    pub runtime: Option<PathBuf>,
}

impl ApplicationPaths {
    /// Resolve the directories without creating them.
    ///
    /// # Errors
    ///
    /// Returns an error when the platform exposes no home directory.
    pub fn claim(product: ProductIdentity) -> Result<Self> {
        let (qualifier, organization, application) = product.triple();
        let dirs = ProjectDirs::from(qualifier, organization, application).with_context(|| {
            format!("platform exposes no application directories for {product}")
        })?;
        Ok(Self {
            config: dirs.config_dir().to_path_buf(),
            data: dirs.data_dir().to_path_buf(),
            local_data: dirs.data_local_dir().to_path_buf(),
            cache: dirs.cache_dir().to_path_buf(),
            state: dirs
                .state_dir()
                .map_or_else(|| dirs.data_local_dir().join("state"), Path::to_path_buf),
            runtime: dirs.runtime_dir().map(Path::to_path_buf),
        })
    }

    /// Resolve the directories for the platform the process entered through.
    ///
    /// A platform that owns storage roots every directory beneath the private
    /// root it hands the application; otherwise the user's home applies.
    ///
    /// # Errors
    ///
    /// Returns an error when the platform exposes no application directories.
    pub fn claim_from(product: ProductIdentity, ingress: &Ingress) -> Result<Self> {
        let Some(root) = ingress.private_root() else {
            return Self::claim(product);
        };
        Ok(Self {
            config: root.join("config"),
            data: root.join("data"),
            local_data: root.join("data"),
            cache: root.join("cache"),
            state: root.join("state"),
            runtime: None,
        })
    }

    /// Create the config, data, local data, cache, and state roots.
    ///
    /// # Errors
    ///
    /// Returns the first directory that could not be created.
    pub fn prepare(&self) -> Result<()> {
        for path in [
            &self.config,
            &self.data,
            &self.local_data,
            &self.cache,
            &self.state,
        ] {
            std::fs::create_dir_all(path).with_context(|| format!("create {}", path.display()))?;
        }
        Ok(())
    }
}
