#![expect(
    unused_crate_dependencies,
    reason = "the atelier consumes the native and WebGPU host dependencies through its support module"
)]

mod support;

use anyhow::Result;
use dwemer_poolrooms::{
    chrome::{self, Checkbox, NumberInput, Rail, WheelPlane},
    egui,
    water::Surface,
};
use eternalist_apps::cabinet::{
    Cabinet, CabinetAction, CabinetEntry, CabinetKey, EntryEdit, Shelf, ShelfEdit,
};
use eternalist_apps::{Inspector, LivingWait};
use std::fmt::{Display, Formatter};
use support::Exhibit;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Page {
    Inspector,
    LivingWait,
    Cabinet,
}

impl Page {
    const ALL: [Self; 3] = [Self::Inspector, Self::LivingWait, Self::Cabinet];

    const fn label(self) -> &'static str {
        match self {
            Self::Inspector => "INSPECTOR",
            Self::LivingWait => "LIVING WAIT",
            Self::Cabinet => "CABINET",
        }
    }

    const fn number(self) -> &'static str {
        match self {
            Self::Inspector => "01",
            Self::LivingWait => "02",
            Self::Cabinet => "03",
        }
    }
}

struct Atelier {
    page: Page,
    inspector: InspectorExhibit,
    waiting: WaitingExhibit,
    cabinet: CabinetExhibit,
}

impl Default for Atelier {
    fn default() -> Self {
        Self {
            page: Page::Inspector,
            inspector: InspectorExhibit::default(),
            waiting: WaitingExhibit::default(),
            cabinet: CabinetExhibit::default(),
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

    fn ui(&mut self, ui: &mut egui::Ui, water: &mut Surface) {
        self.tabs(ui);
        match self.page {
            Page::Inspector => self.inspector.show(ui, water),
            Page::LivingWait => self.waiting.show(ui, water),
            Page::Cabinet => self.cabinet.show(ui, water),
        }
    }
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
            .show_inside(ui, |ui| {
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
        .min_size(egui::vec2(176.0, 30.0))
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
        water.heave(ui.ctx(), self.scroll_offset);

        let _stage = egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(chrome::PAGE)
                    .inner_margin(egui::Margin::same(28)),
            )
            .show_inside(ui, |ui| self.stage(ui));
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
            .show_inside(ui, |ui| {
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
            .show_inside(ui, |ui| self.stage(ui));
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
        water.heave(ui.ctx(), inspector.scroll_offset);
        self.apply(inspector.inner);

        let _stage = egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(chrome::PAGE)
                    .inner_margin(egui::Margin::same(28)),
            )
            .show_inside(ui, |ui| {
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

fn main() -> Result<()> {
    support::run(Atelier::default())
}
