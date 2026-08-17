//! A persistent, one-level shelved collection with physical Poolrooms controls.
//!
//! The application owns entry meaning, active-document policy, storage, and
//! action interpretation. The cabinet owns global entry identity, ordering,
//! folders, entry and folder drag berths, optional entry renaming, folder
//! editing, and their common interaction grammar.

use std::{
    any::Any,
    collections::{BTreeSet, HashSet},
    fmt::{Debug, Display},
    hash::Hash,
    sync::Arc,
};

use brass_poolrooms::{
    chrome::{self, Coupled, CouplingGap, DragHandle, MechanismSize, Monoglyph, Symbol},
    water::Surface,
};

/// Identity admitted by a [`Cabinet`].
///
/// Keys are globally unique across loose entries and every shelf. `forge`
/// performs the product's textual refinement; `as_str` supplies stable drag
/// and witness identity.
pub trait CabinetKey: Clone + Debug + Display + Eq + Hash + Send + Sync + 'static {
    /// Refine user-authored text into an identity, rejecting emptiness.
    fn forge(raw: &str) -> Option<Self>;

    /// Borrow the canonical textual identity.
    fn as_str(&self) -> &str;
}

/// An entry whose identity and placement are governed by a [`Cabinet`].
pub trait CabinetEntry: Clone + 'static {
    /// The product's refined entry identity.
    type Key: CabinetKey;

    /// Borrow this entry's identity.
    fn key(&self) -> &Self::Key;

    /// Replace this entry's identity during rename or corrupt-input repair.
    fn rename(&mut self, key: Self::Key);

    /// Optional compact product mark shown beside the entry name.
    fn sigil(&self) -> Option<char> {
        None
    }
}

/// Destination of a dragged cabinet entry.
#[derive(Clone, Debug)]
pub enum Berth<K> {
    /// Before or after another entry in that entry's present container.
    Beside { anchor: K, after: bool },
    /// At the end of one shelf.
    Shelf(usize),
    /// At the end of the loose root collection.
    Root,
}

/// Destination of a dragged cabinet shelf.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ShelfBerth {
    /// Shelf whose present position anchors the destination.
    pub anchor: usize,
    /// Whether the dragged shelf lands after rather than before the anchor.
    pub after: bool,
}

/// One named cabinet shelf.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default, deny_unknown_fields))]
#[derive(Clone, Debug, PartialEq)]
pub struct Shelf<T> {
    /// User-authored shelf name, unique within the cabinet after rectification.
    pub name: String,
    /// Running fold state. Persistence remains an application decision.
    #[cfg_attr(feature = "serde", serde(skip, default = "shelf_open"))]
    pub open: bool,
    /// Entries presently moored to this shelf.
    pub entries: Vec<T>,
}

impl<T> Default for Shelf<T> {
    fn default() -> Self {
        Self {
            name: String::new(),
            open: true,
            entries: Vec::new(),
        }
    }
}

#[cfg(feature = "serde")]
const fn shelf_open() -> bool {
    true
}

/// An ordered collection of globally named entries, optionally grouped one
/// level deep into shelves.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default, deny_unknown_fields))]
#[derive(Clone, Debug, PartialEq)]
pub struct Cabinet<T> {
    /// Entries outside any shelf, in user order.
    pub saved: Vec<T>,
    /// Named shelves, in user order.
    pub shelves: Vec<Shelf<T>>,
}

impl<T> Default for Cabinet<T> {
    fn default() -> Self {
        Self {
            saved: Vec::new(),
            shelves: Vec::new(),
        }
    }
}

impl<T: CabinetEntry> Cabinet<T> {
    /// Construct and rectify a cabinet from an application persistence
    /// projection. No entry is discarded: duplicate identities receive the
    /// first free numeric suffix.
    #[must_use]
    pub fn forge(saved: Vec<T>, shelves: Vec<Shelf<T>>) -> Self {
        let mut cabinet = Self { saved, shelves };
        cabinet.rectify();
        cabinet
    }

    /// Restore the cabinet invariants after deserializing application-owned
    /// storage: globally unique entry keys and normalized, unique shelf names.
    pub fn rectify(&mut self) {
        let mut keys = HashSet::new();
        for entry in &mut self.saved {
            rectify_entry(entry, &mut keys);
        }
        let mut shelf_names = HashSet::new();
        for shelf in &mut self.shelves {
            shelf.name = spare_shelf_name(&shelf_names, &shelf.name);
            let _unique = shelf_names.insert(shelf.name.clone());
            for entry in &mut shelf.entries {
                rectify_entry(entry, &mut keys);
            }
        }
    }

    /// Iterate over loose and shelved entries in presentation order.
    pub fn all(&self) -> impl Iterator<Item = &T> {
        self.saved
            .iter()
            .chain(self.shelves.iter().flat_map(|shelf| shelf.entries.iter()))
    }

    /// Mutably iterate over loose and shelved entries in presentation order.
    pub fn all_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.saved.iter_mut().chain(
            self.shelves
                .iter_mut()
                .flat_map(|shelf| shelf.entries.iter_mut()),
        )
    }

    /// Resolve one globally named entry.
    pub fn get(&self, key: &T::Key) -> Option<&T> {
        self.all().find(|entry| entry.key() == key)
    }

    /// Resolve one globally named entry mutably.
    pub fn get_mut(&mut self, key: &T::Key) -> Option<&mut T> {
        self.find_mut(key)
    }

    /// Whether this key is already occupied anywhere in the cabinet.
    pub fn taken(&self, key: &T::Key) -> bool {
        self.get(key).is_some()
    }

    /// Retain an active key only while its entry still exists.
    pub fn active(&self, active: Option<T::Key>) -> Option<T::Key> {
        active.filter(|key| self.taken(key))
    }

    /// Replace an identically named entry in place, or append a new entry to
    /// the loose collection.
    pub fn upsert(&mut self, entry: T) {
        match self.find_mut(entry.key()) {
            Some(slot) => *slot = entry,
            None => self.saved.push(entry),
        }
    }

    /// Remove one entry without disturbing the order of its siblings.
    pub fn remove(&mut self, key: &T::Key) -> Option<T> {
        if let Some(slot) = self.saved.iter().position(|entry| entry.key() == key) {
            return Some(self.saved.remove(slot));
        }
        self.shelves.iter_mut().find_map(|shelf| {
            shelf
                .entries
                .iter()
                .position(|entry| entry.key() == key)
                .map(|slot| shelf.entries.remove(slot))
        })
    }

    /// Rename one entry in place, rejecting a vanished source or occupied key.
    /// Returns whether the rename was admitted.
    pub fn rename(&mut self, old: &T::Key, new: T::Key) -> bool {
        if old != &new && self.taken(&new) {
            return false;
        }
        let Some(entry) = self.find_mut(old) else {
            return false;
        };
        entry.rename(new);
        true
    }

    /// Move an entry to a name-anchored berth. A vanished destination falls
    /// back to the loose collection; a vanished source is a no-op.
    pub fn moor(&mut self, key: &T::Key, berth: &Berth<T::Key>) {
        if let Berth::Beside { anchor, .. } = berth
            && anchor == key
        {
            return;
        }
        let Some(entry) = self.remove(key) else {
            return;
        };
        match berth {
            Berth::Beside { anchor, after } => {
                let slip = usize::from(*after);
                match self.berth_of(anchor) {
                    Some((None, slot)) => self.saved.insert(slot + slip, entry),
                    Some((Some(rack), slot)) => {
                        self.shelves[rack].entries.insert(slot + slip, entry);
                    }
                    None => self.saved.push(entry),
                }
            }
            Berth::Shelf(rack) => match self.shelves.get_mut(*rack) {
                Some(rack) => rack.entries.push(entry),
                None => self.saved.push(entry),
            },
            Berth::Root => self.saved.push(entry),
        }
    }

    /// Move one shelf before or after another shelf. Invalid or reflexive
    /// berths are no-ops.
    pub fn moor_shelf(&mut self, source: usize, berth: ShelfBerth) {
        if source == berth.anchor
            || source >= self.shelves.len()
            || berth.anchor >= self.shelves.len()
        {
            return;
        }
        let moving = self.shelves.remove(source);
        let anchor = berth.anchor - usize::from(source < berth.anchor);
        self.shelves
            .insert(anchor + usize::from(berth.after), moving);
    }

    /// Add an empty, uniquely named shelf.
    pub fn add_shelf(&mut self) {
        let names = self
            .shelves
            .iter()
            .map(|shelf| shelf.name.clone())
            .collect::<HashSet<_>>();
        self.shelves.push(Shelf {
            name: spare_shelf_name(&names, "folder"),
            ..Shelf::default()
        });
    }

    /// Toggle one shelf when it still exists.
    pub fn toggle_shelf(&mut self, index: usize) {
        if let Some(rack) = self.shelves.get_mut(index) {
            rack.open = !rack.open;
        }
    }

    /// Restore fold state from the application persistence projection.
    pub fn restore_folds(&mut self, closed: &BTreeSet<String>) {
        for shelf in &mut self.shelves {
            shelf.open = !closed.contains(&shelf.name);
        }
    }

    /// Project the closed shelf names for application persistence.
    #[must_use]
    pub fn closed_shelves(&self) -> BTreeSet<String> {
        self.shelves
            .iter()
            .filter(|shelf| !shelf.open)
            .map(|shelf| shelf.name.clone())
            .collect()
    }

    /// Insert a new entry immediately after an existing entry, preserving its
    /// container. A vanished anchor falls back to the loose collection.
    pub fn adopt_beside(&mut self, anchor: &T::Key, entry: T) {
        match self.berth_of(anchor) {
            Some((None, slot)) => self.saved.insert(slot + 1, entry),
            Some((Some(rack), slot)) => self.shelves[rack].entries.insert(slot + 1, entry),
            None => self.saved.push(entry),
        }
    }

    /// Delete one shelf and spill its entries, in order, into the loose tail.
    pub fn scuttle_shelf(&mut self, index: usize) {
        if index < self.shelves.len() {
            let rack = self.shelves.remove(index);
            self.saved.extend(rack.entries);
        }
    }

    /// Normalize and commit a shelf rename unless another shelf already owns
    /// the result. Returns whether the edit was admitted.
    pub fn rename_shelf(&mut self, index: usize, raw: &str) -> bool {
        let name = normalize_shelf_name(raw);
        if self
            .shelves
            .iter()
            .enumerate()
            .any(|(slot, candidate)| slot != index && candidate.name == name)
        {
            return false;
        }
        let Some(rack) = self.shelves.get_mut(index) else {
            return false;
        };
        rack.name = name;
        true
    }

    /// Derive the first free numeric variant of a desired key.
    pub fn spare_named(&self, base: &T::Key) -> T::Key {
        if !self.taken(base) {
            return base.clone();
        }
        let mut suffix = 2_u64;
        loop {
            let raw = format!("{} {suffix}", base.as_str());
            if let Some(candidate) = T::Key::forge(&raw)
                && !self.taken(&candidate)
            {
                return candidate;
            }
            suffix = suffix.saturating_add(1);
        }
    }

    /// Render the common cabinet body and return application-owned semantic
    /// actions. The scope becomes stable drag and witness identity.
    pub fn show(
        &self,
        ui: &mut egui::Ui,
        water: &mut Surface,
        scope: &'static str,
        noun: &'static str,
        active: Option<&T::Key>,
        shelf_edit: &mut Option<ShelfEdit>,
    ) -> Vec<CabinetAction<T>> {
        self.show_with_entry_renaming(
            ui,
            water,
            scope,
            noun,
            active,
            shelf_edit,
            &mut EntryRenaming::Disabled,
        )
    }

    /// Render a cabinet whose entries admit inline renaming.
    ///
    /// The edit is presentation state retained by the caller between frames.
    /// A valid commit becomes [`CabinetAction::RenameEntry`]; Cabinet never
    /// mutates or persists the application projection during layout.
    #[expect(
        clippy::too_many_arguments,
        reason = "the opt-in projection adds only its retained edit state to the ordinary cabinet surface"
    )]
    pub fn show_renamable(
        &self,
        ui: &mut egui::Ui,
        water: &mut Surface,
        scope: &'static str,
        noun: &'static str,
        active: Option<&T::Key>,
        shelf_edit: &mut Option<ShelfEdit>,
        entry_edit: &mut Option<EntryEdit<T::Key>>,
    ) -> Vec<CabinetAction<T>> {
        self.show_with_entry_renaming(
            ui,
            water,
            scope,
            noun,
            active,
            shelf_edit,
            &mut EntryRenaming::Enabled(entry_edit),
        )
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "the internal renderer receives the complete explicit cabinet projection"
    )]
    fn show_with_entry_renaming(
        &self,
        ui: &mut egui::Ui,
        water: &mut Surface,
        scope: &'static str,
        noun: &'static str,
        active: Option<&T::Key>,
        shelf_edit: &mut Option<ShelfEdit>,
        entry_renaming: &mut EntryRenaming<'_, T::Key>,
    ) -> Vec<CabinetAction<T>> {
        let mut actions = Vec::new();
        for entry in &self.saved {
            entry_row(
                self,
                ui,
                water,
                scope,
                noun,
                active,
                entry,
                entry_renaming,
                &mut actions,
            );
        }
        root_berth(self, ui, scope, &mut actions);
        for (slot, shelf) in self.shelves.iter().enumerate() {
            shelf_rows(
                self,
                ui,
                water,
                scope,
                noun,
                slot,
                shelf,
                active,
                shelf_edit,
                entry_renaming,
                &mut actions,
            );
        }
        let _controls = ui.horizontal_wrapped(|ui| {
            let add = Monoglyph::symbol(Symbol::Add)
                .show(ui)
                .on_hover_text("new folder");
            water.monoglyph(&add);
            record(ui, format!("cabinet.{scope}.new-shelf"), add.interact_rect);
            if add.clicked() {
                actions.push(CabinetAction::NewShelf);
            }
            let _label = ui.label(chrome::section_title("NEW FOLDER"));
        });
        actions
    }

    fn find_mut(&mut self, key: &T::Key) -> Option<&mut T> {
        self.saved
            .iter_mut()
            .chain(
                self.shelves
                    .iter_mut()
                    .flat_map(|shelf| shelf.entries.iter_mut()),
            )
            .find(|entry| entry.key() == key)
    }

    fn berth_of(&self, key: &T::Key) -> Option<(Option<usize>, usize)> {
        if let Some(slot) = self.saved.iter().position(|entry| entry.key() == key) {
            return Some((None, slot));
        }
        self.shelves.iter().enumerate().find_map(|(shelf, rack)| {
            rack.entries
                .iter()
                .position(|entry| entry.key() == key)
                .map(|slot| (Some(shelf), slot))
        })
    }

    fn is_shelved(&self, key: &T::Key) -> bool {
        matches!(self.berth_of(key), Some((Some(_), _)))
    }
}

/// Application-owned consequences emitted by [`Cabinet::show`].
#[derive(Clone, Debug)]
pub enum CabinetAction<T: CabinetEntry> {
    /// Activate a cloned entry value.
    Load(T),
    /// Clone the named entry.
    Clone(T::Key),
    /// Delete the named entry.
    Delete(T::Key),
    /// Rename an entry after textual refinement and collision checks.
    RenameEntry { from: T::Key, to: T::Key },
    /// Re-home the named entry.
    Moor { key: T::Key, berth: Berth<T::Key> },
    /// Reorder one shelf around another shelf.
    MoorShelf { shelf: usize, berth: ShelfBerth },
    /// Add an empty shelf.
    NewShelf,
    /// Toggle one shelf fold.
    ToggleShelf(usize),
    /// Delete one shelf and spill its entries.
    ScuttleShelf(usize),
    /// Begin editing one shelf name.
    BeginShelfRename(usize),
    /// Commit the current [`ShelfEdit`].
    CommitShelfRename,
}

/// In-flight shelf-name edit retained by the application between frames.
#[derive(Clone, Debug)]
pub struct ShelfEdit {
    /// Edited shelf index.
    pub shelf: usize,
    /// Running user-authored name.
    pub name: String,
    /// Whether the next frame must acquire keyboard focus.
    pub focus: bool,
}

/// In-flight entry-name edit retained between calls to
/// [`Cabinet::show_renamable`].
///
/// Cabinet owns the edit's internal state; applications need only retain an
/// `Option<EntryEdit<_>>` alongside their other ephemeral UI state.
#[derive(Clone, Debug)]
pub struct EntryEdit<K> {
    key: K,
    name: String,
    focus: bool,
    fault: Option<EntryNameFault>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EntryNameFault {
    Invalid,
    Occupied,
}

enum EntryRenaming<'a, K> {
    Disabled,
    Enabled(&'a mut Option<EntryEdit<K>>),
}

impl<K> EntryRenaming<'_, K> {
    const fn enabled(&self) -> bool {
        matches!(self, Self::Enabled(_))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ShelfDrag(usize);

#[expect(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    reason = "one entry row is one cohesive physical assembly with complete explicit interaction state"
)]
fn entry_row<T: CabinetEntry>(
    cabinet: &Cabinet<T>,
    ui: &mut egui::Ui,
    water: &mut Surface,
    scope: &'static str,
    noun: &'static str,
    active: Option<&T::Key>,
    entry: &T,
    entry_renaming: &mut EntryRenaming<'_, T::Key>,
    actions: &mut Vec<CabinetAction<T>>,
) {
    let row = ui.horizontal(|ui| {
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
        let key = entry.key();
        let drag = |ui: &mut egui::Ui| {
            ui.push_id((scope, "drag", key.as_str()), |ui| {
                DragHandle::friction_pad()
                    .size(MechanismSize::Small)
                    .show(ui)
                    .on_hover_text("drag to rearrange")
            })
            .inner
        };
        let delete = |ui: &mut egui::Ui| {
            Monoglyph::symbol(Symbol::Remove)
                .size(MechanismSize::Small)
                .show(ui)
                .on_hover_text(format!("delete {noun}"))
        };
        let clone = |ui: &mut egui::Ui| {
            Monoglyph::symbol(Symbol::Duplicate)
                .size(MechanismSize::Small)
                .show(ui)
                .on_hover_text(format!("clone {noun}"))
        };
        let (drag, rename, delete, clone) = if entry_renaming.enabled() {
            let assembly = Coupled::horizontal_with_gap(ui, CouplingGap::MINIMUM, drag, |ui| {
                Coupled::horizontal_with_gap(
                    ui,
                    CouplingGap::MINIMUM,
                    |ui| {
                        Monoglyph::symbol(Symbol::Rename)
                            .size(MechanismSize::Small)
                            .show(ui)
                            .on_hover_text(format!("rename {noun}"))
                    },
                    |ui| Coupled::horizontal_with_gap(ui, CouplingGap::MINIMUM, delete, clone),
                )
            });
            (
                assembly.left,
                Some(assembly.right.left),
                assembly.right.right.left,
                assembly.right.right.right,
            )
        } else {
            let assembly = Coupled::horizontal_with_gap(ui, CouplingGap::MINIMUM, drag, |ui| {
                Coupled::horizontal_with_gap(ui, CouplingGap::MINIMUM, delete, clone)
            });
            (
                assembly.left,
                None,
                assembly.right.left,
                assembly.right.right,
            )
        };
        water.drag_handle(&drag);
        if let Some(rename) = &rename {
            water.monoglyph(rename);
            record(
                ui,
                format!("cabinet.{scope}.rename/{}", key.as_str()),
                rename.interact_rect,
            );
        }
        water.monoglyph(&delete);
        water.monoglyph(&clone);
        drag.dnd_set_drag_payload(key.clone());
        if delete.clicked() {
            actions.push(CabinetAction::Delete(key.clone()));
        }
        record(
            ui,
            format!("cabinet.{scope}.clone/{}", key.as_str()),
            clone.interact_rect,
        );
        if clone.clicked() {
            actions.push(CabinetAction::Clone(key.clone()));
        }
        if delete.clicked() || clone.clicked() || drag.drag_started() {
            cancel_entry_edit(entry_renaming, key);
        } else if rename
            .as_ref()
            .is_some_and(chrome::MonoglyphResponse::clicked)
            && let EntryRenaming::Enabled(edit) = entry_renaming
        {
            **edit = Some(EntryEdit {
                key: key.clone(),
                name: key.as_str().to_owned(),
                focus: true,
                fault: None,
            });
        }
        entry_identity(
            cabinet,
            ui,
            water,
            scope,
            active,
            entry,
            entry_renaming,
            actions,
        );
    });
    let rect = row.response.rect;
    let after = ui
        .ctx()
        .pointer_latest_pos()
        .is_some_and(|position| position.y > rect.center().y);
    if let Some(payload) = row.response.dnd_hover_payload::<T::Key>()
        && *payload != *entry.key()
    {
        let y = if after { rect.bottom() } else { rect.top() };
        let _line = ui
            .painter()
            .hline(rect.x_range(), y, egui::Stroke::new(1.0_f32, chrome::HOT));
    }
    if let Some(payload) = release_matching_payload::<T::Key>(&row.response)
        && *payload != *entry.key()
    {
        actions.push(CabinetAction::Moor {
            key: (*payload).clone(),
            berth: Berth::Beside {
                anchor: entry.key().clone(),
                after,
            },
        });
    }
}

fn cancel_entry_edit<K: Eq>(renaming: &mut EntryRenaming<'_, K>, key: &K) {
    if let EntryRenaming::Enabled(edit) = renaming
        && edit.as_ref().is_some_and(|edit| &edit.key == key)
    {
        **edit = None;
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "entry identity rendering receives the complete explicit cabinet interaction state"
)]
fn entry_identity<T: CabinetEntry>(
    cabinet: &Cabinet<T>,
    ui: &mut egui::Ui,
    water: &mut Surface,
    scope: &'static str,
    active: Option<&T::Key>,
    entry: &T,
    renaming: &mut EntryRenaming<'_, T::Key>,
    actions: &mut Vec<CabinetAction<T>>,
) {
    let key = entry.key();
    let EntryRenaming::Enabled(edit_slot) = renaming else {
        entry_label(ui, water, scope, active, entry, actions);
        return;
    };
    if !edit_slot.as_ref().is_some_and(|edit| edit.key == *key) {
        entry_label(ui, water, scope, active, entry, actions);
        return;
    }

    let (cancel, commit) = {
        let Some(edit) = edit_slot.as_mut() else {
            return;
        };
        let before = edit.name.clone();
        let mut field =
            egui::TextEdit::singleline(&mut edit.name).desired_width(ui.available_width());
        if edit.fault.is_some() {
            field = field.text_color(chrome::HOT);
        }
        let response = ui.add(field);
        if let Some(wake) = chrome::text_wake(ui, &response, &before, &edit.name) {
            water.text(wake);
        }
        if edit.focus {
            response.request_focus();
            edit.focus = false;
        }
        if let Some(fault) = edit.fault {
            let message = match fault {
                EntryNameFault::Invalid => "name cannot be empty",
                EntryNameFault::Occupied => "name already exists",
            };
            let _tooltip = response.clone().on_hover_text(message);
            let _fault = ui.painter().rect_stroke(
                response.rect,
                1.0,
                egui::Stroke::new(1.0_f32, chrome::HOT),
                egui::StrokeKind::Inside,
            );
        }
        record(
            ui,
            format!("cabinet.{scope}.entry-edit/{}", key.as_str()),
            response.interact_rect,
        );
        let (enter, escape) = ui.input(|input| {
            (
                input.key_pressed(egui::Key::Enter),
                input.key_pressed(egui::Key::Escape),
            )
        });
        (
            response.has_focus() && escape,
            (response.has_focus() && enter) || response.lost_focus(),
        )
    };
    if cancel {
        **edit_slot = None;
        return;
    }
    if !commit {
        return;
    }

    let Some(edit) = edit_slot.as_mut() else {
        return;
    };
    let Some(to) = T::Key::forge(&edit.name) else {
        edit.fault = Some(EntryNameFault::Invalid);
        edit.focus = true;
        return;
    };
    if to != edit.key && cabinet.taken(&to) {
        edit.fault = Some(EntryNameFault::Occupied);
        edit.focus = true;
        return;
    }
    let from = edit.key.clone();
    **edit_slot = None;
    if from != to {
        actions.push(CabinetAction::RenameEntry { from, to });
    }
}

fn entry_label<T: CabinetEntry>(
    ui: &mut egui::Ui,
    water: &mut Surface,
    scope: &'static str,
    active: Option<&T::Key>,
    entry: &T,
    actions: &mut Vec<CabinetAction<T>>,
) {
    let key = entry.key();
    let selected = active == Some(key);
    let sigil = entry.sigil().map(|sigil| format!("[{sigil}] "));
    let label = format!(
        "{}{}{}",
        if selected { "● " } else { "" },
        sigil.as_deref().unwrap_or_default(),
        key
    );
    let font = egui::TextStyle::Button.resolve(ui.style());
    let natural = ui
        .painter()
        .layout_no_wrap(label.clone(), font, egui::Color32::PLACEHOLDER)
        .size()
        .x;
    let truncated = natural > ui.available_width();
    let response = ui.selectable_label(selected, label);
    let response = if truncated {
        response.on_hover_text(key.as_str())
    } else {
        response
    };
    record(
        ui,
        format!("cabinet.{scope}.entry/{}", key.as_str()),
        response.interact_rect,
    );
    if chrome::hover_started(ui, &response) {
        water.bump(response.rect);
    }
    if response.clicked() {
        actions.push(CabinetAction::Load(entry.clone()));
    }
}

#[expect(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    reason = "one shelf is one cohesive physical assembly with complete explicit interaction state"
)]
fn shelf_rows<T: CabinetEntry>(
    cabinet: &Cabinet<T>,
    ui: &mut egui::Ui,
    water: &mut Surface,
    scope: &'static str,
    noun: &'static str,
    slot: usize,
    shelf: &Shelf<T>,
    active: Option<&T::Key>,
    shelf_edit: &mut Option<ShelfEdit>,
    entry_renaming: &mut EntryRenaming<'_, T::Key>,
    actions: &mut Vec<CabinetAction<T>>,
) {
    let id = ui.make_persistent_id((scope, "shelf", slot));
    let header = ui.horizontal(|ui| {
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
        let symbol = if shelf.open {
            Symbol::Collapse
        } else {
            Symbol::Expand
        };
        let assembly = Coupled::horizontal_with_gap(
            ui,
            CouplingGap::MINIMUM,
            |ui| {
                ui.push_id((scope, "shelf-drag", slot), |ui| {
                    DragHandle::friction_pad()
                        .size(MechanismSize::Large)
                        .show(ui)
                        .on_hover_text("drag to rearrange folders")
                })
                .inner
            },
            |ui| {
                Coupled::horizontal_with_gap(
                    ui,
                    CouplingGap::MINIMUM,
                    |ui| {
                        Monoglyph::symbol(symbol)
                            .show(ui)
                            .on_hover_text(if shelf.open {
                                "collapse folder"
                            } else {
                                "expand folder"
                            })
                    },
                    |ui| {
                        Coupled::horizontal_with_gap(
                            ui,
                            CouplingGap::MINIMUM,
                            |ui| {
                                Monoglyph::symbol(Symbol::Rename)
                                    .show(ui)
                                    .on_hover_text("rename folder")
                            },
                            |ui| {
                                Monoglyph::symbol(Symbol::Remove)
                                    .show(ui)
                                    .on_hover_text(format!("delete folder ({noun}s spill out)"))
                            },
                        )
                    },
                )
            },
        );
        water.drag_handle(&assembly.left);
        water.monoglyph(&assembly.right.left);
        water.monoglyph(&assembly.right.right.left);
        water.monoglyph(&assembly.right.right.right);
        assembly.left.dnd_set_drag_payload(ShelfDrag(slot));
        record(
            ui,
            format!("cabinet.{scope}.shelf-drag/{}", shelf.name),
            assembly.left.interact_rect,
        );
        record(
            ui,
            format!("cabinet.{scope}.shelf/{}", shelf.name),
            assembly.right.left.interact_rect,
        );
        if assembly.right.left.clicked() {
            actions.push(CabinetAction::ToggleShelf(slot));
        }
        if assembly.right.right.left.clicked() {
            actions.push(CabinetAction::BeginShelfRename(slot));
        }
        if assembly.right.right.right.clicked() {
            actions.push(CabinetAction::ScuttleShelf(slot));
        }
        match shelf_edit {
            Some(edit) if edit.shelf == slot => {
                let before = edit.name.clone();
                let entry = ui.text_edit_singleline(&mut edit.name);
                if let Some(wake) = chrome::text_wake(ui, &entry, &before, &edit.name) {
                    water.text(wake);
                }
                if edit.focus {
                    entry.request_focus();
                    edit.focus = false;
                }
                let enter = ui.input(|input| input.key_pressed(egui::Key::Enter));
                if entry.lost_focus() || (entry.has_focus() && enter) {
                    actions.push(CabinetAction::CommitShelfRename);
                }
            }
            _ => {
                let _name = ui.label(chrome::section_title(format!(
                    "{} ({})",
                    shelf.name,
                    shelf.entries.len()
                )));
            }
        }
    });
    let after = ui
        .ctx()
        .pointer_latest_pos()
        .is_some_and(|position| position.y > header.response.rect.center().y);
    if let Some(payload) = header.response.dnd_hover_payload::<ShelfDrag>()
        && payload.0 != slot
    {
        let y = if after {
            header.response.rect.bottom()
        } else {
            header.response.rect.top()
        };
        let _line = ui.painter().hline(
            header.response.rect.x_range(),
            y,
            egui::Stroke::new(1.0_f32, chrome::HOT),
        );
    }
    if let Some(payload) = release_matching_payload::<ShelfDrag>(&header.response)
        && payload.0 != slot
    {
        actions.push(CabinetAction::MoorShelf {
            shelf: payload.0,
            berth: ShelfBerth {
                anchor: slot,
                after,
            },
        });
    }
    if header.response.dnd_hover_payload::<T::Key>().is_some() {
        let _glow = ui.painter().rect_stroke(
            header.response.rect,
            2.0,
            egui::Stroke::new(1.0_f32, chrome::HOT),
            egui::StrokeKind::Inside,
        );
    }
    if let Some(payload) = release_matching_payload::<T::Key>(&header.response) {
        actions.push(CabinetAction::Moor {
            key: (*payload).clone(),
            berth: Berth::Shelf(slot),
        });
    }
    if shelf.open {
        let _body = ui.indent(id.with("body"), |ui| {
            if shelf.entries.is_empty() {
                let _empty = ui.label(chrome::muted("empty"));
            }
            for entry in &shelf.entries {
                entry_row(
                    cabinet,
                    ui,
                    water,
                    scope,
                    noun,
                    active,
                    entry,
                    entry_renaming,
                    actions,
                );
            }
        });
    }
}

fn root_berth<T: CabinetEntry>(
    cabinet: &Cabinet<T>,
    ui: &mut egui::Ui,
    scope: &'static str,
    actions: &mut Vec<CabinetAction<T>>,
) {
    let (rect, response) = ui
        .push_id((scope, "root"), |ui| {
            ui.allocate_exact_size(egui::vec2(ui.available_width(), 18.0), egui::Sense::hover())
        })
        .inner;
    record(ui, format!("cabinet.{scope}.root-berth"), rect);
    let armed =
        egui::DragAndDrop::payload::<T::Key>(ui.ctx()).is_some_and(|key| cabinet.is_shelved(&key));
    if armed {
        let hot = response.dnd_hover_payload::<T::Key>().is_some();
        let color = if hot {
            chrome::HOT
        } else {
            chrome::EDGE_STRONG
        };
        let _berth = ui.painter().rect_stroke(
            rect,
            2.0,
            egui::Stroke::new(1.0_f32, color),
            egui::StrokeKind::Inside,
        );
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "DROP OUTSIDE FOLDERS",
            egui::FontId::monospace(9.0),
            color,
        );
    } else {
        let _division = ui.painter().hline(
            rect.x_range(),
            rect.center().y,
            egui::Stroke::new(1.0_f32, chrome::EDGE),
        );
    }
    if armed && let Some(payload) = release_matching_payload::<T::Key>(&response) {
        actions.push(CabinetAction::Moor {
            key: (*payload).clone(),
            berth: Berth::Root,
        });
    }
}

/// Consume a drop only after refining egui's erased payload to the requested
/// type. `take_payload` removes before downcasting, so an unguarded probe for a
/// competing drag type would destroy the valid payload.
fn release_matching_payload<Payload>(response: &egui::Response) -> Option<Arc<Payload>>
where
    Payload: Any + Send + Sync,
{
    egui::DragAndDrop::has_payload_of_type::<Payload>(&response.ctx)
        .then(|| response.dnd_release_payload::<Payload>())
        .flatten()
}

fn rectify_entry<T: CabinetEntry>(entry: &mut T, seen: &mut HashSet<T::Key>) {
    let base = entry.key().clone();
    if seen.insert(base.clone()) {
        return;
    }
    let mut suffix = 2_u64;
    loop {
        let raw = format!("{} {suffix}", base.as_str());
        if let Some(candidate) = T::Key::forge(&raw)
            && seen.insert(candidate.clone())
        {
            entry.rename(candidate);
            return;
        }
        suffix = suffix.saturating_add(1);
    }
}

fn normalize_shelf_name(raw: &str) -> String {
    let name = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    if name.is_empty() {
        "folder".to_owned()
    } else {
        name
    }
}

fn spare_shelf_name(taken: &HashSet<String>, raw: &str) -> String {
    let base = normalize_shelf_name(raw);
    if !taken.contains(&base) {
        return base;
    }
    let mut suffix = 2_u64;
    loop {
        let candidate = format!("{base} {suffix}");
        if !taken.contains(&candidate) {
            return candidate;
        }
        suffix = suffix.saturating_add(1);
    }
}

#[inline]
fn record(ui: &egui::Ui, name: String, rect: egui::Rect) {
    #[cfg(all(
        feature = "egui-test",
        any(target_os = "linux", target_os = "macos", target_os = "windows")
    ))]
    {
        // Scroll areas lay out concealed children; their disjoint interaction
        // rectangles are inverted and cannot name a reachable test target.
        let visible = rect.intersect(ui.clip_rect());
        if visible.is_positive() {
            egui_tester_witness::egui::record(ui, name, visible);
        }
    }
    #[cfg(not(all(
        feature = "egui-test",
        any(target_os = "linux", target_os = "macos", target_os = "windows")
    )))]
    {
        let _ = (ui, name, rect);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[derive(Clone, Debug, Eq, Hash, PartialEq)]
    struct Name(String);

    impl Display for Name {
        fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str(&self.0)
        }
    }

    impl CabinetKey for Name {
        fn forge(raw: &str) -> Option<Self> {
            let name = raw.split_whitespace().collect::<Vec<_>>().join(" ");
            (!name.is_empty()).then_some(Self(name))
        }

        fn as_str(&self) -> &str {
            &self.0
        }
    }

    #[derive(Clone, Debug, PartialEq)]
    struct Mark(Name);

    impl CabinetEntry for Mark {
        type Key = Name;

        fn key(&self) -> &Self::Key {
            &self.0
        }

        fn rename(&mut self, key: Self::Key) {
            self.0 = key;
        }
    }

    #[derive(Clone, Debug)]
    enum Mutation {
        Beside { source: u8, anchor: u8, after: bool },
        Shelf { source: u8, shelf: u8 },
        Root { source: u8 },
        ReorderShelf { source: u8, anchor: u8, after: bool },
        ScuttleShelf { shelf: u8 },
        Rename { source: u8, name: u8 },
    }

    fn mutation() -> impl Strategy<Value = Mutation> {
        prop_oneof![
            (any::<u8>(), any::<u8>(), any::<bool>()).prop_map(|(source, anchor, after)| {
                Mutation::Beside {
                    source,
                    anchor,
                    after,
                }
            }),
            (any::<u8>(), any::<u8>())
                .prop_map(|(source, shelf)| Mutation::Shelf { source, shelf }),
            any::<u8>().prop_map(|source| Mutation::Root { source }),
            (any::<u8>(), any::<u8>(), any::<bool>()).prop_map(|(source, anchor, after)| {
                Mutation::ReorderShelf {
                    source,
                    anchor,
                    after,
                }
            }),
            any::<u8>().prop_map(|shelf| Mutation::ScuttleShelf { shelf }),
            (any::<u8>(), any::<u8>()).prop_map(|(source, name)| Mutation::Rename { source, name }),
        ]
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct Model {
        saved: Vec<Name>,
        shelves: Vec<(String, Vec<Name>)>,
    }

    impl Model {
        fn select(&self, selector: u8) -> Name {
            let entries = self
                .saved
                .iter()
                .chain(self.shelves.iter().flat_map(|(_, entries)| entries));
            entries
                .clone()
                .nth(usize::from(selector) % entries.count())
                .cloned()
                .expect("the model always owns entries")
        }

        fn remove(&mut self, key: &Name) -> Name {
            if let Some(index) = self.saved.iter().position(|candidate| candidate == key) {
                return self.saved.remove(index);
            }
            for (_, entries) in &mut self.shelves {
                if let Some(index) = entries.iter().position(|candidate| candidate == key) {
                    return entries.remove(index);
                }
            }
            panic!("selected model entry vanished")
        }

        fn berth(&self, key: &Name) -> (Option<usize>, usize) {
            if let Some(index) = self.saved.iter().position(|candidate| candidate == key) {
                return (None, index);
            }
            self.shelves
                .iter()
                .enumerate()
                .find_map(|(shelf, (_, entries))| {
                    entries
                        .iter()
                        .position(|candidate| candidate == key)
                        .map(|index| (Some(shelf), index))
                })
                .expect("selected model entry has a berth")
        }

        fn moor_beside(&mut self, moving: &Name, anchor: &Name, after: bool) {
            if moving == anchor {
                return;
            }
            let entry = self.remove(moving);
            let (rack, index) = self.berth(anchor);
            let index = index + usize::from(after);
            match rack {
                None => self.saved.insert(index, entry),
                Some(rack) => self.shelves[rack].1.insert(index, entry),
            }
        }

        fn moor_shelf(&mut self, moving: &Name, rack: usize) {
            let entry = self.remove(moving);
            match self.shelves.get_mut(rack) {
                Some((_, entries)) => entries.push(entry),
                None => self.saved.push(entry),
            }
        }

        fn moor_root(&mut self, source: &Name) {
            let entry = self.remove(source);
            self.saved.push(entry);
        }

        fn reorder_shelf(&mut self, moving: usize, anchor: usize, after: bool) {
            if moving == anchor || moving >= self.shelves.len() || anchor >= self.shelves.len() {
                return;
            }
            let rack = self.shelves.remove(moving);
            let anchor = anchor - usize::from(moving < anchor);
            self.shelves.insert(anchor + usize::from(after), rack);
        }

        fn scuttle(&mut self, rack: usize) {
            if rack < self.shelves.len() {
                let (_, entries) = self.shelves.remove(rack);
                self.saved.extend(entries);
            }
        }
    }

    fn cabinet() -> Cabinet<Mark> {
        Cabinet::forge(
            vec![mark("a"), mark("b")],
            vec![
                Shelf {
                    name: "one".to_owned(),
                    open: true,
                    entries: vec![mark("c"), mark("d")],
                },
                Shelf {
                    name: "two".to_owned(),
                    open: true,
                    entries: vec![mark("e"), mark("f")],
                },
            ],
        )
    }

    fn project(cabinet: &Cabinet<Mark>) -> Model {
        Model {
            saved: cabinet.saved.iter().map(|entry| entry.0.clone()).collect(),
            shelves: cabinet
                .shelves
                .iter()
                .map(|shelf| {
                    (
                        shelf.name.clone(),
                        shelf.entries.iter().map(|entry| entry.0.clone()).collect(),
                    )
                })
                .collect(),
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(128))]

        #[test]
        fn arbitrary_mutations_match_the_ordered_container_model(
            mutations in proptest::collection::vec(mutation(), 0..64)
        ) {
            let mut cabinet = cabinet();
            let mut model = project(&cabinet);

            for mutation in mutations {
                match mutation {
                    Mutation::Beside { source, anchor, after } => {
                        let source = model.select(source);
                        let anchor = model.select(anchor);
                        cabinet.moor(&source, &Berth::Beside { anchor: anchor.clone(), after });
                        model.moor_beside(&source, &anchor, after);
                    }
                    Mutation::Shelf { source, shelf } => {
                        let source = model.select(source);
                        let shelf = usize::from(shelf) % (model.shelves.len() + 1);
                        cabinet.moor(&source, &Berth::Shelf(shelf));
                        model.moor_shelf(&source, shelf);
                    }
                    Mutation::Root { source } => {
                        let source = model.select(source);
                        cabinet.moor(&source, &Berth::Root);
                        model.moor_root(&source);
                    }
                    Mutation::ReorderShelf { source, anchor, after } => {
                        let modulus = model.shelves.len() + 1;
                        let source = usize::from(source) % modulus;
                        let anchor = usize::from(anchor) % modulus;
                        cabinet.moor_shelf(source, ShelfBerth { anchor, after });
                        model.reorder_shelf(source, anchor, after);
                    }
                    Mutation::ScuttleShelf { shelf } => {
                        let shelf = usize::from(shelf) % (model.shelves.len() + 1);
                        cabinet.scuttle_shelf(shelf);
                        model.scuttle(shelf);
                    }
                    Mutation::Rename { source, name } => {
                        let source = model.select(source);
                        let name = Name(format!("n{}", name % 8));
                        let berth = model.berth(&source);
                        let occupied = model
                            .saved
                            .iter()
                            .chain(model.shelves.iter().flat_map(|(_, entries)| entries))
                            .any(|candidate| candidate == &name);
                        let admitted = source == name || !occupied;
                        prop_assert_eq!(cabinet.rename(&source, name.clone()), admitted);
                        if admitted {
                            let _prior = model.remove(&source);
                            let entry = name;
                            match berth.0 {
                                None => model.saved.insert(berth.1, entry),
                                Some(shelf) => model.shelves[shelf].1.insert(berth.1, entry),
                            }
                        }
                    }
                }
                prop_assert_eq!(project(&cabinet), model.clone());
            }
        }
    }

    #[test]
    fn rectification_preserves_every_entry_under_free_normalized_names() {
        let duplicate = mark("alpha");
        let cabinet = Cabinet::forge(
            vec![duplicate.clone(), mark("beta")],
            vec![
                Shelf {
                    name: " storms ".to_owned(),
                    open: true,
                    entries: vec![duplicate],
                },
                Shelf {
                    name: "storms".to_owned(),
                    ..Shelf::default()
                },
            ],
        );
        assert_eq!(names(cabinet.all()), ["alpha", "beta", "alpha 2"]);
        assert_eq!(shelf_names(&cabinet), ["storms", "storms 2"]);
    }

    #[test]
    fn competing_drop_types_cannot_devour_an_item_release() {
        let context = egui::Context::default();
        let position = egui::pos2(40.0, 40.0);
        let input = |pressed| egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(100.0, 100.0),
            )),
            events: vec![
                egui::Event::PointerMoved(position),
                egui::Event::PointerButton {
                    pos: position,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::default(),
                },
            ],
            ..egui::RawInput::default()
        };
        context
            .run_ui(input(true), |ui| {
                let _drop_zone = ui.allocate_response(ui.available_size(), egui::Sense::hover());
            })
            .drop_without_applying_deltas();

        let expected = Name("dragged item".to_owned());
        egui::DragAndDrop::set_payload(&context, expected.clone());
        let mut recovered = None;
        context
            .run_ui(input(false), |ui| {
                let response = ui.allocate_response(ui.available_size(), egui::Sense::hover());
                assert!(release_matching_payload::<ShelfDrag>(&response).is_none());
                if let Some(payload) = release_matching_payload::<Name>(&response) {
                    recovered = Some(payload);
                }
            })
            .drop_without_applying_deltas();

        assert_eq!(recovered.as_deref(), Some(&expected));
    }

    fn mark(name: &str) -> Mark {
        Mark(Name::forge(name).expect("static test key"))
    }

    fn names<'a>(entries: impl Iterator<Item = &'a Mark>) -> Vec<&'a str> {
        entries.map(|entry| entry.key().as_str()).collect()
    }

    fn shelf_names(cabinet: &Cabinet<Mark>) -> Vec<&str> {
        cabinet
            .shelves
            .iter()
            .map(|shelf| shelf.name.as_str())
            .collect()
    }
}
