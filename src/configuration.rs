//! Strict, format-preserving TOML configuration outside the event-loop thread.

#![deny(missing_docs)]

use std::{
    collections::BTreeSet,
    fmt::{self, Display, Formatter},
    fs,
    io::{self, Write as _},
    path::{Path, PathBuf},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use anyhow::{Context as _, Result, bail, ensure};
use crossbeam_channel::{Receiver, TryRecvError, bounded};
use serde::{Serialize, de::DeserializeOwned};
use toml_edit::{Document, DocumentMut, Item, Table};

use crate::{NativeWake, ScribeOutcome, SettledScribe};

/// Typed application configuration admitted by [`ConfigurationLedger`].
///
/// Deserialization supplies the schema. The ledger independently rejects every
/// key ignored by Serde, so callers cannot accidentally admit misspellings by
/// omitting `deny_unknown_fields`. Implement [`Self::validate`] only for
/// semantic laws not expressible in the Rust representation.
pub trait Configuration:
    Clone + Default + PartialEq + Serialize + DeserializeOwned + Send + 'static
{
    /// Reject a structurally valid value that violates a product invariant.
    fn validate(&self) -> std::result::Result<(), String> {
        Ok(())
    }
}

/// A configuration condition that requires user attention.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConfigurationFault {
    message: String,
}

impl ConfigurationFault {
    fn forge(error: impl Display) -> Self {
        Self {
            message: error.to_string(),
        }
    }

    /// Exact actionable fault detail.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl Display for ConfigurationFault {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        self.message.fmt(formatter)
    }
}

impl std::error::Error for ConfigurationFault {}

struct Inscription<T> {
    expected: T,
    desired: T,
}

struct Submitted<T> {
    sequence: u64,
    desired: T,
}

struct Reload<T> {
    result: Receiver<std::result::Result<T, String>>,
    thread: JoinHandle<()>,
}

/// A strict, settled, round-trip-safe TOML preference ledger.
///
/// Startup and reload validate on a worker boundary. Mutations settle through
/// [`SettledScribe`]. Each write rereads the file, rejects invalid or unknown
/// keys, performs a per-key optimistic merge, replaces existing scalar source
/// spans without regenerating surrounding text, follows an existing symlink,
/// and commits through `atomic-write-file`.
pub struct ConfigurationLedger<T: Configuration> {
    path: PathBuf,
    live: T,
    durable: T,
    scribe: SettledScribe<Inscription<T>>,
    submitted: Option<Submitted<T>>,
    reload: Option<Reload<T>>,
    fault: Option<ConfigurationFault>,
    wake: NativeWake,
}

impl<T: Configuration> ConfigurationLedger<T> {
    /// Restore a configuration file, using `T::default()` when it does not yet
    /// exist.
    pub fn raise(
        name: impl Into<String>,
        ctx: &egui::Context,
        path: PathBuf,
        settle: Duration,
    ) -> Result<Self> {
        Self::raise_with_fallback(name, ctx, path, settle, T::default())
    }

    /// Restore a configuration file with a product-supplied value used only
    /// when the file does not yet exist.
    ///
    /// This is the migration seam for a prior lawful preference store. A
    /// nondefault fallback is marked dirty and reaches the new TOML file
    /// through the ordinary settlement and atomic-write law.
    pub fn raise_with_fallback(
        name: impl Into<String>,
        ctx: &egui::Context,
        path: PathBuf,
        settle: Duration,
        fallback: T,
    ) -> Result<Self> {
        fallback
            .validate()
            .map_err(anyhow::Error::msg)
            .context("validate configuration fallback")?;
        let restored = load::<T>(&path);
        let (live, durable, fault, migrate) = match restored {
            Ok(Loaded::Absent) => {
                let migrate = fallback != T::default();
                (fallback, T::default(), None, migrate)
            }
            Ok(Loaded::Present(value)) => (value.clone(), value, None, false),
            Err(error) => (
                T::default(),
                T::default(),
                Some(ConfigurationFault::forge(error)),
                false,
            ),
        };
        let worker_path = path.clone();
        let scribe =
            SettledScribe::spawn(name, ctx, settle, move |inscription: Inscription<T>| {
                merge_and_write(&worker_path, &inscription.expected, &inscription.desired)
            })?;
        let mut ledger = Self {
            path,
            live,
            durable,
            scribe,
            submitted: None,
            reload: None,
            fault,
            wake: NativeWake::from_context(ctx),
        };
        if migrate {
            ledger.scribe.mark();
        }
        Ok(ledger)
    }

    /// Current typed configuration applied by the application.
    #[must_use]
    pub const fn live(&self) -> &T {
        &self.live
    }

    /// Platform-correct configuration path selected by the application.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Current blocking read, validation, conflict, or write condition.
    #[must_use]
    pub const fn fault(&self) -> Option<&ConfigurationFault> {
        self.fault.as_ref()
    }

    /// Whether the ledger currently admits application mutations.
    #[must_use]
    pub const fn writable(&self) -> bool {
        self.fault.is_none() && self.reload.is_none()
    }

    /// Whether a background reread is in flight.
    #[must_use]
    pub const fn reload_pending(&self) -> bool {
        self.reload.is_some()
    }

    /// Whether no application mutation or write remains unsettled.
    #[must_use]
    pub fn settled(&self) -> bool {
        self.live == self.durable
            && self.submitted.is_none()
            && self.scribe.deadline().is_none()
            && self.reload.is_none()
    }

    /// Apply one typed mutation and restart the settlement clock.
    pub fn revise(&mut self, revision: impl FnOnce(&mut T)) -> Result<bool> {
        ensure!(self.writable(), "configuration is blocked pending repair");
        let mut desired = self.live.clone();
        revision(&mut desired);
        desired
            .validate()
            .map_err(anyhow::Error::msg)
            .context("validate revised configuration")?;
        if desired == self.live {
            return Ok(false);
        }
        self.live = desired;
        self.scribe.mark();
        Ok(true)
    }

    /// Next settlement deadline, independent of rendering.
    #[must_use]
    pub fn deadline(&self) -> Option<Instant> {
        (self.writable() && self.submitted.is_none())
            .then(|| self.scribe.deadline())
            .flatten()
    }

    /// Advance every matured persistence obligation without performing I/O on
    /// the event-loop thread.
    ///
    /// Returns whether the visible configuration condition changed.
    pub fn service_deadline_reached(&mut self, now: Instant) -> bool {
        let mut changed = self.absorb();
        if self.deadline().is_none_or(|deadline| deadline > now) {
            return changed;
        }
        let desired = self.live.clone();
        let inscription = Inscription {
            expected: self.durable.clone(),
            desired: desired.clone(),
        };
        match self.scribe.tend(now, || inscription) {
            Ok(Some(sequence)) => {
                self.submitted = Some(Submitted { sequence, desired });
            }
            Ok(None) => {}
            Err(error) => {
                self.fault = Some(ConfigurationFault::forge(format!(
                    "Could not submit the configuration write: {error:#}"
                )));
                changed = true;
            }
        }
        changed
    }

    /// Start an explicit background reread.
    ///
    /// A successful reload replaces both live and durable values. Callers
    /// should ask for confirmation before invoking this while unsettled.
    pub fn request_reload(&mut self) -> Result<bool> {
        if self.reload.is_some() {
            return Ok(false);
        }
        ensure!(
            self.fault.is_some() || self.settled(),
            "configuration has unsettled application changes"
        );
        let path = self.path.clone();
        let wake = self.wake.clone();
        let (publish, result) = bounded(1);
        let thread = thread::Builder::new()
            .name("configuration-reload".to_owned())
            .spawn(move || {
                let loaded = load::<T>(&path).map(|loaded| match loaded {
                    Loaded::Absent => T::default(),
                    Loaded::Present(value) => value,
                });
                let _sent = publish.send(loaded.map_err(|error| format!("{error:#}")));
                let _woken = wake.request_repaint();
            })
            .context("spawn configuration reload")?;
        self.reload = Some(Reload { result, thread });
        Ok(true)
    }

    /// Absorb completed writes and reloads without waiting.
    ///
    /// Returns whether the visible value or fault condition changed.
    pub fn absorb(&mut self) -> bool {
        let mut changed = false;
        if let Some(outcome) = self.scribe.take_outcome() {
            match outcome {
                ScribeOutcome::Saved { sequence } => {
                    if self
                        .submitted
                        .as_ref()
                        .is_some_and(|submitted| submitted.sequence == sequence)
                    {
                        let submitted = self.submitted.take().expect("matching submission exists");
                        self.durable = submitted.desired;
                    }
                }
                ScribeOutcome::Fault { message, .. } => {
                    self.fault = Some(ConfigurationFault::forge(format!(
                        "Could not save the configuration: {message}"
                    )));
                    changed = true;
                }
            }
        }
        changed | self.absorb_reload()
    }

    fn absorb_reload(&mut self) -> bool {
        let result = match self.reload.as_ref().map(|reload| reload.result.try_recv()) {
            None | Some(Err(TryRecvError::Empty)) => return false,
            Some(Ok(result)) => result,
            Some(Err(TryRecvError::Disconnected)) => {
                Err("configuration reload worker fell before publication".to_owned())
            }
        };
        let reload = self.reload.take().expect("reload result has an owner");
        if reload.thread.join().is_err() {
            self.fault = Some(ConfigurationFault::forge(
                "Configuration reload worker panicked",
            ));
            return true;
        }
        match result {
            Ok(value) => {
                self.live = value.clone();
                self.durable = value;
                self.submitted = None;
                self.scribe.clear();
                self.fault = None;
            }
            Err(message) => self.fault = Some(ConfigurationFault::forge(message)),
        }
        true
    }
}

impl<T: Configuration> Drop for ConfigurationLedger<T> {
    fn drop(&mut self) {
        if let Some(reload) = self.reload.take() {
            let _joined = reload.thread.join();
        }
        if self.fault.is_none() && (self.live != self.durable || self.submitted.is_some()) {
            let inscription = Inscription {
                expected: self.durable.clone(),
                desired: self.live.clone(),
            };
            if let Err(error) = self.scribe.flush(inscription) {
                eprintln!("could not retire configuration cleanly: {error:#}");
            }
        }
    }
}

enum Loaded<T> {
    Absent,
    Present(T),
}

fn load<T: Configuration>(path: &Path) -> Result<Loaded<T>> {
    match fs::read_to_string(path) {
        Ok(source) => strict_decode(&source).map(Loaded::Present),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Loaded::Absent),
        Err(error) => Err(error).with_context(|| format!("read `{}`", path.display())),
    }
}

fn strict_decode<T: Configuration>(source: &str) -> Result<T> {
    if source.is_empty() {
        let value = T::default();
        value.validate().map_err(anyhow::Error::msg)?;
        return Ok(value);
    }
    let deserializer =
        toml_edit::de::Deserializer::parse(source).context("parse TOML configuration")?;
    let mut unknown = Vec::new();
    let value: T = serde_ignored::deserialize(deserializer, |path| unknown.push(path.to_string()))
        .context("decode typed configuration")?;
    if !unknown.is_empty() {
        unknown.sort();
        unknown.dedup();
        bail!(
            "Unknown configuration {}: {}",
            if unknown.len() == 1 { "key" } else { "keys" },
            unknown.join(", ")
        );
    }
    value.validate().map_err(anyhow::Error::msg)?;
    Ok(value)
}

#[derive(Clone)]
struct Patch {
    path: Vec<String>,
    expected: Option<Item>,
    desired: Option<Item>,
}

fn merge_and_write<T: Configuration>(path: &Path, expected: &T, desired: &T) -> Result<()> {
    let source = match fs::read_to_string(path) {
        Ok(source) => source,
        Err(error) if error.kind() == io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(error).with_context(|| format!("read `{}`", path.display())),
    };
    let current = strict_decode::<T>(&source)?;
    let expected =
        toml_edit::ser::to_document(expected).context("project expected configuration")?;
    let desired = toml_edit::ser::to_document(desired).context("project desired configuration")?;
    let current = toml_edit::ser::to_document(&current).context("project current configuration")?;
    let mut patches = Vec::new();
    diff_tables(&[], expected.as_table(), desired.as_table(), &mut patches);
    let mut merged = source.clone();
    for patch in patches {
        let disk = item_at(&current, &patch.path);
        let expected = patch.expected.as_ref();
        let desired = patch.desired.as_ref();
        if same_item(disk, desired) {
            continue;
        }
        ensure!(
            same_item(disk, expected),
            "Configuration changed concurrently at `{}`; reload before replacing it",
            patch.path.join(".")
        );
        merged = apply_patch(&merged, &patch)?;
    }
    let _validated = strict_decode::<T>(&merged)?;
    if merged == source {
        return Ok(());
    }
    replace(path, merged.as_bytes())
}

fn diff_tables(prefix: &[String], left: &Table, right: &Table, patches: &mut Vec<Patch>) {
    let keys = left
        .iter()
        .map(|(key, _)| key.to_owned())
        .chain(right.iter().map(|(key, _)| key.to_owned()))
        .collect::<BTreeSet<_>>();
    for key in keys {
        let left = left.get(&key);
        let right = right.get(&key);
        let mut path = prefix.to_vec();
        path.push(key);
        match (
            left.and_then(Item::as_table),
            right.and_then(Item::as_table),
        ) {
            (Some(left), Some(right)) => diff_tables(&path, left, right, patches),
            _ if same_item(left, right) => {}
            _ => patches.push(Patch {
                path,
                expected: left.cloned(),
                desired: right.cloned(),
            }),
        }
    }
}

fn same_item(left: Option<&Item>, right: Option<&Item>) -> bool {
    match (left, right) {
        (None, None) => true,
        (Some(left), Some(right)) => left.to_string() == right.to_string(),
        (None, Some(_)) | (Some(_), None) => false,
    }
}

fn item_at<'a>(document: &'a DocumentMut, path: &[String]) -> Option<&'a Item> {
    item_at_root(document.as_item(), path)
}

fn item_at_root<'a>(mut item: &'a Item, path: &[String]) -> Option<&'a Item> {
    for key in path {
        item = item.get(key)?;
    }
    Some(item)
}

fn apply_patch(source: &str, patch: &Patch) -> Result<String> {
    let document =
        Document::parse(source.to_owned()).context("parse source-preserving TOML document")?;
    let existing = item_at_root(document.as_item(), &patch.path);
    if let (Some(existing), Some(desired)) = (existing, patch.desired.as_ref())
        && let (Some(span), Some(value)) = (existing.span(), desired.as_value())
    {
        let mut replaced = source.to_owned();
        replaced.replace_range(span, &value.to_string());
        return Ok(replaced);
    }
    let mut document = document.into_mut();
    let (name, parent) = patch
        .path
        .split_last()
        .context("configuration patch has no key")?;
    let table = table_mut(document.as_table_mut(), parent)?;
    if let Some(desired) = &patch.desired {
        table.insert(name, desired.clone());
    } else {
        table.remove(name);
    }
    Ok(document.to_string())
}

fn table_mut<'a>(mut table: &'a mut Table, path: &[String]) -> Result<&'a mut Table> {
    for key in path {
        let item = table
            .entry(key)
            .or_insert_with(|| Item::Table(Table::new()));
        table = item
            .as_table_mut()
            .with_context(|| format!("`{key}` is not a table"))?;
    }
    Ok(table)
}

fn replace(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path.parent().context("configuration path has no parent")?;
    let mut directory = fs::DirBuilder::new();
    directory.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt as _;
        directory.mode(0o700);
    }
    directory
        .create(parent)
        .with_context(|| format!("create `{}`", parent.display()))?;
    let target = match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => path
            .canonicalize()
            .with_context(|| format!("resolve configuration symlink `{}`", path.display()))?,
        Ok(_) => path.to_owned(),
        Err(error) if error.kind() == io::ErrorKind::NotFound => path.to_owned(),
        Err(error) => {
            return Err(error).with_context(|| format!("inspect `{}`", path.display()));
        }
    };
    let mut options = atomic_write_file::OpenOptions::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    let mut file = options
        .open(&target)
        .with_context(|| format!("stage `{}`", target.display()))?;
    file.write_all(bytes)
        .with_context(|| format!("write `{}`", target.display()))?;
    file.commit()
        .with_context(|| format!("commit `{}`", target.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use serde::Deserialize;

    #[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
    #[serde(default)]
    struct Specimen {
        enabled: bool,
        level: u8,
    }

    impl Configuration for Specimen {
        fn validate(&self) -> std::result::Result<(), String> {
            (self.level <= 9)
                .then_some(())
                .ok_or_else(|| "level must not exceed 9".to_owned())
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(128))]

        #[test]
        fn strict_surgical_merge_preserves_unrelated_source(
            heading in "[a-zA-Z0-9 ]{0,40}",
            tail in "[a-zA-Z0-9 ]{0,40}",
            garbage in any::<bool>(),
        ) {
            let extra = if garbage { "garbage = 3\n" } else { "" };
            let source = format!(
                "# {heading}\nenabled  =  false # crown\n\n# {tail}\nlevel = 4\n{extra}"
            );
            let expected = Specimen { enabled: false, level: 4 };
            let desired = Specimen { enabled: true, level: 4 };
            if garbage {
                prop_assert!(strict_decode::<Specimen>(&source).is_err());
            } else {
                let path = tempfile::NamedTempFile::new()?.into_temp_path();
                fs::write(&path, &source)?;
                merge_and_write(&path, &expected, &desired).expect("merge configuration");
                let actual = fs::read_to_string(&path).expect("read merged configuration");
                prop_assert_eq!(
                    actual,
                    format!(
                        "# {heading}\nenabled  =  true # crown\n\n# {tail}\nlevel = 4\n"
                    )
                );
            }
        }
    }
}
