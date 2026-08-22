#![expect(
    unused_crate_dependencies,
    reason = "the atelier consumes the native and WebGPU host dependencies through its support module"
)]

mod support;

#[cfg(any(
    target_arch = "wasm32",
    target_os = "linux",
    target_os = "macos",
    target_os = "windows"
))]
use anyhow::Result;
use brass_poolrooms::{
    chrome::{self, Checkbox, NumberInput, Rail, WheelPlane},
    egui,
    water::Surface,
};
use eternalist_apps::cabinet::{
    Cabinet, CabinetAction, CabinetEntry, CabinetKey, EntryEdit, Shelf, ShelfEdit,
};
use eternalist_apps::command_guide::{CommandGuide, GuideGesture, GuideSection};
use eternalist_apps::commands::{
    CommandCanon, CommandDispatch, CommandScope, CommandSpec, CommandStatus, Shortcut, ShortcutKey,
    ShortcutModifiers, TextFocusPolicy,
};
use eternalist_apps::panel_navigation::PanelNavigator;
use eternalist_apps::settings::{SettingSpec, SettingsFile, SettingsSheet};
use eternalist_apps::{Inspector, LivingWait};
use std::{
    fmt::{Display, Formatter},
    path::Path,
    sync::OnceLock,
};

#[cfg(all(target_os = "linux", feature = "egui-test"))]
const FOCUS_SENTINEL: &str = "--focus-sentinel";
#[cfg(all(target_os = "linux", feature = "egui-test"))]
const SHORT_WINDOW: &str = "--short-window";
use support::Exhibit;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Page {
    Inspector,
    LivingWait,
    Cabinet,
    Commands,
    Settings,
}

impl Page {
    const ALL: [Self; 5] = [
        Self::Inspector,
        Self::LivingWait,
        Self::Cabinet,
        Self::Commands,
        Self::Settings,
    ];

    const fn label(self) -> &'static str {
        match self {
            Self::Inspector => "INSPECTOR",
            Self::LivingWait => "LIVING WAIT",
            Self::Cabinet => "CABINET",
            Self::Commands => "COMMANDS",
            Self::Settings => "SETTINGS",
        }
    }

    const fn number(self) -> &'static str {
        match self {
            Self::Inspector => "01",
            Self::LivingWait => "02",
            Self::Cabinet => "03",
            Self::Commands => "04",
            Self::Settings => "05",
        }
    }

    #[cfg(all(
        feature = "egui-test",
        any(target_os = "linux", target_os = "macos", target_os = "windows")
    ))]
    const fn wire(self) -> &'static str {
        match self {
            Self::Inspector => "inspector",
            Self::LivingWait => "living_wait",
            Self::Cabinet => "cabinet",
            Self::Commands => "commands",
            Self::Settings => "settings",
        }
    }

    const fn target(self) -> &'static str {
        match self {
            Self::Inspector => "atelier.tab.inspector",
            Self::LivingWait => "atelier.tab.living_wait",
            Self::Cabinet => "atelier.tab.cabinet",
            Self::Commands => "atelier.tab.commands",
            Self::Settings => "atelier.tab.settings",
        }
    }
}

struct Atelier {
    page: Page,
    inspector: InspectorExhibit,
    waiting: WaitingExhibit,
    cabinet: CabinetExhibit,
    commands: CommandsExhibit,
    settings: SettingsExhibit,
}

impl Default for Atelier {
    fn default() -> Self {
        Self {
            page: Page::Inspector,
            inspector: InspectorExhibit::default(),
            waiting: WaitingExhibit::default(),
            cabinet: CabinetExhibit::default(),
            commands: CommandsExhibit::default(),
            settings: SettingsExhibit::default(),
        }
    }
}

impl Exhibit for Atelier {
    const TITLE: &'static str = "Eternalist · application primitive atelier";
    #[cfg(not(target_arch = "wasm32"))]
    const SIZE: [f64; 2] = [1180.0, 780.0];
    #[cfg(target_arch = "wasm32")]
    const CANVAS_ID: &'static str = "eternalist";
    #[cfg(target_arch = "wasm32")]
    const READY_MESSAGE: &'static str = "Eternalist WebGPU atelier is live";

    #[cfg(all(
        feature = "egui-test",
        any(target_os = "linux", target_os = "macos", target_os = "windows")
    ))]
    type Observation = AtelierObservation;

    fn ui(&mut self, ui: &mut egui::Ui, water: &mut Surface) {
        self.tabs(ui);
        match self.page {
            Page::Inspector => self.inspector.show(ui, water),
            Page::LivingWait => self.waiting.show(ui, water),
            Page::Cabinet => self.cabinet.show(ui, water),
            Page::Commands => self.commands.show(ui, water),
            Page::Settings => self.settings.show(ui, water),
        }
    }

    #[cfg(all(
        feature = "egui-test",
        any(target_os = "linux", target_os = "macos", target_os = "windows")
    ))]
    fn observe(&self, _text_edit_focused: bool) -> Self::Observation {
        AtelierObservation {
            page: self.page.wire(),
            guide_open: self.commands.guide.is_open(),
            status: self.commands.status.clone(),
            selected: self.commands.selected,
            filter: self.commands.filter.clone(),
            density: self.commands.density,
            inspector_expanded: self.commands.inspector_expanded,
            inspector_extent: self.commands.inspector_extent,
            settings: AtelierSettingsObservation {
                open: self.settings.sheet.is_open(),
                fault: self.settings.faulted(),
            },
        }
    }
}

#[cfg(all(target_os = "linux", feature = "egui-test"))]
struct ShortAtelier(Atelier);

#[cfg(all(target_os = "linux", feature = "egui-test"))]
impl Exhibit for ShortAtelier {
    const TITLE: &'static str = Atelier::TITLE;
    const SIZE: [f64; 2] = [763.0, 311.0];
    type Observation = AtelierObservation;

    fn ui(&mut self, ui: &mut egui::Ui, water: &mut Surface) {
        self.0.ui(ui, water);
    }

    fn observe(&self, text_edit_focused: bool) -> Self::Observation {
        self.0.observe(text_edit_focused)
    }
}

#[cfg(all(target_os = "linux", feature = "egui-test"))]
struct FocusSentinel;

#[cfg(all(target_os = "linux", feature = "egui-test"))]
impl Exhibit for FocusSentinel {
    const TITLE: &'static str = "Eternalist · focus sentinel";
    const SIZE: [f64; 2] = [220.0, 140.0];
    type Observation = ();

    fn ui(&mut self, ui: &mut egui::Ui, _water: &mut Surface) {
        let _panel = egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(chrome::PAGE))
            .show(ui, |ui| {
                let _label = ui.centered_and_justified(|ui| {
                    ui.label(chrome::muted("FOCUS SENTINEL"));
                });
            });
    }

    fn observe(&self, _text_edit_focused: bool) -> Self::Observation {}
}

#[cfg(all(
    feature = "egui-test",
    any(target_os = "linux", target_os = "macos", target_os = "windows")
))]
#[derive(serde::Serialize)]
struct AtelierObservation {
    page: &'static str,
    guide_open: bool,
    status: String,
    selected: bool,
    filter: String,
    density: u16,
    inspector_expanded: bool,
    inspector_extent: f32,
    settings: AtelierSettingsObservation,
}

#[cfg(all(
    feature = "egui-test",
    any(target_os = "linux", target_os = "macos", target_os = "windows")
))]
#[derive(serde::Serialize)]
struct AtelierSettingsObservation {
    open: bool,
    fault: bool,
}

impl Atelier {
    fn tabs(&mut self, ui: &mut egui::Ui) {
        let panel = egui::Panel::top("atelier-tabs")
            .exact_size(82.0)
            .frame(
                egui::Frame::new()
                    .fill(chrome::SURFACE)
                    .stroke(egui::Stroke::new(1.0_f32, chrome::EDGE_STRONG))
                    .inner_margin(egui::Margin::symmetric(20, 12)),
            )
            .show(ui, |ui| {
                let _heading = ui.horizontal(|ui| {
                    let _title = ui.label(chrome::title("ETERNALIST ATELIER"));
                    ui.add_space(10.0);
                    let _kind = ui.label(chrome::muted("APPLICATION PRIMITIVES"));
                });
                ui.add_space(8.0);
                let _tabs = ui.horizontal(|ui| {
                    for page in Page::ALL {
                        let response = tab(ui, page, self.page == page);
                        if response.clicked() && self.page != page {
                            self.page = page;
                            ui.ctx().request_discard("atelier page changed");
                        }
                    }
                });
            });
        let _response = panel.response;
    }
}

fn tab(ui: &mut egui::Ui, page: Page, selected: bool) -> egui::Response {
    let label = format!("{}  {}", page.number(), page.label());
    let button = egui::Button::new(chrome::section_title(label))
        .min_size(egui::vec2(166.0, 30.0))
        .fill(if selected {
            chrome::RAISED
        } else {
            chrome::CONTROL
        })
        .stroke(egui::Stroke::new(
            if selected { 1.5_f32 } else { 1.0_f32 },
            if selected { chrome::HOT } else { chrome::EDGE },
        ));
    let response = ui.add(button);
    chrome::shallow_tension(ui, &response);
    record_response(ui, page.target(), &response);
    response
}

struct InspectorExhibit {
    lamps: bool,
    labels: bool,
    density: u16,
    opacity: f32,
    title: String,
    scroll_offset: f32,
}

impl Default for InspectorExhibit {
    fn default() -> Self {
        Self {
            lamps: true,
            labels: true,
            density: 4,
            opacity: 0.75,
            title: "CHRONICLE 07".to_owned(),
            scroll_offset: 0.0,
        }
    }
}

impl InspectorExhibit {
    fn show(&mut self, ui: &mut egui::Ui, water: &mut Surface) {
        let inspector = Inspector::new("atelier-inspector")
            .scroll_offset(self.scroll_offset)
            .show(ui, |ui| self.controls(ui, water));
        self.scroll_offset = inspector.scroll_offset;
        inspector.agitate(water);

        let _stage = egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(chrome::PAGE)
                    .inner_margin(egui::Margin::same(28)),
            )
            .show(ui, |ui| self.stage(ui));
    }

    fn controls(&mut self, ui: &mut egui::Ui, water: &mut Surface) {
        let _eyebrow = ui.label(chrome::eyebrow("PERSISTENT APPLICATION RAIL"));
        let _title = ui.label(chrome::title("INSPECTOR"));
        let _law = ui.label(chrome::muted(
            "geometry and scrolling are shared; meaning and state remain here",
        ));
        ui.add_space(16.0);

        let wake = chrome::section(ui, "atelier-view", "VIEW", true, |ui| {
            let lamps = Checkbox::new(&mut self.lamps, "PLATE LAMPS").show(ui);
            water.checkbox(&lamps);
            ui.add_space(9.0);
            let labels = Checkbox::new(&mut self.labels, "PLATE LABELS").show(ui);
            water.checkbox(&labels);
        });
        water.fold(wake);
        ui.add_space(10.0);

        let wake = chrome::section(ui, "atelier-density", "DENSITY", true, |ui| {
            let _legend = ui.label(chrome::muted(format!("{} active plates", self.density)));
            let rail = Rail::new(&mut self.density, 1..=8)
                .detents(8)
                .wheel()
                .width(ui.available_width())
                .show(ui);
            water.rail(&rail);
        });
        water.fold(wake);
        ui.add_space(10.0);

        let wake = chrome::section(ui, "atelier-material", "MATERIAL", true, |ui| {
            let _caption = ui.label(chrome::muted("plate opacity"));
            let opacity = NumberInput::new(&mut self.opacity, 0.15..=1.0, 0.05, 2)
                .wheel_plane(WheelPlane::YZ)
                .register_width(74.0)
                .show(ui);
            water.number_input(&opacity);
        });
        water.fold(wake);
        ui.add_space(10.0);

        let wake = chrome::section(ui, "atelier-identity", "IDENTITY", false, |ui| {
            let before = self.title.clone();
            let response = ui.add(
                egui::TextEdit::singleline(&mut self.title)
                    .desired_width(ui.available_width())
                    .char_limit(24),
            );
            if let Some(wake) = chrome::text_wake(ui, &response, &before, &self.title) {
                water.text(wake);
            }
            let _hint = ui.label(chrome::muted("application-owned state and persistence"));
            ui.add_space(7.0);
            let _retention = ui.label(chrome::muted("retention · permanent"));
            let _ordering = ui.label(chrome::muted("ordering · newest first"));
            let _scope = ui.label(chrome::muted("scope · local chronicle"));
            let _authority = ui.label(chrome::muted("authority · product model"));
        });
        water.fold(wake);
        ui.add_space(18.0);

        let _note = chrome::note(
            ui,
            "The primitive never invents these sections, controls, or values.",
        );
        ui.add_space(12.0);
    }

    fn stage(&mut self, ui: &mut egui::Ui) {
        let _eyebrow = ui.label(chrome::eyebrow("LIVE COMPOSITION"));
        let _title = ui.label(chrome::title(self.title.to_uppercase()));
        let _law = ui.label(chrome::muted(
            "the application canvas remains unconstrained beside the shared rail",
        ));
        ui.add_space(18.0);

        let (rect, response) = ui.allocate_exact_size(ui.available_size(), egui::Sense::click());
        chrome::shallow_tension(ui, &response);
        let rect = rect.shrink2(egui::vec2(4.0, 4.0));
        let painter = ui.painter_at(rect);
        let _body = painter.rect_filled(rect, 2.0, chrome::SURFACE);
        let _edge = painter.rect_stroke(
            rect,
            2.0,
            egui::Stroke::new(1.0_f32, chrome::EDGE_STRONG),
            egui::StrokeKind::Inside,
        );

        paint_plates(
            &painter,
            rect.shrink(34.0),
            self.density,
            self.opacity,
            self.lamps,
            self.labels,
        );
    }
}

fn paint_plates(
    painter: &egui::Painter,
    rect: egui::Rect,
    count: u16,
    opacity: f32,
    lamps: bool,
    labels: bool,
) {
    const COLUMNS: u16 = 4;
    let gap = 14.0;
    let width = (rect.width() - gap * f32::from(COLUMNS - 1)) / f32::from(COLUMNS);
    let height = ((rect.height() - gap) * 0.5).min(170.0);
    for index in 0..count {
        let column = index % COLUMNS;
        let row = index / COLUMNS;
        let min = egui::pos2(
            rect.left() + f32::from(column) * (width + gap),
            rect.top() + f32::from(row) * (height + gap),
        );
        let plate = egui::Rect::from_min_size(min, egui::vec2(width, height));
        let fill = chrome::RAISED.gamma_multiply(opacity);
        let _fill = painter.rect_filled(plate, 2.0, fill);
        let _edge = painter.rect_stroke(
            plate,
            2.0,
            egui::Stroke::new(1.0_f32, chrome::EDGE_STRONG),
            egui::StrokeKind::Inside,
        );
        if lamps {
            let lamp = egui::Rect::from_min_size(
                plate.left_top() + egui::vec2(12.0, 12.0),
                egui::vec2((width - 24.0).max(8.0), 5.0),
            );
            let _lamp = painter.rect_filled(lamp, 1.0, chrome::HOT.gamma_multiply(opacity));
        }
        if labels {
            painter.text(
                plate.left_bottom() + egui::vec2(12.0, -12.0),
                egui::Align2::LEFT_BOTTOM,
                format!("PLATE {:02}", index + 1),
                egui::FontId::monospace(12.0),
                chrome::TEXT,
            );
        }
    }
}

struct WaitingExhibit {
    primary: bool,
    index: bool,
    export: bool,
    wait: LivingWait,
}

impl Default for WaitingExhibit {
    fn default() -> Self {
        Self {
            primary: true,
            index: true,
            export: false,
            wait: LivingWait::default(),
        }
    }
}

impl WaitingExhibit {
    fn show(&mut self, ui: &mut egui::Ui, water: &mut Surface) {
        let _controls = egui::Panel::top("waiting-controls")
            .exact_size(122.0)
            .frame(
                egui::Frame::new()
                    .fill(chrome::PAGE)
                    .inner_margin(egui::Margin::symmetric(28, 16)),
            )
            .show(ui, |ui| {
                let _eyebrow = ui.label(chrome::eyebrow("ONE FRAME · ONE PHYSICAL RAFT"));
                let _title = ui.label(chrome::title("VISIBLE WAIT ARBITRATION"));
                ui.add_space(10.0);
                let _row = ui.horizontal(|ui| {
                    waiting_switch(ui, water, &mut self.primary, "PRIMARY LOAD");
                    ui.add_space(24.0);
                    waiting_switch(ui, water, &mut self.index, "INDEX STATUS");
                    ui.add_space(24.0);
                    waiting_switch(ui, water, &mut self.export, "EXPORT STATUS");
                });
            });

        let _stage = egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(chrome::PAGE)
                    .inner_margin(egui::Margin::same(28)),
            )
            .show(ui, |ui| self.stage(ui));
        self.wait.compose(ui.ctx(), water);
    }

    fn stage(&mut self, ui: &mut egui::Ui) {
        let arena = ui.max_rect();
        let painter = ui.painter().clone();
        let _fill = painter.rect_filled(arena, 2.0, chrome::SURFACE);
        let _edge = painter.rect_stroke(
            arena,
            2.0,
            egui::Stroke::new(1.0_f32, chrome::EDGE_STRONG),
            egui::StrokeKind::Inside,
        );

        if self.index {
            let rect = egui::Rect::from_min_size(
                arena.left_bottom() + egui::vec2(20.0, -92.0),
                egui::vec2(300.0, 68.0),
            );
            waiting_card(ui, &mut self.wait, rect, "INDEXING 4,192 RECORDS");
        }
        if self.export {
            let rect = egui::Rect::from_min_size(
                arena.right_top() + egui::vec2(-250.0, 24.0),
                egui::vec2(220.0, 82.0),
            );
            waiting_card(ui, &mut self.wait, rect, "FORGING ARCHIVE");
        }
        if self.primary {
            let _rect = self.wait.bouncer(ui, arena);
        }
        if !self.primary && !self.index && !self.export {
            painter.text(
                arena.center(),
                egui::Align2::CENTER_CENTER,
                "QUIET",
                egui::FontId::proportional(40.0),
                chrome::MUTED,
            );
        }
    }
}

fn waiting_switch(ui: &mut egui::Ui, water: &mut Surface, state: &mut bool, label: &'static str) {
    let response = Checkbox::new(state, label).show(ui);
    water.checkbox(&response);
}

fn waiting_card(ui: &egui::Ui, wait: &mut LivingWait, rect: egui::Rect, label: &'static str) {
    wait.claim(rect);
    let painter = ui.painter();
    let _fill = painter.rect_filled(rect, 2.0, chrome::RAISED);
    let _edge = painter.rect_stroke(
        rect,
        2.0,
        egui::Stroke::new(1.0_f32, chrome::EDGE_STRONG),
        egui::StrokeKind::Inside,
    );
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::monospace(15.0),
        chrome::HOT,
    );
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct ItemName(String);

impl ItemName {
    fn new(name: &str) -> Self {
        Self(name.to_owned())
    }
}

impl Display for ItemName {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl CabinetKey for ItemName {
    fn forge(raw: &str) -> Option<Self> {
        let name = raw.split_whitespace().collect::<Vec<_>>().join(" ");
        (!name.is_empty()).then_some(Self(name))
    }

    fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug)]
struct Item {
    name: ItemName,
    sigil: char,
    law: &'static str,
}

impl Item {
    fn new(name: &str, sigil: char, law: &'static str) -> Self {
        Self {
            name: ItemName::new(name),
            sigil,
            law,
        }
    }
}

impl CabinetEntry for Item {
    type Key = ItemName;

    fn key(&self) -> &ItemName {
        &self.name
    }

    fn rename(&mut self, name: ItemName) {
        self.name = name;
    }

    fn sigil(&self) -> Option<char> {
        Some(self.sigil)
    }
}

struct CabinetExhibit {
    cabinet: Cabinet<Item>,
    active: ItemName,
    shelf_edit: Option<ShelfEdit>,
    entry_edit: Option<EntryEdit<ItemName>>,
    status: String,
}

impl Default for CabinetExhibit {
    fn default() -> Self {
        let active = ItemName::new("field notes");
        Self {
            cabinet: Cabinet::forge(
                vec![
                    Item::new("field notes", 'N', "application-owned working record"),
                    Item::new("north transect", 'T', "application-owned spatial intent"),
                ],
                vec![
                    Shelf {
                        name: "archive".to_owned(),
                        open: true,
                        entries: vec![Item::new(
                            "winter plate",
                            'W',
                            "application-owned immutable snapshot",
                        )],
                    },
                    Shelf {
                        name: "reference".to_owned(),
                        open: false,
                        entries: vec![Item::new(
                            "material index",
                            'M',
                            "application-owned reference record",
                        )],
                    },
                ],
            ),
            active,
            shelf_edit: None,
            entry_edit: None,
            status: "cabinet identity and placement are shared".to_owned(),
        }
    }
}

impl CabinetExhibit {
    fn show(&mut self, ui: &mut egui::Ui, water: &mut Surface) {
        let inspector = Inspector::new("cabinet-atelier").show(ui, |ui| {
            let _eyebrow = ui.label(chrome::eyebrow("PERSISTENT LOGICAL COLLECTION"));
            let _title = ui.label(chrome::title("CABINET"));
            let _law = ui.label(chrome::muted(
                "identity, order, shelves, and drag grammar are shared",
            ));
            ui.add_space(16.0);
            self.cabinet.show_renamable(
                ui,
                water,
                "atelier",
                "item",
                Some(&self.active),
                &mut self.shelf_edit,
                &mut self.entry_edit,
            )
        });
        inspector.agitate(water);
        self.apply(inspector.inner);

        let _stage = egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(chrome::PAGE)
                    .inner_margin(egui::Margin::same(28)),
            )
            .show(ui, |ui| {
                let _eyebrow = ui.label(chrome::eyebrow("APPLICATION PROJECTION"));
                let _title = ui.label(chrome::title(self.active.to_string().to_uppercase()));
                let law = self
                    .cabinet
                    .get(&self.active)
                    .map_or("entry removed", |item| item.law);
                let _law = ui.label(chrome::muted(law));
                ui.add_space(18.0);
                let _note = chrome::note(ui, &self.status);
                ui.add_space(18.0);
                let _division = chrome::note(
                    ui,
                    "The cabinet does not own document meaning, active-state policy, or persistence.",
                );
            });
    }

    fn apply(&mut self, actions: Vec<CabinetAction<Item>>) {
        for action in actions {
            match action {
                CabinetAction::Load(item) => {
                    item.name.clone_into(&mut self.active);
                    self.status = format!("loaded `{}`", item.name);
                }
                CabinetAction::Clone(name) => {
                    let Some(mut item) = self.cabinet.get(&name).cloned() else {
                        continue;
                    };
                    let clone = self.cabinet.spare_named(&name);
                    item.rename(clone.clone());
                    self.cabinet.adopt_beside(&name, item);
                    clone.clone_into(&mut self.active);
                    self.status = format!("cloned `{clone}` beside its source");
                }
                CabinetAction::Delete(name) => {
                    let _removed = self.cabinet.remove(&name);
                    if self.active == name
                        && let Some(first) = self.cabinet.all().next()
                    {
                        first.name.clone_into(&mut self.active);
                    }
                    self.status = format!("deleted `{name}`");
                }
                CabinetAction::RenameEntry { from, to } => {
                    if self.cabinet.rename(&from, to.clone()) {
                        if self.active == from {
                            to.clone_into(&mut self.active);
                        }
                        self.status = format!("renamed `{from}` → `{to}`");
                    }
                }
                CabinetAction::Moor { key, berth } => {
                    self.cabinet.moor(&key, &berth);
                    self.status = format!("re-homed `{key}`");
                }
                CabinetAction::MoorShelf { shelf, berth } => {
                    self.cabinet.moor_shelf(shelf, berth);
                    self.shelf_edit = None;
                    "reordered folders".clone_into(&mut self.status);
                }
                CabinetAction::NewShelf => {
                    self.cabinet.add_shelf();
                    "forged an empty shelf".clone_into(&mut self.status);
                }
                CabinetAction::ToggleShelf(index) => self.cabinet.toggle_shelf(index),
                CabinetAction::ScuttleShelf(index) => {
                    self.cabinet.scuttle_shelf(index);
                    self.shelf_edit = None;
                    "scuttled a shelf; its entries spilled to the root"
                        .clone_into(&mut self.status);
                }
                CabinetAction::BeginShelfRename(index) => {
                    let name = self
                        .cabinet
                        .shelves
                        .get(index)
                        .map(|shelf| shelf.name.clone())
                        .unwrap_or_default();
                    self.shelf_edit = Some(ShelfEdit {
                        shelf: index,
                        name,
                        focus: true,
                    });
                }
                CabinetAction::CommitShelfRename => {
                    if let Some(edit) = self.shelf_edit.take() {
                        self.status = if self.cabinet.rename_shelf(edit.shelf, &edit.name) {
                            format!("renamed shelf to `{}`", edit.name.trim())
                        } else {
                            format!("shelf `{}` already exists", edit.name.trim())
                        };
                    }
                }
            }
        }
    }
}

const RESTORE_WORKSPACE: SettingSpec = SettingSpec::new(
    "restore_workspace",
    "RESTORE WORKSPACE",
    "Reopen the last active document and working context at launch.",
);
const CONFIRM_DISCARD: SettingSpec = SettingSpec::new(
    "confirm_discard",
    "CONFIRM DISCARD",
    "Ask before abandoning edits that have not reached durable storage.",
);
const SETTLEMENT_DELAY: SettingSpec = SettingSpec::new(
    "settlement_delay",
    "SETTLEMENT DELAY",
    "Wait this many seconds after the last change before committing it.",
);

struct SettingsExhibit {
    sheet: SettingsSheet,
    restore_workspace: bool,
    confirm_discard: bool,
    settlement_delay: f64,
    condition: SettingsCondition,
    visit: SettingsVisit,
}

#[derive(Clone, Copy, Default, Eq, PartialEq)]
enum SettingsCondition {
    #[default]
    Ready,
    Fault,
}

#[derive(Clone, Copy, Default, Eq, PartialEq)]
enum SettingsVisit {
    #[default]
    Unseen,
    Seen,
}

impl Default for SettingsExhibit {
    fn default() -> Self {
        Self {
            sheet: SettingsSheet::default(),
            restore_workspace: true,
            confirm_discard: true,
            settlement_delay: 0.4,
            condition: SettingsCondition::Ready,
            visit: SettingsVisit::Unseen,
        }
    }
}

impl SettingsExhibit {
    fn show(&mut self, ui: &mut egui::Ui, water: &mut Surface) {
        let _invoked = self.sheet.take_shortcut(ui.ctx());
        if self.visit == SettingsVisit::Unseen {
            self.visit = SettingsVisit::Seen;
            self.sheet.open(ui.ctx());
        }
        let _stage = egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(chrome::PAGE)
                    .inner_margin(egui::Margin::same(28)),
            )
            .show(ui, |ui| {
                let _eyebrow = ui.label(chrome::eyebrow("CENTRAL APPLICATION CONFIGURATION"));
                let _heading = ui.horizontal(|ui| {
                    let _title = ui.label(chrome::title("SETTINGS SHEET"));
                    let activator = self.sheet.activator(ui, self.faulted());
                    water.monoglyph(&activator);
                });
                let _law = ui.label(chrome::muted(
                    "contextual controls and one complete surface share the same setting declarations",
                ));
                ui.add_space(22.0);
                let _specimen = egui::Frame::new()
                    .fill(chrome::SURFACE)
                    .stroke(egui::Stroke::new(1.0_f32, chrome::EDGE_STRONG))
                    .inner_margin(egui::Margin::same(18))
                    .show(ui, |ui| {
                        let _title = ui.label(chrome::section_title("PREFLIGHT SPECIMEN"));
                        ui.add_space(8.0);
                        let mut faulted = self.faulted();
                        let fault = Checkbox::new(&mut faulted, "SIMULATE INVALID FILE")
                            .size(chrome::MechanismSize::Small)
                            .show(ui);
                        record_response(ui, "atelier.settings.fault", &fault);
                        water.checkbox(&fault);
                        if fault.changed() {
                            self.condition = if faulted {
                                self.sheet.require_attention(ui.ctx());
                                SettingsCondition::Fault
                            } else {
                                SettingsCondition::Ready
                            };
                        }
                        ui.add_space(8.0);
                        let state = if self.faulted() {
                            "unknown keys block mutation and summon this sheet without rewriting the file"
                        } else {
                            "configuration admitted; application controls remain writable"
                        };
                        let _state = ui.label(chrome::muted(state));
                    });
            });

        let path = Path::new("config/atelier.toml");
        let file = if self.faulted() {
            SettingsFile::fault(path, "Unknown configuration key: restore_workpace")
        } else {
            SettingsFile::ready(path)
        };
        let _response = self.sheet.show(ui.ctx(), water, file, |ui| {
            ui.section("WORKSPACE");
            let _restored = ui.boolean(RESTORE_WORKSPACE, &mut self.restore_workspace);
            let _confirmed = ui.boolean(CONFIRM_DISCARD, &mut self.confirm_discard);
            let _delay = ui.number(
                SETTLEMENT_DELAY,
                &mut self.settlement_delay,
                0.1..=2.0,
                0.1,
                1,
            );
        });
        if let Some(rect) = self.sheet.rect() {
            record_rect(ui.ctx(), "atelier.settings.sheet", rect);
        }
    }

    const fn faulted(&self) -> bool {
        matches!(self.condition, SettingsCondition::Fault)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DemoCommand {
    Open,
    Save,
    Rename,
    FocusSearch,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DemoScope {
    Workspace,
}

const OPEN_KEYS: [Shortcut; 1] = [Shortcut::primary('O')];
const SAVE_KEYS: [Shortcut; 1] = [Shortcut::primary('S')];
const RENAME_KEYS: [Shortcut; 1] = [Shortcut::new(
    ShortcutModifiers::NONE,
    ShortcutKey::Function(3),
)];
const SEARCH_KEYS: [Shortcut; 1] = [Shortcut::new(ShortcutModifiers::NONE, ShortcutKey::Slash)];
const TOGGLE_CONTROLS: [Shortcut; 1] = [Shortcut::new(
    ShortcutModifiers::NONE,
    ShortcutKey::Function(9),
)];
const NEXT_CONTROL_GROUP: [Shortcut; 1] =
    [Shortcut::new(ShortcutModifiers::CONTROL, ShortcutKey::Tab)];
const PREVIOUS_CONTROL_GROUP: [Shortcut; 1] = [Shortcut::new(
    ShortcutModifiers::CONTROL.plus(ShortcutModifiers::SHIFT),
    ShortcutKey::Tab,
)];
const ADJUST_DENSITY: [Shortcut; 2] = [
    Shortcut::new(ShortcutModifiers::NONE, ShortcutKey::ArrowLeft),
    Shortcut::new(ShortcutModifiers::NONE, ShortcutKey::ArrowRight),
];
const DENSITY_BOUNDS: [Shortcut; 2] = [
    Shortcut::new(ShortcutModifiers::NONE, ShortcutKey::Home),
    Shortcut::new(ShortcutModifiers::NONE, ShortcutKey::End),
];
const WORKSPACE_GESTURES: [GuideGesture; 5] = [
    GuideGesture::new(
        "Show or hide controls",
        "Conceals or reveals the complete control sidebar.",
        &TOGGLE_CONTROLS,
    ),
    GuideGesture::new(
        "Next control group",
        "Moves focus to the next group in the sidebar.",
        &NEXT_CONTROL_GROUP,
    ),
    GuideGesture::new(
        "Previous control group",
        "Moves focus to the previous group in the sidebar.",
        &PREVIOUS_CONTROL_GROUP,
    ),
    GuideGesture::new(
        "Adjust density",
        "Changes the focused density value by one step; hovering it also admits the wheel.",
        &ADJUST_DENSITY,
    ),
    GuideGesture::new(
        "Density bounds",
        "Moves the focused density value directly to its minimum or maximum.",
        &DENSITY_BOUNDS,
    ),
];
const WORKSPACE_GUIDANCE: GuideSection =
    GuideSection::new("WORKSPACE CONTROLS", &WORKSPACE_GESTURES);
const DEMO_COMMANDS: [CommandSpec<DemoCommand, DemoScope>; 4] = [
    CommandSpec::new(
        DemoCommand::Open,
        "workspace.open",
        "Open archive",
        CommandScope::Global,
    )
    .with_detail("Selects an application-owned archive.")
    .with_default_shortcuts(&OPEN_KEYS)
    .with_mnemonic('O')
    .with_text_focus(TextFocusPolicy::Capture),
    CommandSpec::new(
        DemoCommand::Save,
        "workspace.save",
        "Save archive",
        CommandScope::Global,
    )
    .with_detail("Persists the active application document.")
    .with_default_shortcuts(&SAVE_KEYS)
    .with_mnemonic('S')
    .with_text_focus(TextFocusPolicy::Capture),
    CommandSpec::new(
        DemoCommand::Rename,
        "workspace.rename",
        "Rename selection",
        CommandScope::Context(DemoScope::Workspace),
    )
    .with_detail("Begins an in-place rename of the current selection.")
    .with_default_shortcuts(&RENAME_KEYS)
    .with_mnemonic('R'),
    CommandSpec::new(
        DemoCommand::FocusSearch,
        "workspace.focus_search",
        "Focus search",
        CommandScope::Context(DemoScope::Workspace),
    )
    .with_detail("Transfers keyboard focus to the search field.")
    .with_default_shortcuts(&SEARCH_KEYS)
    .with_mnemonic('F'),
];

struct CommandsExhibit {
    guide: CommandGuide,
    panels: PanelNavigator,
    selected: bool,
    filter: String,
    density: u16,
    focus_search: bool,
    status: String,
    scroll_offset: f32,
    inspector_expanded: bool,
    inspector_extent: f32,
}

impl Default for CommandsExhibit {
    fn default() -> Self {
        Self {
            guide: CommandGuide::default(),
            panels: PanelNavigator::default(),
            selected: true,
            filter: String::new(),
            density: 4,
            focus_search: false,
            status: "No command dispatched.".to_owned(),
            scroll_offset: 0.0,
            inspector_expanded: true,
            inspector_extent: eternalist_apps::inspector::WIDTH,
        }
    }
}

impl CommandsExhibit {
    fn show(&mut self, ui: &mut egui::Ui, water: &mut Surface) {
        let canon = demo_canon();
        if !self.guide.take_shortcuts(ui.ctx())
            && let Some(dispatch) = canon.route(ui.ctx(), &[DemoScope::Workspace], |command| {
                demo_status(command, self.selected)
            })
        {
            self.apply(dispatch);
        }

        let inspector = Inspector::new("command-atelier")
            .scroll_offset(self.scroll_offset)
            .show(ui, |ui| self.controls(ui, water));
        self.scroll_offset = inspector.scroll_offset;
        self.inspector_expanded = inspector.is_expanded();
        self.inspector_extent = inspector.visible_extent();
        record_response(
            ui,
            "atelier.commands.inspector-boundary",
            inspector.boundary(),
        );
        if let Some(actuator) = inspector.actuator() {
            record_response(ui, "atelier.commands.inspector-actuator", actuator);
        }
        inspector.agitate(water);
        if let Some(command) = inspector.inner {
            self.apply(CommandDispatch::Invoke(command));
        }

        let _stage = egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(chrome::PAGE)
                    .inner_margin(egui::Margin::same(28)),
            )
            .show(ui, |ui| {
                let _eyebrow = ui.label(chrome::eyebrow("TYPED APPLICATION CONSEQUENCE"));
                let _title = ui.label(chrome::title("COMMAND FOUNDRY"));
                let _law = ui.label(chrome::muted(
                    "one declaration routes keys, labels buttons, and renders help",
                ));
                ui.add_space(20.0);
                let _status = egui::Frame::new()
                    .fill(chrome::SURFACE)
                    .stroke(egui::Stroke::new(1.0_f32, chrome::EDGE_STRONG))
                    .inner_margin(egui::Margin::same(18))
                    .show(ui, |ui| {
                        let _label = ui.label(chrome::eyebrow("LAST DISPATCH"));
                        let _value = ui.label(chrome::section_title(self.status.to_uppercase()));
                    });
                ui.add_space(18.0);
                let _note = chrome::note(
                    ui,
                    "Use Tab within the lit panel. Control+Tab crosses panels. Alt mnemonics are permanently underlined.",
                );
                ui.add_space(18.0);
                let copy = ui.label(chrome::muted("COPY CAPABILITY SENTINEL"));
                record_response(ui, "atelier.commands.copy", &copy);
            });

        let selected = self.selected;
        self.guide.show(
            ui.ctx(),
            canon,
            &[DemoScope::Workspace],
            |_| "WORKSPACE",
            |command| demo_status(command, selected),
            &[WORKSPACE_GUIDANCE],
        );
        if let Some(rect) = self.guide.rect() {
            record_rect(ui.ctx(), "atelier.commands.guide", rect);
        }
    }

    fn controls(&mut self, ui: &mut egui::Ui, water: &mut Surface) -> Option<DemoCommand> {
        let _eyebrow = ui.label(chrome::eyebrow("KEYBOARD-COMPLETE INSPECTOR"));
        let _heading = ui.horizontal(|ui| {
            let _title = ui.label(chrome::title("COMMANDS"));
            let help = self.guide.activator(ui);
            record_response(ui, "atelier.commands.help", &help);
            water.monoglyph(&help);
        });
        let _law = ui.label(chrome::muted(
            "stable IDs leave a clean future keymap projection",
        ));
        ui.add_space(16.0);

        let mut invoked = None;
        let mut panels = self.panels.frame(ui.ctx());
        let file = panels.section(ui, "command-file", "FILE", true, |ui| {
            for command in [DemoCommand::Open, DemoCommand::Save] {
                let response = demo_canon().button(command, ui);
                record_response(ui, demo_target(command), &response);
                if response.clicked() {
                    invoked = Some(command);
                }
            }
        });
        record_response(ui, "atelier.commands.panel.file", &file.header);
        water.fold(file.wake);
        ui.add_space(10.0);

        let selection = panels.section(ui, "command-selection", "SELECTION", true, |ui| {
            let selected = Checkbox::new(&mut self.selected, "ITEM SELECTED").show(ui);
            record_response(ui, "atelier.commands.selected", &selected);
            water.checkbox(&selected);
            ui.add_space(8.0);
            let rename = ui
                .add_enabled_ui(self.selected, |ui| {
                    demo_canon().button(DemoCommand::Rename, ui)
                })
                .inner;
            record_response(ui, "atelier.commands.rename", &rename);
            if rename.clicked() {
                invoked = Some(DemoCommand::Rename);
            }
            ui.add_space(8.0);
            let before = self.filter.clone();
            let search = ui.add(
                egui::TextEdit::singleline(&mut self.filter)
                    .hint_text("search workspace")
                    .desired_width(ui.available_width()),
            );
            if self.focus_search {
                search.request_focus();
                self.focus_search = false;
            }
            record_response(ui, "atelier.commands.search", &search);
            if let Some(wake) = chrome::text_wake(ui, &search, &before, &self.filter) {
                water.text(wake);
            }
        });
        record_response(ui, "atelier.commands.panel.selection", &selection.header);
        water.fold(selection.wake);
        ui.add_space(10.0);

        let density = panels.section(ui, "command-density", "DENSITY", true, |ui| {
            let _value = ui.label(chrome::muted(format!("{} columns", self.density)));
            let rail = Rail::new(&mut self.density, 1..=8)
                .detents(8)
                .width(ui.available_width())
                .show(ui);
            record_response(ui, "atelier.commands.density", &rail);
            water.rail(&rail);
        });
        record_response(ui, "atelier.commands.panel.density", &density.header);
        water.fold(density.wake);
        drop(panels);
        ui.add_space(14.0);
        let _hint = chrome::note(
            ui,
            "F1 or ? opens the generated guide. Disabled commands retain their refusal reason.",
        );
        invoked
    }

    fn apply(&mut self, dispatch: CommandDispatch<'_, DemoCommand>) {
        match dispatch {
            CommandDispatch::Invoke(DemoCommand::Open) => {
                "opened archive".clone_into(&mut self.status);
            }
            CommandDispatch::Invoke(DemoCommand::Save) => {
                "saved archive".clone_into(&mut self.status);
            }
            CommandDispatch::Invoke(DemoCommand::Rename) => {
                "began selection rename".clone_into(&mut self.status);
            }
            CommandDispatch::Invoke(DemoCommand::FocusSearch) => {
                self.focus_search = true;
                "focused search".clone_into(&mut self.status);
            }
            CommandDispatch::Refused { reason, .. } => {
                self.status = format!("refused: {reason}");
            }
        }
    }
}

fn demo_canon() -> &'static CommandCanon<DemoCommand, DemoScope> {
    static CANON: OnceLock<CommandCanon<DemoCommand, DemoScope>> = OnceLock::new();
    CANON.get_or_init(|| CommandCanon::new(&DEMO_COMMANDS))
}

const fn demo_target(command: DemoCommand) -> &'static str {
    match command {
        DemoCommand::Open => "atelier.commands.open",
        DemoCommand::Save => "atelier.commands.save",
        DemoCommand::Rename => "atelier.commands.rename",
        DemoCommand::FocusSearch => "atelier.commands.search",
    }
}

#[inline]
fn record_response(ui: &egui::Ui, name: &'static str, response: &egui::Response) {
    #[cfg(all(
        feature = "egui-test",
        any(target_os = "linux", target_os = "macos", target_os = "windows")
    ))]
    egui_tester_witness::egui::record_response(ui, name, response);
    #[cfg(not(all(
        feature = "egui-test",
        any(target_os = "linux", target_os = "macos", target_os = "windows")
    )))]
    let _ = (ui, name, response);
}

#[inline]
fn record_rect(ctx: &egui::Context, name: &'static str, rect: egui::Rect) {
    #[cfg(all(
        feature = "egui-test",
        any(target_os = "linux", target_os = "macos", target_os = "windows")
    ))]
    egui_tester_witness::egui::record_rect(ctx, name, rect);
    #[cfg(not(all(
        feature = "egui-test",
        any(target_os = "linux", target_os = "macos", target_os = "windows")
    )))]
    let _ = (ctx, name, rect);
}

fn demo_status(command: DemoCommand, selected: bool) -> CommandStatus<'static> {
    if command == DemoCommand::Rename && !selected {
        CommandStatus::Disabled("select an item first")
    } else {
        CommandStatus::Enabled
    }
}

#[cfg(any(
    target_arch = "wasm32",
    target_os = "linux",
    target_os = "macos",
    target_os = "windows"
))]
fn main() -> Result<()> {
    #[cfg(all(target_os = "linux", feature = "egui-test"))]
    if std::env::args_os().nth(1).as_deref() == Some(std::ffi::OsStr::new(FOCUS_SENTINEL)) {
        return support::run(FocusSentinel);
    }
    #[cfg(all(target_os = "linux", feature = "egui-test"))]
    if std::env::args_os().nth(1).as_deref() == Some(std::ffi::OsStr::new(SHORT_WINDOW)) {
        return support::run(ShortAtelier(Atelier {
            page: Page::Settings,
            ..Atelier::default()
        }));
    }
    support::run(Atelier::default())
}

#[cfg(not(any(
    target_arch = "wasm32",
    target_os = "linux",
    target_os = "macos",
    target_os = "windows"
)))]
fn main() {}
