use crate::{
    graphics::{Event, Graphics, SDL_GetTicks, SDL_PollEvent, SDL_WaitEventTimeout},
    platform::{Request, Worker},
    video::{Video, VideoEvent},
};
use aerial_native_probe::{
    catalogue::{self, Capabilities, Scene},
    model::*,
    redraw::{event_wait_ms, TimedVisuals},
    Direction,
};
use serde_json::{json, Value};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use std::{
    collections::{HashMap, HashSet},
    time::{Instant, SystemTime, UNIX_EPOCH},
};
const ID: &str = "io.github.justmaaarco.aerialhome.nativeprobe";
#[derive(Clone, PartialEq)]
enum Screen {
    Home,
    Apps,
    Search,
    Controls,
    Inputs,
    Settings,
    Clock,
    Aerials,
    ClockSettings,
    IconSettings,
    Oled,
    Sound,
    Sleep,
    Favorites,
    Hidden,
    Profile,
    Folder(String),
    Manage(String),
    FolderManage(String),
    Folders,
}
#[derive(Clone)]
enum Do {
    Screen(Screen),
    Launch(String),
    Cycle(&'static str),
    Toggle(&'static str),
    Next,
    Scene(String),
    Category(String),
    Volume(i16),
    Mute,
    Sleep(u16),
    Favorite(String),
    Hide(String),
    CreateFolder,
    FolderAdd(String, String),
    FolderRemove(String, String),
    RenameProfile,
    RenameFolder(String),
    DeleteFolder(String),
    Move(String, i32),
    EditHelper,
    Preview(u64),
    SaveText,
    Exit,
}
struct Row {
    title: String,
    detail: String,
    action: Do,
}
fn row(title: impl Into<String>, detail: impl Into<String>, action: Do) -> Row {
    Row {
        title: title.into(),
        detail: detail.into(),
        action,
    }
}
struct Ui {
    row_cache: RefCell<Option<Rc<Vec<Row>>>>,
    failed: HashSet<String>,
    prefs: Preferences,
    apps: Vec<App>,
    screen: Screen,
    menu: usize,
    home: HomeState,
    toolbar: Option<usize>,
    toolbar_left: Cell<i32>,
    home_offset: i32,
    grid_offset: i32,
    search_offset: i32,
    menu_offset: i32,
    query: String,
    editing: Option<String>,
    edit_search: bool,
    edit_folder: Option<String>,
    edit_helper: bool,
    keyboard_lower: bool,
    text_cursor: usize,
    oled_preview: Option<u64>,
    pointer_wake_until: u64,
    held_wake: Option<i32>,
    order_context: Vec<String>,
    toast: String,
    toast_until: u64,
    idle: IdleTracker,
    icons: HashMap<String, String>,
    icon_colors: HashMap<String, [u8; 4]>,
    home_return: HomeState,
    asked: HashSet<String>,
    scenes: Vec<Scene>,
    bag: ShuffleBag,
    caps: Capabilities,
    current: Option<Scene>,
    variants: Vec<catalogue::Variant>,
    variant: usize,
    video_playing: bool,
    video_loading: bool,
    video_since: u64,
    video_due: u64,
    helper_poll: u64,
    inventory_refreshed_at: u64,
    greeting_seed: u64,
    visible: bool,
    volume: u8,
    muted: bool,
    dirty: bool,
    worker: Worker,
    video: Option<Video>,
}
impl Ui {
    fn oled_state(&self, now: u64) -> OledState {
        if let Some(idle) = self.oled_preview {
            let mut prefs = self.prefs.clone();
            prefs.oled_protection = true;
            prefs.oled_dimming = true;
            oled_state(&prefs, idle, idle, true)
        } else {
            self.idle.state(now, &self.prefs, self.visible)
        }
    }
    fn refresh_inventory(&mut self, now: u64) {
        if now.saturating_sub(self.inventory_refreshed_at) >= 30_000 {
            self.inventory_refreshed_at = now;
            self.send(Request::Inventory);
            self.send(Request::Inputs);
        }
    }
    fn send(&self, r: Request) {
        let _ = self.worker.sender.try_send(r);
    }
    fn save(&mut self) {
        *self.row_cache.borrow_mut() = None;
        self.prefs.sanitize();
        let mut p = serde_json::to_value(&self.prefs).unwrap_or(json!({}));
        p["shuffleBag"] = serde_json::to_value(&self.bag).unwrap_or(Value::Null);
        self.send(Request::SavePreferences(p));
        self.dirty = true;
    }
    fn tell(&mut self, s: impl Into<String>, now: u64) {
        self.toast = s.into();
        self.toast_until = now + 5000;
        self.dirty = true;
    }
    fn open(&mut self, s: Screen) {
        if matches!(s, Screen::Manage(_)) {
            self.order_context = self.app_list().into_iter().map(|a| a.id).collect();
        }
        *self.row_cache.borrow_mut() = None;
        if s == Screen::Search {
            self.editing = Some(self.query.clone());
            self.edit_search = true;
            self.edit_helper = false;
            self.edit_folder = None;
            self.text_cursor = 0;
        }
        if self.screen == Screen::Home && s != Screen::Home {
            self.home_return = self.home.clone();
        }
        self.home = if s == Screen::Home {
            let mut home = self.home_return.clone();
            if home.input_row_open {
                home.back();
            }
            home
        } else {
            HomeState::new()
        };
        self.screen = s;
        self.menu = 0;
        self.toolbar = None;
        self.grid_offset = 0;
        self.search_offset = 0;
        self.menu_offset = 0;
        self.dirty = true;
        if self.screen == Screen::Sound {
            self.send(Request::AudioStatus);
        }
        self.reveal_focus();
    }
    /// Explicit keyboard navigation may scroll a view. Pointer hover only
    /// changes focus, so the content under a stationary pointer stays put.
    fn reveal_focus(&mut self) {
        let count = self.app_list().len();
        self.home.clamp(count, self.input_list().len());
        match self.screen {
            Screen::Home => {
                let last = count
                    .div_ceil(self.prefs.columns.max(1))
                    .max(1)
                    .saturating_sub(1) as i32;
                let end = if self.home.input_row_open {
                    input_top(self, count) + 151
                } else {
                    446 + last * 263 + 213 + 50
                };
                self.home_offset = self.home_offset.min((end + 150 - 940).max(0));
                let (top, bottom) = if self.home.input_row_open {
                    let top = input_top(self, count);
                    (top, top + 151)
                } else if let Focus::App(i) = self.home.focus {
                    let top = 446 + (i / self.prefs.columns.max(1)) as i32 * 263;
                    (top, top + 213)
                } else {
                    return;
                };
                self.home_offset = reveal_offset(self.home_offset, top, bottom, 0, 940);
            }
            Screen::Apps | Screen::Folder(_) => {
                let last = count.div_ceil(4).max(1).saturating_sub(1) as i32;
                self.grid_offset = self
                    .grid_offset
                    .min((178 + last * 248 + 213 + 20 - 994).max(0));
                if let Focus::App(i) = self.home.focus {
                    let top = 178 + (i / 4) as i32 * 248;
                    self.grid_offset =
                        reveal_offset(self.grid_offset, top - 20, top + 233, 158, 994);
                }
            }
            Screen::Search => {
                let last = count.div_ceil(2).max(1).saturating_sub(1) as i32;
                self.search_offset = self.search_offset.min((244 + last * 86 + 76 - 994).max(0));
                if let Focus::App(i) = self.home.focus {
                    let top = 244 + (i / 2) as i32 * 86;
                    self.search_offset = reveal_offset(self.search_offset, top, top + 76, 244, 994);
                }
            }
            Screen::Clock => {}
            _ => {
                let (positions, _, height) = menu_positions(self);
                self.menu_offset = self.menu_offset.min(
                    positions
                        .last()
                        .map(|y| (y + 76 - (48 + height - 36)).max(0))
                        .unwrap_or(0),
                );
                if let Some(top) = positions.get(self.menu) {
                    self.menu_offset =
                        reveal_offset(self.menu_offset, *top, *top + 76, 158, 48 + height - 36);
                }
            }
        }
    }
    fn input_list(&self) -> Vec<App> {
        self.apps.iter().filter(|a| a.input).cloned().collect()
    }
    fn app_list(&self) -> Vec<App> {
        match &self.screen {
            Screen::Home => home_apps(&self.apps, &self.prefs)
                .into_iter()
                .filter(|app| !app.input)
                .collect(),
            Screen::Folder(id) => visible_apps(&self.apps, &self.prefs, "", Some(id)),
            Screen::Search => visible_apps(&self.apps, &self.prefs, &self.query, None)
                .into_iter()
                .filter(|a| !a.input)
                .collect(),
            _ => visible_apps(&self.apps, &self.prefs, "", None)
                .into_iter()
                .filter(|a| !a.input)
                .collect(),
        }
    }
    fn val(&self, key: &str) -> String {
        serde_json::to_value(&self.prefs)
            .ok()
            .and_then(|v| v[key].as_str().map(str::to_owned))
            .unwrap_or_default()
    }
    fn rows(&self) -> Rc<Vec<Row>> {
        if let Some(rows) = self.row_cache.borrow().as_ref() {
            return rows.clone();
        }
        use Do::{
            Category, CreateFolder, Cycle, Exit, Favorite, FolderAdd, FolderRemove, Hide, Launch,
            Mute, Next, RenameProfile, Sleep, Toggle, Volume,
        };
        let p = &self.prefs;
        let mut r = Vec::new();
        match &self.screen {
            Screen::Controls => {
                r.push(row(&p.profile, "Profile", Do::Screen(Screen::Profile)));
                for (t, s) in [
                    ("All apps", Screen::Apps),
                    ("Sound", Screen::Sound),
                    ("Sleep timer", Screen::Sleep),
                    ("Inputs", Screen::Inputs),
                    ("Aerials", Screen::Aerials),
                    ("Screen saver", Screen::Clock),
                    ("Settings", Screen::Settings),
                ] {
                    r.push(row(t, "", Do::Screen(s)));
                }
            }
            Screen::Inputs => {
                for app in self.input_list() {
                    r.push(row(app.title, "", Launch(app.id)));
                }
            }
            Screen::Settings => {
                r = vec![
                    row("Appearance", self.val("appearance"), Cycle("appearance")),
                    row(
                        "App size",
                        if p.columns == 6 {
                            "Standard · 6 columns"
                        } else {
                            "Large · 5 columns"
                        },
                        Cycle("columns"),
                    ),
                    row("Reduce motion", on(p.reduce_motion), Toggle("reduceMotion")),
                    row(
                        "Clock style",
                        p.clock_style.label(),
                        Do::Screen(Screen::ClockSettings),
                    ),
                    row(
                        "Icon style",
                        p.icon_style.label(),
                        Do::Screen(Screen::IconSettings),
                    ),
                    row("Aerials & screen saver", "", Do::Screen(Screen::Aerials)),
                    row(
                        "OLED care",
                        "Static element protection",
                        Do::Screen(Screen::Oled),
                    ),
                    row("Apps on Home", p.home_mode.label(), Cycle("homeMode")),
                    row("Favourites", "Choose apps", Do::Screen(Screen::Favorites)),
                    row("New folder", "", CreateFolder),
                    row(
                        "Hidden apps",
                        p.hidden.len().to_string(),
                        Do::Screen(Screen::Hidden),
                    ),
                    row("Manage folders", "", Do::Screen(Screen::Folders)),
                    row("Profile name", &p.profile, Do::Screen(Screen::Profile)),
                    row("LG TV settings", "", Launch("com.palm.app.settings".into())),
                    row("Exit native launcher", "", Exit),
                ];
            }
            Screen::ClockSettings => {
                for (s, k) in [
                    (ClockStyle::Classic, "clock:classic"),
                    (ClockStyle::Minimal, "clock:minimal"),
                    (ClockStyle::Glass, "clock:glass"),
                    (ClockStyle::Immersive, "clock:immersive"),
                ] {
                    r.push(row(
                        s.label(),
                        if p.clock_style == s { "Selected" } else { "" },
                        Cycle(k),
                    ));
                }
                r.push(row(
                    "Time format",
                    p.clock_format.label(),
                    Cycle("clockFormat"),
                ));
                r.push(row("Show date", on(p.clock_date), Toggle("clockDate")));
                r.push(row("Show full-screen clock", "", Do::Screen(Screen::Clock)));
            }
            Screen::IconSettings => {
                for (s, k) in [
                    (IconStyle::Classic, "icon:classic"),
                    (IconStyle::Glass, "icon:glass"),
                    (IconStyle::Minimal, "icon:minimal"),
                    (IconStyle::Monochrome, "icon:mono"),
                ] {
                    r.push(row(
                        s.label(),
                        if p.icon_style == s { "Selected" } else { "" },
                        Cycle(k),
                    ));
                }
                r.push(row(
                    "App labels",
                    self.val("iconLabels"),
                    Cycle("iconLabels"),
                ));
            }
            Screen::Aerials => {
                r = vec![
                    row("Moving aerials", on(p.aerial), Toggle("aerial")),
                    row("Next aerial", "", Next),
                    row("Shuffle", on(p.shuffle), Toggle("shuffle")),
                    row("Quality", p.quality.label(), Cycle("quality")),
                    row(
                        "Start delay",
                        format!("{} seconds", p.aerial_delay),
                        Cycle("aerialDelay"),
                    ),
                    row(
                        "Screen saver",
                        format!("{} minutes", p.saver_minutes),
                        Cycle("saverMinutes"),
                    ),
                    row(
                        "Compatibility helper",
                        if p.media_server.is_empty() {
                            "Not configured"
                        } else {
                            &p.media_server
                        },
                        Do::EditHelper,
                    ),
                ];
                for cat in ["Cities", "Landscapes", "Space", "Underwater"] {
                    r.push(row(
                        cat,
                        on(p.categories.iter().any(|c| c == cat)),
                        Category(cat.into()),
                    ));
                }
                for s in &self.scenes {
                    if p.categories.contains(&s.category)
                        && !catalogue::choose(s, &self.val("quality"), self.caps, &p.media_server)
                            .is_empty()
                    {
                        r.push(row(&s.title, &s.category, Do::Scene(s.id.clone())));
                    }
                }
            }
            Screen::Oled => {
                r = vec![
                    row(
                        "Protect static elements",
                        on(p.oled_protection),
                        Toggle("oledProtection"),
                    ),
                    row(
                        "Dim after 30 idle minutes",
                        on(p.oled_dimming),
                        Toggle("oledDimming"),
                    ),
                    row(
                        "Preview faded UI",
                        "First input restores the UI",
                        Do::Preview(120000),
                    ),
                    row(
                        "Preview hidden UI",
                        "First input restores the UI",
                        Do::Preview(300000),
                    ),
                    row(
                        "Preview 30-minute dimming",
                        "First input restores the UI",
                        Do::Preview(1800000),
                    ),
                ];
            }
            Screen::Sound => {
                r = vec![
                    row("Volume up", self.volume.to_string(), Volume(5)),
                    row("Volume down", self.volume.to_string(), Volume(-5)),
                    row(if self.muted { "Unmute" } else { "Mute" }, "", Mute),
                ];
            }
            Screen::Sleep => {
                for m in [0, 15, 30, 60, 90] {
                    r.push(row(
                        if m == 0 {
                            "Cancel sleep timer".into()
                        } else {
                            format!("{m} minutes")
                        },
                        "",
                        Sleep(m),
                    ));
                }
            }
            Screen::Favorites => {
                for a in self.apps.iter().filter(|a| !a.input && !a.utility) {
                    r.push(row(
                        &a.title,
                        if p.favorites.contains(&a.id) {
                            "Favourite"
                        } else {
                            ""
                        },
                        Favorite(a.id.clone()),
                    ));
                }
            }
            Screen::Hidden => {
                for a in self.apps.iter().filter(|a| !a.input) {
                    r.push(row(
                        &a.title,
                        if p.hidden.contains(&a.id) {
                            "Hidden"
                        } else {
                            "Visible"
                        },
                        Hide(a.id.clone()),
                    ));
                }
            }
            Screen::Manage(id) => {
                if let Some(a) = self.apps.iter().find(|a| &a.id == id) {
                    r.push(row(format!("Open {}", a.title), "", Launch(id.clone())));
                    r.push(row(
                        "Favourite",
                        on(p.favorites.contains(id)),
                        Favorite(id.clone()),
                    ));
                    r.push(row("Hidden", on(p.hidden.contains(id)), Hide(id.clone())));
                    r.push(row("Move left / earlier", "", Do::Move(id.clone(), -1)));
                    r.push(row("Move right / later", "", Do::Move(id.clone(), 1)));
                    for f in &p.folders {
                        if f.apps.contains(id) {
                            r.push(row(
                                format!("Remove from {}", f.title),
                                "",
                                FolderRemove(f.id.clone(), id.clone()),
                            ));
                        } else {
                            r.push(row(
                                format!("Add to {}", f.title),
                                "",
                                FolderAdd(f.id.clone(), id.clone()),
                            ));
                        }
                    }
                }
            }
            Screen::FolderManage(id) => {
                if let Some(f) = p.folders.iter().find(|f| &f.id == id) {
                    r.push(row(
                        "Open folder",
                        &f.title,
                        Do::Screen(Screen::Folder(id.clone())),
                    ));
                    r.push(row("Rename folder", &f.title, Do::RenameFolder(id.clone())));
                    r.push(row(
                        "Delete folder",
                        "Keeps all installed apps",
                        Do::DeleteFolder(id.clone()),
                    ));
                }
            }
            Screen::Folders => {
                r.push(row("Create folder", "", CreateFolder));
                for f in &p.folders {
                    r.push(row(
                        &f.title,
                        format!("{} apps", f.apps.len()),
                        Do::Screen(Screen::FolderManage(f.id.clone())),
                    ));
                }
            }
            Screen::Profile => r.push(row("Edit profile name", &p.profile, RenameProfile)),
            _ => {}
        }
        let r = Rc::new(r);
        *self.row_cache.borrow_mut() = Some(r.clone());
        r
    }
    fn act(&mut self, a: Do, now: u64) {
        *self.row_cache.borrow_mut() = None;
        match a {
            Do::Screen(s) => {
                if s == Screen::Search {
                    self.query.clear();
                }
                self.open(s);
            }
            Do::Launch(id) => {
                if let Some(v) = &self.video {
                    let _ = v.set_visible(false);
                }
                self.send(Request::Launch(id));
                self.tell("Opening app…", now);
            }
            Do::Cycle(k) => {
                let mut p = serde_json::to_value(&self.prefs).unwrap();
                let keys: Vec<Value> = match k {
                    "appearance" => vec![json!("dark"), json!("light"), json!("auto")],
                    "columns" => vec![json!(6), json!(5)],
                    "clockFormat" => vec![json!("auto"), json!("12"), json!("24")],
                    "iconLabels" => vec![json!("focused"), json!("always")],
                    "homeMode" => vec![json!("most-used"), json!("favorites"), json!("all")],
                    "quality" => vec![json!("auto"), json!("dolby"), json!("hdr"), json!("sdr")],
                    "aerialDelay" => vec![json!(0), json!(3), json!(5), json!(10)],
                    "saverMinutes" => vec![json!(0), json!(1), json!(3), json!(5), json!(10)],
                    _ => vec![],
                };
                if let Some(s) = k.strip_prefix("clock:") {
                    p["clockStyle"] = json!(s);
                } else if let Some(s) = k.strip_prefix("icon:") {
                    p["iconStyle"] = json!(s);
                } else if !keys.is_empty() {
                    let i = keys.iter().position(|v| v == &p[k]).unwrap_or(0);
                    p[k] = keys[(i + 1) % keys.len()].clone();
                }
                self.prefs = serde_json::from_value(p).unwrap_or_default();
                self.save();
            }
            Do::Toggle(k) => {
                let mut p = serde_json::to_value(&self.prefs).unwrap();
                p[k] = json!(!p[k].as_bool().unwrap_or(false));
                self.prefs = serde_json::from_value(p).unwrap_or_default();
                if k == "aerial" {
                    if self.prefs.aerial {
                        self.video_due = now + 500;
                    } else {
                        self.stop_video();
                    }
                }
                self.save();
            }
            Do::Next => self.next_video(now),
            Do::Scene(id) => {
                self.prefs.scene_id = id.clone();
                self.save();
                self.load_scene(&id, now);
            }
            Do::Category(c) => {
                if self.prefs.categories.contains(&c) {
                    self.prefs.categories.retain(|v| v != &c);
                } else {
                    self.prefs.categories.push(c);
                }
                self.save();
            }
            Do::Volume(d) => {
                self.volume = (self.volume as i16 + d).clamp(0, 100) as u8;
                self.send(Request::SetVolume(self.volume));
                self.dirty = true;
            }
            Do::Mute => {
                self.muted = !self.muted;
                self.send(Request::SetMute(self.muted));
                self.dirty = true;
            }
            Do::Sleep(m) => {
                self.send(Request::SleepTimer(m));
                self.tell("Sleep timer updated", now);
            }
            Do::Favorite(id) => {
                if self.prefs.favorites.contains(&id) {
                    self.prefs.favorites.retain(|v| v != &id);
                } else {
                    self.prefs.favorites.push(id);
                }
                self.save();
            }
            Do::Hide(id) => {
                if self.prefs.hidden.contains(&id) {
                    self.prefs.hidden.retain(|v| v != &id);
                } else {
                    self.prefs.hidden.push(id);
                }
                self.save();
            }
            Do::CreateFolder => {
                let id = format!("folder-{}", unix_ms());
                self.prefs.folders.push(Folder {
                    id,
                    title: format!("Folder {}", self.prefs.folders.len() + 1),
                    apps: vec![],
                });
                self.save();
                self.tell("Open app options to add it to a folder", now);
            }
            Do::FolderAdd(f, id) => {
                if let Some(f) = self.prefs.folders.iter_mut().find(|x| x.id == f) {
                    f.apps.push(id);
                }
                self.save();
            }
            Do::FolderRemove(f, id) => {
                if let Some(f) = self.prefs.folders.iter_mut().find(|x| x.id == f) {
                    f.apps.retain(|x| x != &id);
                }
                self.save();
            }
            Do::RenameFolder(id) => {
                if let Some(f) = self.prefs.folders.iter().find(|f| f.id == id) {
                    self.editing = Some(f.title.clone());
                    self.edit_folder = Some(id);
                    self.edit_search = false;
                    self.edit_helper = false;
                    self.keyboard_lower = false;
                    self.text_cursor = 0;
                }
            }
            Do::DeleteFolder(id) => {
                self.prefs.folders.retain(|f| f.id != id);
                self.prefs.order.retain(|v| v != &id);
                self.prefs.hidden.retain(|v| v != &id);
                self.save();
                self.open(Screen::Folders);
            }
            Do::Move(id, delta) => {
                let mut ids = self.order_context.clone();
                if !ids.contains(&id) {
                    ids = self
                        .apps
                        .iter()
                        .filter(|a| !a.input)
                        .map(|a| a.id.clone())
                        .collect();
                }
                if move_order(&mut self.prefs.order, &ids, &id, delta) {
                    self.save();
                    self.tell("App order saved", now);
                }
            }
            Do::EditHelper => {
                self.editing = Some(self.prefs.media_server.clone());
                self.edit_search = false;
                self.edit_folder = None;
                self.edit_helper = true;
                self.keyboard_lower = true;
                self.text_cursor = 0;
            }
            Do::Preview(idle) => {
                self.oled_preview = Some(idle);
                self.dirty = true;
            }
            Do::RenameProfile => {
                self.edit_search = false;
                self.edit_folder = None;
                self.edit_helper = false;
                self.keyboard_lower = false;
                self.text_cursor = 0;
                self.editing = Some(self.prefs.profile.clone());
                self.tell("Type a name, then OK to save", now);
            }
            Do::SaveText => {
                let value = self.editing.take().unwrap_or_default();
                if self.edit_helper {
                    if value.is_empty() || valid_media_server(&value) {
                        self.prefs.media_server = value;
                        self.edit_helper = false;
                        self.failed.clear();
                        self.stop_video();
                        self.video_due = now + 500;
                        self.save();
                    } else {
                        self.editing = Some(value);
                        self.tell("Use http://LAN-address:port or leave empty", now);
                    }
                } else if let Some(id) = self.edit_folder.take() {
                    if let Some(f) = self.prefs.folders.iter_mut().find(|f| f.id == id) {
                        if !value.trim().is_empty() {
                            f.title = value;
                        }
                    }
                    self.save();
                } else if self.edit_search {
                    self.query = value;
                    self.home = HomeState::new();
                    self.search_offset = 0;
                    self.dirty = true;
                } else {
                    self.prefs.profile = value;
                    self.save();
                }
            }
            Do::Exit => {}
        }
    }
    fn stop_video(&mut self) {
        if let Some(v) = &self.video {
            let _ = v.stop();
        }
        self.video_playing = false;
        self.video_loading = false;
        self.helper_poll = 0;
        self.dirty = true;
    }
    fn next_video(&mut self, now: u64) {
        if self.video.is_none() {
            self.tell("Native aerial backend is unavailable", now);
            return;
        }
        let ids: Vec<_> = self
            .scenes
            .iter()
            .filter(|s| {
                self.prefs.categories.contains(&s.category)
                    && !catalogue::choose(
                        s,
                        &self.val("quality"),
                        self.caps,
                        &self.prefs.media_server,
                    )
                    .is_empty()
            })
            .map(|s| s.id.clone())
            .collect();
        for _ in 0..ids.len() {
            if let Some(id) = self.bag.next(&ids, self.prefs.shuffle) {
                if !self.failed.contains(&id) {
                    self.load_scene(&id, now);
                    return;
                }
            }
        }
        self.tell("No compatible aerials available. Retrying shortly.", now);
        self.failed.clear();
        self.video_due = now + 60000;
    }
    fn load_scene(&mut self, id: &str, now: u64) {
        if self.video.is_none() {
            self.tell("Native aerial backend is unavailable", now);
            return;
        }
        self.stop_video();
        if let Some(s) = self.scenes.iter().find(|s| s.id == id).cloned() {
            self.variants = catalogue::choose(
                &s,
                &self.val("quality"),
                self.caps,
                &self.prefs.media_server,
            );
            self.variant = 0;
            self.prefs.scene_id = s.id.clone();
            self.current = Some(s);
            self.save();
            self.play_variant(now);
        } else {
            self.current = None;
            self.prefs.scene_id.clear();
            self.video_due = now + 1000;
        }
    }
    fn play_variant(&mut self, now: u64) {
        self.video_playing = false;
        self.helper_poll = 0;
        if !self.visible || !self.prefs.aerial {
            return;
        }
        let Some(v) = self.variants.get(self.variant) else {
            if let Some(s) = &self.current {
                self.failed.insert(s.id.clone());
            }
            self.video_loading = false;
            self.video_due = now + 1000;
            return;
        };
        self.video_since = now;
        self.video_loading = true;
        if v.fps > 60.0 {
            if let Some(s) = &self.current {
                self.send(Request::PrepareAerial {
                    helper: self.prefs.media_server.clone(),
                    scene_id: s.id.clone(),
                });
                self.helper_poll = now + 2500;
            }
        } else if let Some(video) = &self.video {
            if let Err(e) = video.load(&v.url) {
                self.tell(e, now);
            }
        }
        self.dirty = true;
    }
    fn replies(&mut self, now: u64) {
        while let Ok(r) = self.worker.receiver.try_recv() {
            match r.result {
                Ok(v) => match r.request {
                    Request::LoadPreferences => {
                        *self.row_cache.borrow_mut() = None;
                        if let Ok(bag) = serde_json::from_value(v["shuffleBag"].clone()) {
                            self.bag = bag;
                        }
                        self.prefs = serde_json::from_value(v.clone()).unwrap_or_default();
                        self.prefs.sanitize();
                        if let Some(a) = v["cachedApps"].as_array() {
                            self.merge_apps(a);
                        }
                        self.video_due = now + self.prefs.aerial_delay * 1000;
                        self.dirty = true;
                    }
                    Request::Inventory => {
                        if let Some(a) = v["apps"].as_array() {
                            let have_inputs = self.apps.iter().any(|app| app.input);
                            self.apps.retain(|app| app.input);
                            let fresh: Vec<_> = a
                                .iter()
                                .filter(|app| !have_inputs || app["input"] != true)
                                .cloned()
                                .collect();
                            self.merge_apps(&fresh);
                        }
                        self.dirty = true;
                    }
                    Request::Inputs => {
                        if let Some(a) = v["apps"].as_array() {
                            self.apps.retain(|app| !app.input);
                            self.merge_apps(a);
                        }
                        self.dirty = true;
                    }
                    Request::Artwork { id, .. } => {
                        if let Some(color) = v["color"].as_str().and_then(hex_color) {
                            self.icon_colors.insert(id.clone(), color);
                            self.dirty = true;
                        }
                        if let Some(p) = v["path"].as_str() {
                            self.icons.insert(id, p.into());
                            self.dirty = true;
                        }
                    }
                    Request::Favorites => {
                        if self.prefs.favorites.is_empty() {
                            if let Some(a) = v["favorites"].as_array() {
                                self.prefs.favorites = a
                                    .iter()
                                    .filter_map(Value::as_str)
                                    .map(str::to_owned)
                                    .collect();
                                *self.row_cache.borrow_mut() = None;
                                self.dirty = true;
                            }
                        }
                    }
                    Request::Capabilities => {
                        *self.row_cache.borrow_mut() = None;
                        self.caps = Capabilities {
                            dolby: v["dolbyVision"] == true,
                            hdr: v["hdr10"] == true,
                        };
                        self.dirty = true;
                    }
                    Request::Launch(id) => {
                        self.prefs.record_launch(&id, unix_ms());
                        self.save();
                        self.visible = false;
                        self.stop_video();
                    }
                    Request::AudioStatus => {
                        *self.row_cache.borrow_mut() = None;
                        self.volume = v["volume"].as_u64().unwrap_or(20) as u8;
                        self.muted = v["muted"] == true;
                        self.dirty = true;
                    }
                    Request::PrepareAerial { scene_id, .. } => {
                        if self.visible
                            && self.prefs.aerial
                            && self.current.as_ref().is_some_and(|s| s.id == scene_id)
                            && self.video_loading
                            && self
                                .variants
                                .get(self.variant)
                                .is_some_and(|v| v.fps > 60.0)
                        {
                            if v["state"] == "ready" {
                                if let (Some(url), Some(video)) = (v["url"].as_str(), &self.video) {
                                    let _ = video.load(url);
                                    self.helper_poll = 0;
                                }
                            } else {
                                self.helper_poll = now + 2500;
                            }
                        }
                    }
                    _ => {}
                },
                Err(e) => {
                    if matches!(&r.request, Request::Launch(_)) && self.visible {
                        self.stop_video();
                        if let Some(video) = &self.video {
                            let _ = video.set_visible(true);
                        }
                        self.video_due = now + 500;
                    }
                    if let Request::PrepareAerial { scene_id, .. } = &r.request {
                        if self.visible
                            && self.prefs.aerial
                            && self.video_loading
                            && self.current.as_ref().is_some_and(|s| s.id == *scene_id)
                            && self
                                .variants
                                .get(self.variant)
                                .is_some_and(|v| v.fps > 60.0)
                        {
                            self.variant += 1;
                            self.play_variant(now);
                        }
                    } else if !matches!(r.request, Request::Artwork { .. } | Request::Favorites) {
                        self.tell(e, now);
                    }
                }
            }
        }
    }
    fn merge_apps(&mut self, a: &[Value]) {
        *self.row_cache.borrow_mut() = None;
        for v in a {
            let Some(id) = v["id"].as_str() else {
                continue;
            };
            if let Some(color) = v["color"].as_str().and_then(hex_color) {
                self.icon_colors.insert(id.into(), color);
            }
            let app = App {
                id: id.into(),
                title: v["title"].as_str().unwrap_or(id).into(),
                icon: v["icon"].as_str().unwrap_or("").into(),
                input: v["input"] == true,
                utility: id.contains("settings") || id.contains("hbchannel"),
                folder: false,
            };
            if let Some(old) = self.apps.iter_mut().find(|x| x.id == id) {
                if old.icon != app.icon {
                    self.icons.remove(id);
                    self.asked.remove(id);
                }
                *old = app;
            } else {
                self.apps.push(app);
            }
        }
    }
    fn request_artwork(&mut self) {
        for a in self
            .app_list()
            .into_iter()
            .skip(if self.screen == Screen::Home {
                0
            } else {
                match self.home.focus {
                    Focus::App(i) => i / self.prefs.columns / 2 * 2 * self.prefs.columns,
                    _ => 0,
                }
            })
            .take(12)
        {
            if !a.icon.is_empty() && !self.asked.contains(&a.id) {
                if self
                    .worker
                    .sender
                    .try_send(Request::Artwork {
                        id: a.id.clone(),
                        path: a.icon,
                    })
                    .is_ok()
                {
                    self.asked.insert(a.id);
                }
            }
        }
    }
    fn video_events(&mut self, now: u64) {
        let events = self.video.as_ref().map(|v| v.poll()).unwrap_or_default();
        for event in events {
            println!("{}", json!({"event":"video","state":format!("{:?}",event)}));
            match event {
                VideoEvent::Playing => {
                    self.video_playing = true;
                    self.video_loading = false;
                    self.dirty = true;
                }
                VideoEvent::Ended => self.next_video(now),
                VideoEvent::Error(e) => {
                    self.tell(format!("Aerial: {e}"), now);
                    if self.visible && self.prefs.aerial {
                        self.variant += 1;
                        self.play_variant(now);
                    }
                }
                _ => {}
            }
        }
        if self.helper_poll > 0 && now >= self.helper_poll && self.video_loading {
            self.helper_poll = now + 2500;
            if let Some(s) = &self.current {
                self.send(Request::PrepareAerial {
                    helper: self.prefs.media_server.clone(),
                    scene_id: s.id.clone(),
                });
            }
        }
        if self.video_loading && now.saturating_sub(self.video_since) > 45000 {
            self.variant += 1;
            self.play_variant(now);
        }
        if self.video.is_some()
            && self.prefs.aerial
            && self.visible
            && !self.video_loading
            && !self.video_playing
            && now >= self.video_due
        {
            self.video_due = u64::MAX;
            let saved = self.prefs.scene_id.clone();
            if self.current.is_none() && !saved.is_empty() && !self.prefs.shuffle {
                self.load_scene(&saved, now);
            } else {
                self.next_video(now);
            }
        }
    }
}
fn on(v: bool) -> &'static str {
    if v {
        "On"
    } else {
        "Off"
    }
}
fn unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
fn title(s: &Screen) -> &str {
    match s {
        Screen::Home => "Home",
        Screen::Apps => "All apps",
        Screen::Search => "Search apps",
        Screen::Controls => "Control Center",
        Screen::Inputs => "Inputs",
        Screen::Clock => "Clock",
        Screen::ClockSettings => "Clock style",
        Screen::IconSettings => "Icon style",
        Screen::Aerials => "Aerials",
        Screen::Oled => "OLED protection",
        Screen::Sound => "Sound",
        Screen::Sleep => "Sleep timer",
        Screen::Favorites => "Favourites",
        Screen::Hidden => "Hidden apps",
        Screen::Profile => "Profile",
        Screen::Folder(_) => "Folder",
        Screen::FolderManage(_) => "Folder options",
        Screen::Manage(_) => "App options",
        Screen::Folders => "Folders",
        _ => "Settings",
    }
}
// These dimensions follow app/style.css's 1920x1080 logical stage.
#[derive(Clone, Copy)]
struct UiRect {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}
impl UiRect {
    fn contains(self, x: i32, y: i32) -> bool {
        x >= self.x && y >= self.y && x < self.x + self.w && y < self.y + self.h
    }
}
fn panel_open(screen: &Screen) -> bool {
    !matches!(screen, Screen::Home | Screen::Clock)
}
fn wide_panel(screen: &Screen) -> bool {
    matches!(screen, Screen::Apps | Screen::Search | Screen::Folder(_))
}
fn grid_columns(u: &Ui) -> usize {
    match u.screen {
        Screen::Home => u.prefs.columns,
        Screen::Search => 2,
        _ => 4,
    }
}
fn tile_rect(columns: usize, col: usize, row: usize, wide: bool, scroll: i32) -> UiRect {
    let (left, top, total, gap, step) = if wide {
        (366.0, 178, 1188.0, 35.0, 248)
    } else {
        (96.0, 446, 1728.0, 40.0, 263)
    };
    let width = (total - gap * columns.saturating_sub(1) as f64) / columns.max(1) as f64;
    let x = left + col as f64 * (width + gap);
    UiRect {
        x: x.round() as i32,
        y: top + row as i32 * step - scroll,
        w: (x + width).round() as i32 - x.round() as i32,
        h: 158,
    }
}
fn home_scroll(u: &Ui, _count: usize) -> i32 {
    u.home_offset
}
fn input_top(u: &Ui, count: usize) -> i32 {
    let rows = count.div_ceil(u.prefs.columns.max(1)).max(1);
    673 + rows.saturating_sub(1) as i32 * 263
}
fn grid_scroll(u: &Ui) -> i32 {
    u.grid_offset
}
fn reveal_offset(
    offset: i32,
    top: i32,
    bottom: i32,
    viewport_top: i32,
    viewport_bottom: i32,
) -> i32 {
    if top - offset < viewport_top {
        (top - viewport_top).max(0)
    } else if bottom - offset > viewport_bottom {
        (bottom - viewport_bottom).max(0)
    } else {
        offset.max(0)
    }
}
fn panel_section(screen: &Screen, row: usize) -> Option<&'static str> {
    match (screen, row) {
        (Screen::Controls, 2) => Some("TV"),
        (Screen::Controls, 5) => Some("PERSONALIZE"),
        (Screen::Settings, 0) => Some("APPEARANCE"),
        (Screen::Settings, 7) => Some("HOME SCREEN"),
        (Screen::Settings, 14) => Some("ABOUT"),
        _ => None,
    }
}
fn menu_positions(u: &Ui) -> (Vec<i32>, i32, i32) {
    let rows = u.rows();
    let mut y = 158;
    let mut positions = Vec::new();
    for (i, _) in rows.iter().enumerate() {
        if panel_section(&u.screen, i).is_some() {
            y += 62;
        }
        positions.push(y);
        y += 86;
    }
    let height = (y - 10 + 36 - 48).clamp(220, 982);
    (positions, u.menu_offset, height)
}
fn hex_color(s: &str) -> Option<[u8; 4]> {
    if s.len() != 7 || !s.starts_with('#') || !s[1..].bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    Some([
        u8::from_str_radix(&s[1..3], 16).ok()?,
        u8::from_str_radix(&s[3..5], 16).ok()?,
        u8::from_str_radix(&s[5..7], 16).ok()?,
        255,
    ])
}
unsafe fn centered(g: &mut Graphics, s: &str, size: i32, weight: u16, r: UiRect, color: [u8; 4]) {
    let tracking = if weight == 500 && (size == 25 || size == 20) {
        -0.5
    } else {
        0.0
    };
    let width = g.text_width_tracking(s, size, weight, tracking).min(r.w);
    g.text_tracking(
        s,
        size,
        weight,
        tracking,
        r.x + (r.w - width) / 2,
        r.y,
        color,
        r.w,
    );
}
unsafe fn stroke_line(g: &Graphics, x1: i32, y1: i32, x2: i32, y2: i32, c: [u8; 4]) {
    g.line(x1, y1, x2, y2, c);
    if (x2 - x1).abs() >= (y2 - y1).abs() {
        g.line(x1, y1 + 1, x2, y2 + 1, c);
    } else {
        g.line(x1 + 1, y1, x2 + 1, y2, c);
    }
}
unsafe fn glyph(g: &Graphics, name: &str, r: UiRect, c: [u8; 4]) {
    let px = |v: i32| r.x + v * r.w / 24;
    let py = |v: i32| r.y + v * r.h / 24;
    let line = |a, b, d, e| stroke_line(g, px(a), py(b), px(d), py(e), c);
    match name {
        "apps" => {
            for (x, y) in [(3, 3), (14, 3), (3, 14), (14, 14)] {
                g.round_stroke(px(x), py(y), 7 * r.w / 24, 7 * r.h / 24, 2, 2, c);
            }
        }
        "search" => {
            g.round_stroke(
                px(3),
                py(3),
                13 * r.w / 24,
                13 * r.h / 24,
                7 * r.w / 24,
                2,
                c,
            );
            line(15, 15, 21, 21);
        }
        "controls" => {
            for (y, x) in [(6, 8), (12, 16), (18, 10)] {
                line(3, y, 21, y);
                g.round(px(x) - 3, py(y) - 3, 7, 7, 4, c);
            }
        }
        "close" => {
            line(6, 6, 18, 18);
            line(18, 6, 6, 18);
        }
        "input" => {
            g.round_stroke(px(4), py(4), 16 * r.w / 24, 16 * r.h / 24, 3, 2, c);
            line(2, 12, 13, 12);
            line(9, 8, 13, 12);
            line(9, 16, 13, 12);
        }
        "clock" => {
            g.round_stroke(
                px(2),
                py(2),
                20 * r.w / 24,
                20 * r.h / 24,
                10 * r.w / 24,
                2,
                c,
            );
            line(12, 6, 12, 12);
            line(12, 12, 16, 15);
        }
        "person" => {
            g.round_stroke(px(8), py(3), 8 * r.w / 24, 8 * r.h / 24, 4 * r.w / 24, 2, c);
            g.round_stroke(px(4), py(14), 16 * r.w / 24, 8 * r.h / 24, 6, 2, c);
        }
        "volume" => {
            line(3, 9, 7, 9);
            line(7, 9, 12, 5);
            line(12, 5, 12, 19);
            line(12, 19, 7, 15);
            line(7, 15, 3, 15);
            line(3, 15, 3, 9);
            line(17, 8, 19, 10);
            line(19, 10, 19, 14);
            line(19, 14, 17, 16);
        }
        "moon" => {
            g.round_stroke(
                px(3),
                py(3),
                18 * r.w / 24,
                18 * r.h / 24,
                9 * r.w / 24,
                2,
                c,
            );
            line(12, 3, 9, 10);
            line(9, 10, 12, 16);
            line(12, 16, 20, 17);
        }
        "picture" => {
            g.round_stroke(px(2), py(3), 20 * r.w / 24, 18 * r.h / 24, 3, 2, c);
            line(3, 18, 9, 11);
            line(9, 11, 14, 16);
            line(14, 16, 18, 12);
            line(18, 12, 22, 17);
        }
        "folder" => {
            g.round_stroke(px(2), py(6), 20 * r.w / 24, 15 * r.h / 24, 3, 2, c);
            line(3, 6, 3, 3);
            line(3, 3, 9, 3);
            line(9, 3, 12, 6);
        }
        "check" => {
            line(4, 12, 10, 18);
            line(10, 18, 21, 6);
        }
        "settings" => {
            g.round_stroke(
                px(3),
                py(3),
                18 * r.w / 24,
                18 * r.h / 24,
                9 * r.w / 24,
                2,
                c,
            );
            g.round_stroke(px(9), py(9), 6 * r.w / 24, 6 * r.h / 24, 3 * r.w / 24, 2, c);
            for (a, b, d, e) in [
                (12, 0, 12, 4),
                (12, 20, 12, 24),
                (0, 12, 4, 12),
                (20, 12, 24, 12),
            ] {
                line(a, b, d, e);
            }
        }
        _ => {
            line(8, 5, 19, 12);
            line(19, 12, 8, 19);
            line(8, 19, 8, 5);
        }
    }
}
fn row_glyph(r: &Row) -> &'static str {
    let title = r.title.to_lowercase();
    if r.detail == "Profile" {
        "person"
    } else if title.contains("sound") || title.contains("volume") || title.contains("mute") {
        "volume"
    } else if title.contains("sleep") || title.contains("oled") || title.contains("saver") {
        "moon"
    } else if title.contains("clock") || title.contains("time") || title.contains("date") {
        "clock"
    } else if title.contains("input") || title.contains("hdmi") {
        "input"
    } else if title.contains("profile") {
        "person"
    } else if title.contains("folder") {
        "folder"
    } else if title.contains("search") {
        "search"
    } else if title.contains("aerial") || title.contains("appearance") {
        "picture"
    } else if title.contains("settings") {
        "settings"
    } else {
        "apps"
    }
}
unsafe fn draw_app(g: &mut Graphics, u: &Ui, a: &App, r: UiRect, focused: bool, panel: bool) {
    let scale = if focused { 1.11 } else { 1.0 };
    let (w, h) = (
        (r.w as f32 * scale).round() as i32,
        (r.h as f32 * scale).round() as i32,
    );
    let (x, y) = (r.x - (w - r.w) / 2, r.y - (h - r.h) / 2);
    let radius = (28.0 * scale).round() as i32;
    let color = u
        .icon_colors
        .get(&a.id)
        .copied()
        .unwrap_or([57, 75, 104, 255]);
    if focused {
        g.round(x - 5, y + 9, w + 10, h + 10, radius + 5, [0, 0, 0, 55]);
    }
    match u.prefs.icon_style {
        IconStyle::Classic => g.round(x, y, w, h, radius, color),
        IconStyle::Glass => g.round_gradient(
            x,
            y,
            w,
            h,
            radius,
            [255, 255, 255, if focused { 102 } else { 51 }],
            [185, 217, 255, if focused { 59 } else { 33 }],
        ),
        IconStyle::Minimal => g.round(
            x,
            y,
            w,
            h,
            radius,
            if g.light {
                [239, 246, 255, 85]
            } else {
                [24, 43, 59, 77]
            },
        ),
        IconStyle::Monochrome => {
            g.round_gradient(x, y, w, h, radius, [68, 74, 82, 255], [39, 45, 52, 255])
        }
    }
    let ratio = match u.prefs.icon_style {
        IconStyle::Glass => 0.74,
        IconStyle::Minimal => 0.78,
        _ => 1.0,
    };
    let (iw, ih) = (
        (w as f32 * ratio).round() as i32,
        (h as f32 * ratio).round() as i32,
    );
    let (ix, iy) = (x + (w - iw) / 2, y + (h - ih) / 2);
    let image_radius = if ratio == 1.0 {
        radius
    } else {
        ((if u.prefs.icon_style == IconStyle::Glass {
            16.0
        } else {
            12.0
        }) * scale) as i32
    };
    if a.folder {
        if let Some(folder) = u.prefs.folders.iter().find(|f| f.id == a.id) {
            let dot_w = (w - 44 - 24) / 3;
            for (i, id) in folder.apps.iter().take(6).enumerate() {
                if let Some(app) = u.apps.iter().find(|app| &app.id == id) {
                    let dr = UiRect {
                        x: x + 22 + (i % 3) as i32 * (dot_w + 12),
                        y: y + 22 + (i / 3) as i32 * 56,
                        w: dot_w,
                        h: 44,
                    };
                    g.round(dr.x, dr.y, dr.w, dr.h, 10, [255, 255, 255, 112]);
                    centered(
                        g,
                        &app.title.chars().take(1).collect::<String>(),
                        15,
                        700,
                        UiRect { y: dr.y + 11, ..dr },
                        [36, 52, 75, 255],
                    );
                }
            }
        }
    } else if !u.icons.get(&a.id).is_some_and(|p| {
        g.icon_rounded(
            p,
            ix,
            iy,
            iw,
            ih,
            image_radius,
            u.prefs.icon_style == IconStyle::Monochrome,
        )
    }) {
        glyph(
            g,
            if a.input { "input" } else { "play" },
            UiRect {
                x: x + w / 2 - 38,
                y: y + h / 2 - 38,
                w: 76,
                h: 76,
            },
            [255, 255, 255, 245],
        );
    }
    g.round_stroke(
        x,
        y,
        w,
        h,
        radius,
        1,
        [255, 255, 255, if focused { 128 } else { 33 }],
    );
    if focused {
        g.round_stroke(
            x - 3,
            y - 3,
            w + 6,
            h + 6,
            radius + 3,
            3,
            [
                255,
                255,
                255,
                if u.prefs.icon_style == IconStyle::Glass {
                    224
                } else {
                    160
                },
            ],
        );
    }
    if focused || panel || u.prefs.icon_labels == IconLabels::Always {
        centered(
            g,
            &a.title,
            if panel { 20 } else { 25 },
            500,
            UiRect {
                y: r.y + r.h + 23,
                ..r
            },
            [255, 255, 255, 255],
        );
    }
}
unsafe fn glass_numeral(
    g: &mut Graphics,
    s: &str,
    size: i32,
    weight: u16,
    x: i32,
    y: i32,
    _full: bool,
) {
    g.text_glass(s, size, weight, x, y, size);
}
unsafe fn draw_clock(g: &mut Graphics, u: &Ui, full: bool) {
    let face = clock_face(crate::clock::parts(), &u.prefs, false);
    let (size, weight, tracking) = match (u.prefs.clock_style, full) {
        (ClockStyle::Classic, true) => (90, 550, -4),
        (ClockStyle::Minimal, false) => (176, 260, -9),
        (ClockStyle::Minimal, true) => (270, 240, -14),
        (ClockStyle::Glass, false) => (186, 650, -10),
        (ClockStyle::Glass, true) => (310, 650, -17),
        (ClockStyle::Immersive, false) => (268, 720, -11),
        (ClockStyle::Immersive, true) => (430, 720, -18),
        _ => return,
    };
    let period_size = if full && u.prefs.clock_style != ClockStyle::Classic {
        44
    } else if u.prefs.clock_style == ClockStyle::Immersive {
        34
    } else {
        28
    };
    let digit_width = (b'0'..=b'9')
        .map(|d| g.text_width(&(d as char).to_string(), size, weight))
        .max()
        .unwrap_or(size / 2);
    let widths: Vec<_> = face
        .time
        .chars()
        .map(|c| {
            if c.is_ascii_digit() {
                digit_width
            } else {
                g.text_width(&c.to_string(), size, weight)
            }
        })
        .collect();
    let time_width =
        widths.iter().sum::<i32>() + tracking * (widths.len().saturating_sub(1) as i32);
    let period_width = if face.period.is_empty() {
        0
    } else {
        18 + g.text_width(&face.period, period_size, 550)
    };
    let center = full && u.prefs.clock_style != ClockStyle::Classic
        || !full && u.prefs.clock_style == ClockStyle::Immersive;
    let mut x = if center {
        (1920 - time_width - period_width) / 2
    } else {
        96
    };
    let line_height = if !full && u.prefs.clock_style == ClockStyle::Immersive {
        size
    } else {
        (size * 104 + 50) / 100
    };
    let gap = if full && u.prefs.clock_style == ClockStyle::Classic {
        14
    } else {
        20
    };
    let date_height = if full && u.prefs.clock_style == ClockStyle::Classic {
        31
    } else {
        42
    };
    let place_height = if full && u.prefs.clock_style == ClockStyle::Classic {
        29
    } else {
        26
    };
    let group_height = line_height
        + if u.prefs.clock_date {
            gap + date_height
        } else {
            0
        }
        + if u.current.is_some() {
            gap + place_height
        } else {
            0
        };
    let top = if full {
        if center {
            (1080 - group_height) / 2
        } else {
            1080 - 92 - group_height
        }
    } else if u.prefs.clock_style == ClockStyle::Immersive {
        102
    } else {
        148
    };
    // CSS clock line-height is tighter than the font's normal ascender box.
    let y = top - size * 8 / 100;
    for (ch, width) in face.time.chars().zip(widths) {
        let glyph_x = x + (width - g.text_width(&ch.to_string(), size, weight)) / 2;
        if u.prefs.clock_style == ClockStyle::Glass {
            glass_numeral(g, &ch.to_string(), size, weight, glyph_x, y, full);
        } else {
            g.text_weight(
                &ch.to_string(),
                size,
                weight,
                glyph_x,
                y,
                [255, 255, 255, 255],
                size,
            );
        }
        x += width + tracking;
    }
    if !face.period.is_empty() {
        g.text_weight(
            &face.period,
            period_size,
            550,
            x - tracking + 18,
            top + size - period_size - 4,
            [255, 255, 255, 245],
            180,
        );
    }
    if u.prefs.clock_date {
        let date_size = if full && u.prefs.clock_style != ClockStyle::Classic {
            32
        } else if u.prefs.clock_style == ClockStyle::Immersive {
            25
        } else {
            24
        };
        let dr = UiRect {
            x: 96,
            y: top
                + line_height
                + if full {
                    gap
                } else if u.prefs.clock_style == ClockStyle::Immersive {
                    10
                } else {
                    12
                },
            w: 1728,
            h: 45,
        };
        let tracking = if full && u.prefs.clock_style != ClockStyle::Classic {
            1.0
        } else if u.prefs.clock_style == ClockStyle::Immersive {
            2.0
        } else {
            0.1
        };
        let width = g.text_width_tracking(&face.date, date_size, 450, tracking);
        g.text_tracking(
            &face.date,
            date_size,
            450,
            tracking,
            if center { (1920 - width) / 2 } else { dr.x },
            dr.y,
            [241, 246, 255, 223],
            1728,
        );
    }
    if full {
        if let Some(scene) = &u.current {
            let place_y = top
                + line_height
                + gap
                + if u.prefs.clock_date {
                    date_height + gap
                } else {
                    0
                };
            let r = UiRect {
                x: 96,
                y: place_y,
                w: 1728,
                h: 34,
            };
            if center {
                centered(g, &scene.title, 21, 400, r, [255, 255, 255, 191]);
            } else {
                g.text_weight(&scene.title, 24, 400, r.x, r.y, [255, 255, 255, 255], 1500);
            }
        }
    }
}
unsafe fn draw_toolbar(g: &mut Graphics, u: &Ui) {
    let ink = if g.light {
        [20, 32, 51, 255]
    } else {
        [255, 255, 255, 255]
    };
    g.round_stroke(92, 68, 27, 27, 14, 2, ink);
    g.round(94, 82, 23, 11, 5, ink);
    g.text_tracking(&u.prefs.profile, 25, 550, -0.6, 132, 66, ink, 900);
    let classic = u.prefs.clock_style == ClockStyle::Classic;
    let face = clock_face(crate::clock::parts(), &u.prefs, false);
    let time = if face.period.is_empty() {
        face.time
    } else {
        format!("{} {}", face.time, face.period)
    };
    let clock_width = g.text_width(&time, 27, 500).max(105);
    let start = if classic { 1590 - clock_width } else { 1606 };
    u.toolbar_left.set(start);
    for (i, name) in ["apps", "search", "controls"].iter().enumerate() {
        let x = start + i as i32 * 80;
        let focused = u.screen == Screen::Home && u.toolbar == Some(i);
        let r = if focused {
            UiRect {
                x: x - 3,
                y: 52,
                w: 70,
                h: 58,
            }
        } else {
            UiRect {
                x,
                y: 54,
                w: 64,
                h: 54,
            }
        };
        g.round(
            r.x,
            r.y,
            r.w,
            r.h,
            30,
            if focused {
                [255, 255, 255, 255]
            } else if g.light {
                [228, 237, 251, 184]
            } else {
                [63, 73, 91, 164]
            },
        );
        if !focused {
            g.round_stroke(r.x, r.y, r.w, r.h, 30, 1, [255, 255, 255, 74]);
        }
        glyph(
            g,
            name,
            UiRect {
                x: x + 18,
                y: 67,
                w: 27,
                h: 27,
            },
            if focused { [25, 35, 51, 255] } else { ink },
        );
    }
    if classic {
        let w = g.text_width(&time, 27, 500);
        g.text_weight(&time, 27, 500, 1830 - w, 65, ink, w + 2);
    }
}
unsafe fn draw_home(g: &mut Graphics, u: &Ui) {
    let apps: Vec<_> = home_apps(&u.apps, &u.prefs)
        .into_iter()
        .filter(|a| !a.input)
        .collect();
    let scroll = home_scroll(u, apps.len());
    g.set_clip(Some((0, 0, 1920, 940)));
    if u.prefs.clock_style == ClockStyle::Classic {
        let hello = greeting(crate::clock::parts().hour, u.greeting_seed, None);
        g.text_tracking(
            hello.title,
            52,
            650,
            -1.9,
            96,
            236 - scroll,
            [255, 255, 255, 255],
            1728,
        );
        g.text_tracking(
            hello.phrase,
            23,
            420,
            -0.3,
            96,
            315 - scroll,
            if g.light {
                [36, 57, 80, 173]
            } else {
                [240, 243, 250, 201]
            },
            1728,
        );
    } else {
        let shift = g.shift;
        g.shift.1 -= scroll;
        draw_clock(g, u, false);
        g.shift = shift;
    }
    for (i, app) in apps.iter().enumerate() {
        let r = tile_rect(
            u.prefs.columns,
            i % u.prefs.columns,
            i / u.prefs.columns,
            false,
            scroll,
        );
        if r.y + r.h + 55 < 0 || r.y > 940 {
            continue;
        }
        draw_app(
            g,
            u,
            app,
            r,
            u.screen == Screen::Home && u.toolbar.is_none() && u.home.focus == Focus::App(i),
            false,
        );
    }
    if u.screen == Screen::Home && u.home.input_row_open {
        let top = input_top(u, apps.len()) - scroll;
        for (i, app) in u.input_list().iter().enumerate() {
            let r = UiRect {
                x: 102 + i as i32 * 252,
                y: top,
                w: 228,
                h: 88,
            };
            let focus = u.home.focus == Focus::Input(i);
            let face = if focus {
                UiRect {
                    x: r.x - 6,
                    y: r.y - 2,
                    w: 240,
                    h: 92,
                }
            } else {
                r
            };
            let radius = if focus { 21 } else { 20 };
            match u.prefs.icon_style {
                IconStyle::Classic | IconStyle::Monochrome if focus => {
                    g.round(face.x, face.y, face.w, face.h, radius, [245, 248, 255, 255])
                }
                IconStyle::Classic => {
                    g.round(face.x, face.y, face.w, face.h, radius, [35, 48, 66, 230])
                }
                IconStyle::Glass => g.round_gradient(
                    face.x,
                    face.y,
                    face.w,
                    face.h,
                    radius,
                    [255, 255, 255, if focus { 102 } else { 51 }],
                    [185, 217, 255, if focus { 59 } else { 33 }],
                ),
                IconStyle::Minimal => g.round(
                    face.x,
                    face.y,
                    face.w,
                    face.h,
                    radius,
                    if focus {
                        [255, 255, 255, if g.light { 161 } else { 38 }]
                    } else if g.light {
                        [239, 246, 255, 85]
                    } else {
                        [24, 43, 59, 77]
                    },
                ),
                IconStyle::Monochrome => g.round_gradient(
                    face.x,
                    face.y,
                    face.w,
                    face.h,
                    radius,
                    [68, 74, 82, 255],
                    [39, 45, 52, 255],
                ),
            }
            if focus {
                g.round_stroke(
                    face.x - 2,
                    face.y - 2,
                    face.w + 4,
                    face.h + 4,
                    radius + 2,
                    if u.prefs.icon_style == IconStyle::Classic {
                        2
                    } else {
                        3
                    },
                    [
                        255,
                        255,
                        255,
                        if u.prefs.icon_style == IconStyle::Classic {
                            160
                        } else {
                            224
                        },
                    ],
                );
            }
            let dark = focus
                && matches!(
                    u.prefs.icon_style,
                    IconStyle::Classic | IconStyle::Monochrome
                )
                || g.light;
            glyph(
                g,
                "input",
                UiRect {
                    x: r.x + 94,
                    y: r.y + 24,
                    w: 40,
                    h: 40,
                },
                if dark {
                    [29, 41, 61, 255]
                } else {
                    [255, 255, 255, 255]
                },
            );
            centered(
                g,
                &app.title,
                19,
                500,
                UiRect { y: r.y + 101, ..r },
                [255, 255, 255, 255],
            );
        }
    }
    g.set_clip(None);
    if let Some(scene) = &u.current {
        g.round(96, 981, 7, 7, 4, [255, 255, 255, 255]);
        g.text_weight(&scene.title, 20, 500, 115, 972, [255, 255, 255, 255], 1200);
        if let Some(v) = u.variants.get(u.variant) {
            let quality = format!(
                "{} · {}",
                if v.height >= 2160 { "4K" } else { "2K" },
                if v.dynamic_range.is_empty() {
                    v.range.to_uppercase()
                } else {
                    v.dynamic_range.to_uppercase()
                }
            );
            let w = g.text_width(&scene.title, 20, 500);
            g.text_weight(
                &quality,
                12,
                600,
                115 + w + 20,
                980,
                [255, 255, 255, 140],
                500,
            );
        }
    }
    let footer = if g.light {
        [36, 57, 80, 173]
    } else {
        [255, 255, 255, 127]
    };
    g.text_weight(
        "↓ Inputs · ↑ Control Center",
        17,
        400,
        96,
        1020,
        footer,
        1000,
    );
    let w = g.text_width("Aerial Home", 17, 400);
    g.text_weight("Aerial Home", 17, 400, 1824 - w, 1020, footer, w + 2);
    draw_toolbar(g, u);
}
unsafe fn draw_panel(g: &mut Graphics, u: &Ui) {
    let wide = wide_panel(&u.screen);
    let (positions, scroll, height) = if wide {
        (vec![], 0, 982)
    } else {
        menu_positions(u)
    };
    let x = if wide { 310 } else { 1268 };
    let width = if wide { 1300 } else { 590 };
    g.round_gradient(
        x,
        48,
        width,
        height,
        42,
        [58, 70, 91, 242],
        [27, 37, 56, 240],
    );
    g.round_stroke(x, 48, width, height, 42, 1, [255, 255, 255, 74]);
    let heading = if let Screen::Folder(id) = &u.screen {
        u.prefs
            .folders
            .iter()
            .find(|f| &f.id == id)
            .map(|f| f.title.as_str())
            .unwrap_or("Folder")
    } else {
        title(&u.screen)
    };
    g.text_tracking(
        heading,
        31,
        650,
        -0.8,
        x + 36,
        89,
        [255, 255, 255, 255],
        width - 150,
    );
    let close = UiRect {
        x: x + width - 84,
        y: 84,
        w: 48,
        h: 48,
    };
    let focused = u.toolbar == Some(3);
    g.round(
        close.x,
        close.y,
        48,
        48,
        24,
        if focused {
            [245, 248, 255, 255]
        } else {
            [255, 255, 255, 32]
        },
    );
    glyph(
        g,
        "close",
        UiRect {
            x: close.x + 10,
            y: close.y + 10,
            w: 28,
            h: 28,
        },
        if focused {
            [29, 41, 61, 255]
        } else {
            [255, 255, 255, 255]
        },
    );
    g.set_clip(Some((x + 36, 158, width - 72, height - 146)));
    if matches!(u.screen, Screen::Apps | Screen::Folder(_)) {
        let apps = u.app_list();
        let scroll = grid_scroll(u);
        for (i, app) in apps.iter().enumerate() {
            let r = tile_rect(4, i % 4, i / 4, true, scroll);
            if r.y + 213 < 158 || r.y > 994 {
                continue;
            }
            draw_app(
                g,
                u,
                app,
                r,
                u.toolbar.is_none() && u.home.focus == Focus::App(i),
                true,
            );
        }
        if apps.is_empty() {
            g.text_weight(
                "No matching apps",
                28,
                400,
                x + 56,
                190,
                [212, 220, 233, 255],
                width - 112,
            );
        }
    } else if u.screen == Screen::Search {
        g.set_clip(Some((x + 20, 140, width - 40, height - 128)));
        g.round(x + 36, 158, width - 72, 70, 20, [12, 20, 42, 51]);
        g.round_stroke(x + 36, 158, width - 72, 70, 20, 1, [255, 255, 255, 80]);
        if u.toolbar == Some(4) {
            g.round_stroke(x + 27, 149, width - 54, 88, 29, 4, [248, 250, 255, 255]);
        }
        g.text_weight(
            if u.query.is_empty() {
                "Find an app or input"
            } else {
                &u.query
            },
            25,
            400,
            x + 60,
            178,
            [255, 255, 255, 255],
            width - 120,
        );
        g.set_clip(Some((x + 36, 244, width - 72, 750)));
        let apps = u.app_list();
        let skip = (u.search_offset.max(0) / 86) as usize * 2;
        for (i, app) in apps.iter().enumerate().skip(skip).take(20) {
            let r = UiRect {
                x: x + 36 + (i % 2) as i32 * 619,
                y: 244 + (i / 2) as i32 * 86 - u.search_offset,
                w: 609,
                h: 76,
            };
            draw_control(
                g,
                &app.title,
                if app.input { "Input" } else { "" },
                "play",
                r,
                u.toolbar.is_none() && u.home.focus == Focus::App(i),
                false,
            );
        }
        if apps.is_empty() {
            g.text_weight(
                "No matching apps",
                18,
                400,
                x + 44,
                264,
                [212, 220, 233, 255],
                width - 88,
            );
        }
    } else {
        for (i, r) in u.rows().iter().enumerate() {
            let y = positions[i] - scroll;
            if let Some(section) = panel_section(&u.screen, i) {
                if y - 34 >= 158 {
                    g.text_weight(
                        section,
                        17,
                        550,
                        x + 44,
                        y - 34,
                        [212, 220, 233, 255],
                        width - 88,
                    );
                }
            }
            if y + 76 < 158 || y > 48 + height - 36 {
                continue;
            }
            draw_control(
                g,
                &r.title,
                &r.detail,
                row_glyph(r),
                UiRect {
                    x: x + 36,
                    y,
                    w: width - 72,
                    h: 76,
                },
                u.toolbar.is_none() && u.menu == i,
                r.detail == "Selected",
            );
        }
    }
    g.set_clip(None);
}
unsafe fn draw_control(
    g: &mut Graphics,
    label: &str,
    detail: &str,
    icon: &str,
    r: UiRect,
    focused: bool,
    selected: bool,
) {
    g.round(
        r.x,
        r.y,
        r.w,
        r.h,
        21,
        if focused {
            [245, 248, 255, 255]
        } else {
            [255, 255, 255, if selected { 48 } else { 26 }]
        },
    );
    let ink = if focused {
        [29, 41, 61, 255]
    } else {
        [255, 255, 255, 255]
    };
    glyph(
        g,
        icon,
        UiRect {
            x: r.x + 22,
            y: r.y + 24,
            w: 27,
            h: 27,
        },
        ink,
    );
    let detail_width = g.text_width(detail, 17, 400).min(210);
    let label_width = (r.w
        - 22
        - 27
        - 18
        - 22
        - if detail.is_empty() {
            0
        } else {
            detail_width + 18
        })
    .max(60);
    g.text_weight(label, 23, 400, r.x + 67, r.y + 24, ink, label_width);
    if !detail.is_empty() {
        g.text_weight(
            detail,
            17,
            400,
            r.x + r.w - 22 - detail_width,
            r.y + 28,
            if focused {
                [82, 97, 120, 255]
            } else {
                [212, 220, 233, 255]
            },
            detail_width,
        );
    }
}
unsafe fn draw(g: &mut Graphics, u: &Ui, now: u64) {
    g.light = u.prefs.appearance == Appearance::Light
        || (u.prefs.appearance == Appearance::Auto
            && (7..19).contains(&crate::clock::parts().hour));
    g.begin(u.video_playing);
    // The production web wash is translucent and never becomes an opaque floor.
    let saver = u.screen == Screen::Clock;
    if g.light {
        g.vertical_gradient(
            0,
            0,
            1920,
            1080,
            [220, 231, 255, if saver { 17 } else { 85 }],
            [192, 206, 230, if saver { 16 } else { 80 }],
        );
    } else {
        g.vertical_gradient(
            0,
            0,
            1920,
            324,
            [5, 12, 25, if saver { 4 } else { 20 }],
            [8, 16, 32, if saver { 5 } else { 26 }],
        );
        g.vertical_gradient(
            0,
            324,
            1920,
            756,
            [8, 16, 32, if saver { 5 } else { 26 }],
            [5, 12, 27, if saver { 34 } else { 168 }],
        );
    }
    let oled = u.oled_state(now);
    g.alpha = oled.ui_alpha;
    g.shift = (oled.x, oled.y);
    if u.screen == Screen::Clock {
        draw_clock(g, u, true);
    } else {
        draw_home(g, u);
        if panel_open(&u.screen) {
            draw_panel(g, u);
        }
    }
    if now < u.toast_until {
        let w = (g.text_width(&u.toast, 22, 400) + 64).clamp(200, 1100);
        let x = (1920 - w) / 2;
        g.round(x, 904, w, 66, 24, [24, 36, 58, 239]);
        g.text_weight(&u.toast, 22, 400, x + 32, 924, [255, 255, 255, 255], w - 64);
    }
    if let Some(s) = &u.editing {
        g.round_gradient(
            180,
            300,
            1560,
            550,
            42,
            [58, 70, 91, 250],
            [27, 37, 56, 250],
        );
        g.text_weight(s, 32, 400, 230, 335, [255, 255, 255, 255], 1400);
        for i in 0..50 {
            let x = 230 + (i % 10) as i32 * 148;
            let y = 445 + (i / 10) as i32 * 70;
            let focus = i == u.text_cursor;
            g.round(
                x,
                y,
                130,
                55,
                18,
                if focus {
                    [245, 248, 255, 255]
                } else {
                    [255, 255, 255, 26]
                },
            );
            centered(
                g,
                &keyboard_label(i, u.keyboard_lower),
                22,
                500,
                UiRect {
                    x,
                    y: y + 14,
                    w: 130,
                    h: 55,
                },
                if focus {
                    [29, 41, 61, 255]
                } else {
                    [255, 255, 255, 255]
                },
            );
        }
    }
    g.set_clip(None);
    g.alpha = 255;
    g.shift = (0, 0);
    if oled.overlay_alpha > 0 {
        g.rect(0, 0, 1920, 1080, [0, 0, 0, oled.overlay_alpha]);
    }
    g.present();
}

fn make_ui(worker: Worker, scenes: Vec<Scene>, video: Option<Video>) -> Ui {
    Ui {
        row_cache: RefCell::new(None),
        failed: HashSet::new(),
        prefs: Preferences::default(),
        apps: vec![],
        screen: Screen::Home,
        menu: 0,
        home: HomeState::new(),
        toolbar: None,
        toolbar_left: Cell::new(1485),
        home_offset: 0,
        grid_offset: 0,
        search_offset: 0,
        menu_offset: 0,
        query: String::new(),
        editing: None,
        edit_search: false,
        edit_folder: None,
        edit_helper: false,
        keyboard_lower: false,
        text_cursor: 0,
        oled_preview: None,
        pointer_wake_until: 0,
        held_wake: None,
        order_context: vec![],
        toast: String::new(),
        toast_until: 0,
        idle: IdleTracker::new(0),
        icons: HashMap::new(),
        icon_colors: HashMap::new(),
        home_return: HomeState::new(),
        asked: HashSet::new(),
        scenes,
        bag: ShuffleBag::new(unix_ms()),
        caps: Capabilities::default(),
        current: None,
        variants: vec![],
        variant: 0,
        video_playing: false,
        video_loading: false,
        video_since: 0,
        video_due: 4000,
        helper_poll: 0,
        inventory_refreshed_at: 0,
        greeting_seed: unix_ms(),
        visible: true,
        volume: 20,
        muted: false,
        dirty: true,
        worker,
        video,
    }
}

struct PreviewFixture {
    apps: Vec<Value>,
    icons: HashMap<String, String>,
    favorites: Vec<String>,
}
fn load_preview_fixture(path: &std::path::Path) -> Result<PreviewFixture, String> {
    use std::{fs, io::Read, path::Component};
    const MAX_FIXTURE: u64 = 2 * 1024 * 1024;
    const MAX_ICON: u64 = 4 * 1024 * 1024;
    let metadata = fs::symlink_metadata(path).map_err(|e| format!("Preview fixture: {e}"))?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > MAX_FIXTURE {
        return Err("Preview fixture must be a regular JSON file up to 2 MiB".into());
    }
    let canonical = path.canonicalize().map_err(|e| e.to_string())?;
    let root = canonical
        .parent()
        .ok_or("Preview fixture has no parent directory")?;
    let mut bytes = Vec::new();
    fs::File::open(&canonical)
        .map_err(|e| e.to_string())?
        .take(MAX_FIXTURE + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_FIXTURE {
        return Err("Preview fixture exceeds 2 MiB".into());
    }
    let value: Value =
        serde_json::from_slice(&bytes).map_err(|e| format!("Invalid preview fixture JSON: {e}"))?;
    let apps = value["apps"]
        .as_array()
        .filter(|a| !a.is_empty() && a.len() <= 260)
        .ok_or("Preview fixture needs 1–260 apps")?;
    let mut ids = HashSet::new();
    let mut clean = Vec::new();
    for app in apps {
        let id = app["id"]
            .as_str()
            .filter(|s| valid_app_id(s))
            .ok_or("Invalid preview app ID")?;
        let title = app["title"]
            .as_str()
            .filter(|s| !s.trim().is_empty() && s.chars().count() <= 160 && !s.contains('\0'))
            .ok_or("Invalid preview app title")?;
        if !ids.insert(id.to_owned()) {
            return Err("Duplicate preview app ID".into());
        }
        let input = match app.get("input") {
            None => false,
            Some(v) => v.as_bool().ok_or("Invalid preview input flag")?,
        };
        if input && !valid_input_id(id) {
            return Err("Preview input has an invalid HDMI app ID".into());
        }
        let color = match app.get("color") {
            None => "#394b68",
            Some(v) => v
                .as_str()
                .filter(|s| hex_color(s).is_some())
                .ok_or("Invalid preview app color")?,
        };
        clean.push(json!({"id":id,"title":title,"input":input,"color":color,"icon":""}));
    }
    let icon_values = value["icons"]
        .as_object()
        .filter(|icons| icons.len() <= 260)
        .ok_or("Preview icons must be a bounded path map")?;
    let mut icons = HashMap::new();
    for (id, relative) in icon_values {
        if !ids.contains(id) {
            return Err("Preview icon refers to an unknown app".into());
        }
        let relative = relative
            .as_str()
            .filter(|s| !s.is_empty() && s.len() <= 1024)
            .ok_or("Invalid preview icon path")?;
        let relative = std::path::Path::new(relative);
        if relative.is_absolute()
            || relative
                .components()
                .any(|p| !matches!(p, Component::Normal(_)))
        {
            return Err("Preview icons must use relative paths without traversal".into());
        }
        let mut file = root.to_path_buf();
        for part in relative.components() {
            file.push(part.as_os_str());
            let metadata = fs::symlink_metadata(&file).map_err(|e| format!("Preview icon: {e}"))?;
            if metadata.file_type().is_symlink() {
                return Err("Preview icon paths cannot contain symlinks".into());
            }
        }
        let canonical = file.canonicalize().map_err(|e| e.to_string())?;
        if !canonical.starts_with(root) {
            return Err("Preview icon escaped its fixture directory".into());
        }
        let metadata = fs::metadata(&canonical).map_err(|e| e.to_string())?;
        if !metadata.is_file() || metadata.len() == 0 || metadata.len() > MAX_ICON {
            return Err("Preview icons must be regular files up to 4 MiB".into());
        }
        let mut header = [0u8; 12];
        let n = fs::File::open(&canonical)
            .map_err(|e| e.to_string())?
            .read(&mut header)
            .map_err(|e| e.to_string())?;
        let extension = canonical
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let png = extension == "png" && n >= 8 && header[..8] == *b"\x89PNG\r\n\x1a\n";
        let jpeg = matches!(extension.as_str(), "jpg" | "jpeg")
            && n >= 3
            && header[..3] == [0xff, 0xd8, 0xff];
        if !png && !jpeg {
            return Err("Preview icon must be a PNG or JPEG with matching file contents".into());
        }
        icons.insert(
            id.clone(),
            canonical
                .to_str()
                .ok_or("Preview icon path is not UTF-8")?
                .to_owned(),
        );
    }
    let favorites = value["favorites"]
        .as_array()
        .filter(|a| a.len() <= 260)
        .ok_or("Preview favorites must be a bounded ID array")?;
    let mut seen = HashSet::new();
    let mut selected = Vec::new();
    for id in favorites {
        let id = id
            .as_str()
            .filter(|s| valid_app_id(s) && ids.contains(*s))
            .ok_or("Preview favorite refers to an invalid or unknown app")?;
        if seen.insert(id.to_owned()) {
            selected.push(id.to_owned());
        }
    }
    Ok(PreviewFixture {
        apps: clean,
        icons,
        favorites: selected,
    })
}
/// Render only local fixture data into an owned surface. No service worker,
/// native video, Home socket, window or application-manager operation is used.
pub unsafe fn preview(output: &str, screen_name: &str) -> Result<(), String> {
    let arguments: Vec<_> = std::env::args().collect();
    let option = |name: &str| {
        arguments
            .windows(2)
            .find(|v| v[0] == name)
            .map(|v| v[1].as_str())
    };
    let fixture_path = option("--fixture").ok_or("Offline preview requires --fixture JSON")?;
    let fixture = load_preview_fixture(std::path::Path::new(fixture_path))?;
    let output_path = std::path::Path::new(output);
    let parent = output_path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .ok_or("Preview output needs an explicit directory")?;
    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let base = crate::platform::install_directory()?;
    let scenes = catalogue::load(&base.join("catalogue.json"));
    let mut u = make_ui(Worker::idle(), scenes, None);
    u.merge_apps(&fixture.apps);
    u.icons = fixture.icons;
    u.prefs.favorites = fixture.favorites;
    u.prefs.sanitize();
    u.video_due = u64::MAX;
    u.prefs.appearance = match option("--appearance").unwrap_or("dark") {
        "dark" => Appearance::Dark,
        "light" => Appearance::Light,
        "auto" => Appearance::Auto,
        _ => return Err("Unknown preview appearance".into()),
    };
    u.prefs.clock_style = match option("--clock").unwrap_or("classic") {
        "classic" => ClockStyle::Classic,
        "minimal" => ClockStyle::Minimal,
        "glass" => ClockStyle::Glass,
        "immersive" => ClockStyle::Immersive,
        _ => return Err("Unknown preview clock style".into()),
    };
    u.prefs.icon_style = match option("--icons").unwrap_or("classic") {
        "classic" => IconStyle::Classic,
        "minimal" => IconStyle::Minimal,
        "glass" => IconStyle::Glass,
        "mono" => IconStyle::Monochrome,
        _ => return Err("Unknown preview icon style".into()),
    };
    let screen = match screen_name {
        "home" => Screen::Home,
        "settings" => Screen::Settings,
        "controls" => Screen::Controls,
        "apps" => Screen::Apps,
        "clock" => Screen::Clock,
        "aerials" => Screen::Aerials,
        "icons" => Screen::IconSettings,
        _ => return Err("Unknown preview screen".into()),
    };
    u.open(screen);
    u.toast_until = 0;
    u.idle.foreground(0);
    let mut g = Graphics::new_offscreen()?;
    g.verify_offscreen()?;
    if let Some(background) = option("--background") {
        g.preview_background(background)?;
    }
    draw(&mut g, &u, 0);
    g.verify_offscreen()?;
    g.screenshot(output);
    g.verify_offscreen()?;
    let metadata =
        std::fs::metadata(output_path).map_err(|e| format!("Preview output unavailable: {e}"))?;
    if !metadata.is_file() || metadata.len() != 6_220_817 {
        return Err("Preview output is incomplete".into());
    }
    println!(
        "{}",
        json!({"event":"offscreen-preview","offline":true,"file":output,"apps":u.apps.len(),"icons":u.icons.len(),"screen":screen_name})
    );
    Ok(())
}

pub unsafe fn run() -> Result<(), String> {
    let home_signal = crate::home::HomeSignal::bind().map_err(|e| e.to_string())?;
    let start = Instant::now();
    let mut g = Graphics::new()?;
    let worker = Worker::start();
    let (video, video_error) = match Video::new(ID) {
        Ok(v) => (Some(v), None),
        Err(e) => (None, Some(e)),
    };
    if let (Some(v), Some(id)) = (&video, g.video_window()) {
        let _ = v.set_window_id(id);
    }
    let base = crate::platform::install_directory()?;
    let scenes = catalogue::load(&base.join("catalogue.json"));
    let mut u = make_ui(worker, scenes, video);
    if let Some(e) = video_error {
        u.tell(format!("Aerial video unavailable: {e}"), 0);
        u.toast_until = 15000;
    }
    for r in [
        Request::LoadPreferences,
        Request::Inventory,
        Request::Inputs,
        Request::Capabilities,
        Request::Favorites,
    ] {
        u.send(r);
    }
    let bench = std::env::args().any(|a| a == "--benchmark");
    let mut running = true;
    let mut first = true;
    let mut first_apps = true;
    let mut first_artwork = true;
    let mut tick = 0;
    let mut samples = Vec::new();
    let mut pending_input = None;
    let mut presented_time = None;
    while running {
        let mut now = start.elapsed().as_millis() as u64;
        if home_signal.take() {
            u.editing = None;
            u.oled_preview = None;
            u.held_wake = None;
            u.open(Screen::Home);
            u.idle.foreground(now);
            u.refresh_inventory(now);
            println!("{{\"event\":\"home-relaunch\",\"ms\":{now}}}");
        }
        u.replies(now);
        u.video_events(now);
        if u.visible {
            u.request_artwork();
        }
        if now / 1000 != tick {
            tick = now / 1000;
            if u.screen != Screen::Clock && u.idle.saver_due(now, &u.prefs) {
                if u.screen == Screen::Home {
                    u.home_return = u.home.clone();
                }
                u.screen = Screen::Clock;
                u.dirty = true;
            }
        }
        let mut e = Event { bytes: [0; 56] };
        let mut ready = SDL_WaitEventTimeout(&mut e, event_wait_ms(u.dirty, u.visible));
        now = start.elapsed().as_millis() as u64;
        while ready != 0 {
            match e.kind {
                0x100 => {
                    running = false;
                    u.stop_video();
                    u.visible = false;
                }
                0x103 | 0x104 => {
                    u.visible = false;
                    u.stop_video();
                }
                0x105 | 0x106 => {
                    u.visible = true;
                    u.idle.foreground(now);
                    u.refresh_inventory(now);
                    u.video_due = now + u.prefs.aerial_delay * 1000;
                    if let Some(v) = &u.video {
                        let _ = v.set_visible(true);
                    }
                    u.dirty = true;
                }
                0x200 => {
                    let b = e.bytes;
                    match b[12] {
                        12 | 1 | 3 | 9 => {
                            u.visible = true;
                            u.idle.foreground(now);
                            u.refresh_inventory(now);
                            u.video_due = now + u.prefs.aerial_delay * 1000;
                            if let Some(v) = &u.video {
                                let _ = v.set_visible(true);
                            }
                        }
                        13 | 2 | 7 => {
                            u.visible = false;
                            u.stop_video();
                        }
                        14 => running = false,
                        _ => {}
                    }
                    u.dirty = true;
                }
                0x300 => {
                    let b = e.bytes;
                    let key = i32::from_ne_bytes(b[24..28].try_into().unwrap());
                    pending_input = Some(u32::from_ne_bytes(b[4..8].try_into().unwrap()));
                    let preview_wake = u.oled_preview.take().is_some();
                    let consumed = preview_wake
                        || u.held_wake == Some(key)
                        || u.idle.key_down(key, b[17] != 0, now, &u.prefs, u.visible);
                    if consumed {
                        u.held_wake = Some(key);
                        u.idle.foreground(now);
                    }
                    if !consumed {
                        handle_key(&mut u, key, now, &mut running);
                    } else if u.screen == Screen::Clock {
                        u.open(Screen::Home);
                    }
                    u.dirty = true;
                }
                0x301 => {
                    let b = e.bytes;
                    let key = i32::from_ne_bytes(b[24..28].try_into().unwrap());
                    u.idle.key_up(key);
                    if u.held_wake == Some(key) {
                        u.held_wake = None;
                    }
                }
                0x303 => {
                    let b = e.bytes;
                    let end = b[16..48].iter().position(|v| *v == 0).unwrap_or(32);
                    if let Ok(s) = std::str::from_utf8(&b[16..16 + end]) {
                        if u.held_wake.is_some() {
                            ready = SDL_PollEvent(&mut e);
                            continue;
                        }
                        if let Some(edit) = &mut u.editing {
                            if edit.len() < 160 {
                                edit.push_str(s);
                            }
                        } else if u.screen == Screen::Search {
                            u.query.push_str(s);
                            u.home = HomeState::new();
                            u.search_offset = 0;
                        }
                        u.dirty = true;
                    }
                }
                0x400 | 0x401 => {
                    let b = e.bytes;
                    let wake = u.oled_preview.take().is_some()
                        || u.idle.activity(now, &u.prefs, u.visible);
                    if wake {
                        u.pointer_wake_until = now + 400;
                    }
                    if wake && u.screen == Screen::Clock {
                        u.open(Screen::Home);
                    }
                    let hit = pointer(
                        &mut u,
                        i32::from_ne_bytes(b[20..24].try_into().unwrap()),
                        i32::from_ne_bytes(b[24..28].try_into().unwrap()),
                        now,
                    );
                    if e.kind == 0x401 {
                        let suppress = wake || now < u.pointer_wake_until;
                        u.pointer_wake_until = 0;
                        if !suppress && hit {
                            handle_key(&mut u, 13, now, &mut running);
                        }
                    }
                    u.dirty = true;
                }
                _ => {}
            }
            ready = SDL_PollEvent(&mut e);
        }
        if bench && now < 20000 {
            handle_key(
                &mut u,
                if (now / 1000) % 2 == 0 {
                    1073741903
                } else {
                    1073741904
                },
                now,
                &mut running,
            );
            pending_input = Some(SDL_GetTicks());
            u.dirty = true;
        } else if bench {
            running = false;
        }
        let timed = TimedVisuals::new(
            crate::clock::parts(),
            u.oled_state(now),
            now < u.toast_until,
        );
        if presented_time != Some(timed) {
            u.dirty = true;
        }
        if u.dirty && u.visible {
            draw(&mut g, &u, now);
            u.dirty = false;
            presented_time = Some(timed);
            if first {
                println!(
                    "{{\"event\":\"first-present\",\"ms\":{:.3},\"renderer\":\"{}\"}}",
                    start.elapsed().as_secs_f64() * 1000.0,
                    g.backend
                );
                first = false;
            }
            if first_apps && !u.apps.is_empty() {
                println!(
                    "{{\"event\":\"first-app-grid\",\"ms\":{:.3},\"apps\":{}}}",
                    start.elapsed().as_secs_f64() * 1000.0,
                    u.apps.len()
                );
                first_apps = false;
            }
            if first_artwork && !u.icons.is_empty() {
                println!(
                    "{{\"event\":\"first-artwork-frame\",\"ms\":{:.3},\"icons\":{}}}",
                    start.elapsed().as_secs_f64() * 1000.0,
                    u.icons.len()
                );
                first_artwork = false;
            }
            if let Some(at) = pending_input.take() {
                samples.push(SDL_GetTicks().wrapping_sub(at) as f64);
            }
        }
    }
    u.stop_video();
    u.send(Request::Shutdown);
    if !samples.is_empty() {
        println!(
            "{{\"event\":\"summary\",\"samples\":{},\"median_ms\":{},\"p95_ms\":{}}}",
            samples.len(),
            aerial_native_probe::percentile(&samples, 0.5).unwrap(),
            aerial_native_probe::percentile(&samples, 0.95).unwrap()
        );
    }
    Ok(())
}
fn pointer(u: &mut Ui, x: i32, y: i32, now: u64) -> bool {
    let oled = u.idle.state(now, &u.prefs, u.visible);
    let (x, y) = (x - oled.x, y - oled.y);
    if u.editing.is_some() {
        if (230..1710).contains(&x) && (445..795).contains(&y) {
            let col = ((x - 230) / 148) as usize;
            let row = ((y - 445) / 70) as usize;
            if (x - 230) % 148 < 130 && (y - 445) % 70 < 55 {
                u.text_cursor = row * 10 + col;
                return true;
            }
        }
        return false;
    }
    if u.screen == Screen::Clock {
        return false;
    }
    if panel_open(&u.screen) {
        let wide = wide_panel(&u.screen);
        let left = if wide { 310 } else { 1268 };
        let width = if wide { 1300 } else { 590 };
        if (UiRect {
            x: left + width - 84,
            y: 84,
            w: 48,
            h: 48,
        })
        .contains(x, y)
        {
            u.toolbar = Some(3);
            return true;
        }
        if !(UiRect {
            x: left + 36,
            y: 158,
            w: width - 72,
            h: 836,
        })
        .contains(x, y)
        {
            return false;
        }
        if matches!(u.screen, Screen::Apps | Screen::Folder(_)) {
            let apps = u.app_list();
            let scroll = grid_scroll(u);
            for i in 0..apps.len() {
                if tile_rect(4, i % 4, i / 4, true, scroll).contains(x, y) {
                    u.home.focus = Focus::App(i);
                    u.toolbar = None;
                    return true;
                }
            }
        } else if u.screen == Screen::Search {
            if (UiRect {
                x: left + 36,
                y: 158,
                w: width - 72,
                h: 70,
            })
            .contains(x, y)
            {
                u.toolbar = Some(4);
                return true;
            }
            if y < 244 {
                return false;
            }
            let skip = (u.search_offset.max(0) / 86) as usize * 2;
            for i in skip..(skip + 20).min(u.app_list().len()) {
                if (UiRect {
                    x: left + 36 + (i % 2) as i32 * 619,
                    y: 244 + (i / 2) as i32 * 86 - u.search_offset,
                    w: 609,
                    h: 76,
                })
                .contains(x, y)
                {
                    u.home.focus = Focus::App(i);
                    u.toolbar = None;
                    return true;
                }
            }
        } else {
            let (positions, scroll, height) = menu_positions(u);
            if y > 48 + height - 36 {
                return false;
            }
            for (i, top) in positions.into_iter().enumerate() {
                if (UiRect {
                    x: left + 36,
                    y: top - scroll,
                    w: width - 72,
                    h: 76,
                })
                .contains(x, y)
                {
                    u.menu = i;
                    u.toolbar = None;
                    return true;
                }
            }
        }
        return false;
    }
    let start = u.toolbar_left.get();
    for i in 0..3 {
        if (UiRect {
            x: start + i * 80,
            y: 54,
            w: 64,
            h: 54,
        })
        .contains(x, y)
        {
            u.toolbar = Some(i as usize);
            return true;
        }
    }
    let apps = u.app_list();
    let scroll = home_scroll(u, apps.len());
    if u.home.input_row_open {
        for i in 0..u.input_list().len() {
            if (UiRect {
                x: 102 + i as i32 * 252,
                y: input_top(u, apps.len()) - scroll,
                w: 228,
                h: 88,
            })
            .contains(x, y)
            {
                u.home.focus = Focus::Input(i);
                u.toolbar = None;
                return true;
            }
        }
    }
    if y < 940 {
        for i in 0..apps.len() {
            if tile_rect(
                u.prefs.columns,
                i % u.prefs.columns,
                i / u.prefs.columns,
                false,
                scroll,
            )
            .contains(x, y)
            {
                u.home.focus = Focus::App(i);
                u.toolbar = None;
                return true;
            }
        }
    }
    false
}

fn keyboard_label(i: usize, lower: bool) -> String {
    match i {
        0..=25 => ((if lower { b'a' } else { b'A' } + i as u8) as char).to_string(),
        26..=35 => (i - 26).to_string(),
        36 => "Space".into(),
        37 => "Delete".into(),
        38 => "Done".into(),
        39 => "Cancel".into(),
        40 => if lower { "ABC" } else { "abc" }.into(),
        41 => ".".into(),
        42 => ":".into(),
        43 => "/".into(),
        44 => "-".into(),
        45 => "_".into(),
        46 => "@".into(),
        47 => "!".into(),
        48 => "?".into(),
        _ => "'".into(),
    }
}
fn move_order(order: &mut Vec<String>, visible: &[String], id: &str, delta: i32) -> bool {
    let mut ids = visible.to_vec();
    let Some(at) = ids.iter().position(|v| v == id) else {
        return false;
    };
    let to = (at as i64 + delta as i64).clamp(0, ids.len().saturating_sub(1) as i64) as usize;
    if to == at {
        return false;
    }
    ids.swap(at, to);
    for old in order.iter() {
        if !ids.contains(old) {
            ids.push(old.clone());
        }
    }
    *order = ids;
    true
}
fn handle_key(u: &mut Ui, key: i32, now: u64, running: &mut bool) {
    if u.editing.is_some() {
        if [27, 461, 1073742094, (1 << 30) | 482].contains(&key) {
            u.editing = None;
            return;
        }
        if key == 8 {
            u.editing.as_mut().unwrap().pop();
            return;
        }
        if key == 13 {
            match u.text_cursor {
                0..=35 => {
                    let c = keyboard_label(u.text_cursor, u.keyboard_lower);
                    let text = u.editing.as_mut().unwrap();
                    if text.chars().count() < 80 {
                        text.push_str(&c);
                    }
                }
                36 => u.editing.as_mut().unwrap().push(' '),
                37 => {
                    u.editing.as_mut().unwrap().pop();
                }
                38 => u.act(Do::SaveText, now),
                39 => u.editing = None,
                40 => u.keyboard_lower = !u.keyboard_lower,
                _ => {
                    let c = keyboard_label(u.text_cursor, u.keyboard_lower);
                    let text = u.editing.as_mut().unwrap();
                    if text.chars().count() < 128 {
                        text.push_str(&c);
                    }
                }
            }
            return;
        }
        let direction = match key {
            1073741904 => Some(Direction::Left),
            1073741903 => Some(Direction::Right),
            1073741906 => Some(Direction::Up),
            1073741905 => Some(Direction::Down),
            _ => None,
        };
        if let Some(d) = direction {
            u.text_cursor = aerial_native_probe::navigate(u.text_cursor, 50, 10, d);
        }
        return;
    }
    if u.screen == Screen::Clock {
        u.open(Screen::Home);
        return;
    }
    if [27, 461, 1073742094, (1 << 30) | 482].contains(&key) {
        if u.editing.is_some() {
            u.editing = None;
        } else if !u.home.back() {
            if u.screen == Screen::Home {
                u.open(Screen::Controls);
            } else {
                u.open(Screen::Home);
            }
        }
        return;
    }
    if key == 8 {
        if u.screen == Screen::Search {
            u.query.pop();
            u.home = HomeState::new();
            u.search_offset = 0;
        } else if let Some(s) = &mut u.editing {
            s.pop();
        }
        return;
    }
    if key == 13 && u.editing.is_some() {
        u.act(Do::SaveText, now);
        return;
    }
    let d = match key {
        1073741904 => Some(Direction::Left),
        1073741903 => Some(Direction::Right),
        1073741906 => Some(Direction::Up),
        1073741905 => Some(Direction::Down),
        _ => None,
    };
    if let Some(d) = d {
        if let Some(i) = u.toolbar {
            if panel_open(&u.screen) {
                match d {
                    Direction::Down => {
                        u.toolbar = None;
                        u.menu = 0;
                        u.home.focus = Focus::App(0);
                    }
                    Direction::Up if i == 4 => u.toolbar = Some(3),
                    _ => {}
                }
            } else {
                match d {
                    Direction::Left => u.toolbar = Some(i.saturating_sub(1)),
                    Direction::Right => u.toolbar = Some((i + 1).min(2)),
                    Direction::Down => u.toolbar = None,
                    _ => {}
                }
            }
        } else if u.screen == Screen::Home {
            if u.home
                .move_focus(d, u.app_list().len(), u.input_list().len(), u.prefs.columns)
                == NavEffect::Toolbar
            {
                u.toolbar = Some(2);
            }
        } else if matches!(u.screen, Screen::Apps | Screen::Search | Screen::Folder(_)) {
            let count = u.app_list().len();
            let index = if let Focus::App(i) = u.home.focus {
                i
            } else {
                0
            };
            let columns = grid_columns(u);
            if matches!(d, Direction::Up) && index < columns {
                u.toolbar = Some(if u.screen == Screen::Search { 4 } else { 3 });
            } else if count > 0 {
                u.home.focus = Focus::App(aerial_native_probe::navigate(index, count, columns, d));
            }
        } else {
            let n = u.rows().len();
            match d {
                Direction::Up | Direction::Left => {
                    if u.menu == 0 {
                        u.toolbar = Some(3);
                    } else {
                        u.menu -= 1;
                    }
                }
                Direction::Down | Direction::Right => {
                    u.menu = (u.menu + 1).min(n.saturating_sub(1))
                }
            }
        }
        if u.toolbar.is_none() {
            u.reveal_focus();
        }
        return;
    }
    if key == 13 {
        if let Some(i) = u.toolbar {
            if i == 3 {
                u.open(Screen::Home);
            } else if i == 4 && u.screen == Screen::Search {
                u.editing = Some(u.query.clone());
                u.edit_search = true;
                u.edit_helper = false;
                u.edit_folder = None;
                u.text_cursor = 0;
            } else if i < 3 {
                u.open([Screen::Apps, Screen::Search, Screen::Controls][i].clone());
            }
        } else if matches!(
            u.screen,
            Screen::Home | Screen::Apps | Screen::Search | Screen::Folder(_)
        ) {
            match u.home.activate(&u.app_list(), &u.input_list()) {
                Action::Launch(id) => u.act(Do::Launch(id), now),
                Action::OpenFolder(id) => u.open(Screen::Folder(id)),
                _ => {}
            }
        } else if let Some(r) = u.rows().get(u.menu) {
            let a = r.action.clone();
            if matches!(a, Do::Exit) {
                *running = false;
            } else {
                u.act(a, now);
            }
        }
    }
    if [1073741942, 457, (1 << 30) | 483].contains(&key)
        && matches!(u.screen, Screen::Home | Screen::Apps | Screen::Folder(_))
    {
        if let Focus::App(i) = u.home.focus {
            if let Some(a) = u.app_list().get(i) {
                u.open(if a.folder {
                    Screen::FolderManage(a.id.clone())
                } else {
                    Screen::Manage(a.id.clone())
                });
            }
        }
    }
    if u.toolbar.is_none() {
        u.reveal_focus();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::Response;
    use std::sync::mpsc;

    /// A completed local response, with no service thread, window, video or
    /// filesystem activity. Start as an already-painted panel to exercise
    /// invalidation without relying on periodic redraws or later replies.
    fn with_reply(request: Request, value: Value) -> Ui {
        let (sender, requests) = mpsc::sync_channel(1);
        drop(requests);
        let (results, receiver) = mpsc::channel();
        results
            .send(Response {
                request,
                result: Ok(value),
            })
            .unwrap();
        let mut ui = make_ui(Worker { sender, receiver }, vec![], None);
        ui.screen = Screen::Settings;
        ui.dirty = false;
        *ui.row_cache.borrow_mut() = Some(Rc::new(vec![]));
        ui
    }

    #[test]
    fn loaded_preferences_repaint_without_inventory_or_clock_activity() {
        for cached in [false, true] {
            let mut value = json!({"columns":5,"appearance":"light","clockStyle":"glass"});
            if cached {
                value["cachedApps"] = json!([{"id":"youtube.leanback.v4","title":"YouTube"}]);
            }
            let mut ui = with_reply(Request::LoadPreferences, value);
            ui.replies(10);
            assert!(ui.dirty);
            assert!(ui.row_cache.borrow().is_none());
            assert_eq!(ui.prefs.columns, 5);
            assert_eq!(ui.prefs.clock_style, ClockStyle::Glass);
            assert_eq!(ui.apps.len(), usize::from(cached));
        }
    }

    #[test]
    fn capability_reply_repaints_an_already_open_panel() {
        let mut ui = with_reply(
            Request::Capabilities,
            json!({"dolbyVision":true,"hdr10":true}),
        );
        ui.replies(10);
        assert!(ui.dirty);
        assert!(ui.row_cache.borrow().is_none());
        assert!(ui.caps.dolby && ui.caps.hdr);
    }

    #[test]
    fn artwork_colour_repaints_even_without_an_image_path() {
        let mut ui = with_reply(
            Request::Artwork {
                id: "youtube.leanback.v4".into(),
                path: String::new(),
            },
            json!({"color":"#0578ff"}),
        );
        ui.replies(10);
        assert!(ui.dirty);
        assert_eq!(
            ui.icon_colors.get("youtube.leanback.v4"),
            Some(&[5, 120, 255, 255])
        );
        assert!(ui.icons.is_empty());
    }

    #[test]
    fn imported_favourites_invalidate_existing_panel_rows() {
        let mut ui = with_reply(
            Request::Favorites,
            json!({"favorites":["youtube.leanback.v4"]}),
        );
        ui.replies(10);
        assert!(ui.dirty);
        assert!(ui.row_cache.borrow().is_none());
        assert_eq!(ui.prefs.favorites, vec!["youtube.leanback.v4"]);
    }
}
