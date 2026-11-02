//! UI-independent launcher behavior. TV service calls and rendering stay outside
//! this module: moving focus never launches an app or selects an HDMI input.
use crate::Direction;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct App {
    pub id: String,
    pub title: String,
    #[serde(default, deserialize_with = "nullable_string")]
    pub icon: String,
    pub input: bool,
    pub utility: bool,
    pub folder: bool,
}
fn nullable_string<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    Option::<String>::deserialize(deserializer).map(Option::unwrap_or_default)
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Folder {
    pub id: String,
    pub title: String,
    pub apps: Vec<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Appearance {
    #[default]
    Dark,
    Light,
    Auto,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ClockStyle {
    #[default]
    Classic,
    Minimal,
    Glass,
    Immersive,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClockFormat {
    #[default]
    #[serde(rename = "auto")]
    Auto,
    #[serde(rename = "12")]
    Twelve,
    #[serde(rename = "24")]
    TwentyFour,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IconStyle {
    #[default]
    Classic,
    Glass,
    Minimal,
    #[serde(rename = "mono")]
    Monochrome,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IconLabels {
    #[default]
    Focused,
    Always,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum HomeMode {
    #[default]
    #[serde(rename = "most-used")]
    MostUsed,
    #[serde(rename = "favorites")]
    Favorites,
    #[serde(rename = "all")]
    All,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Quality {
    #[default]
    Auto,
    Dolby,
    Hdr,
    Sdr,
}

impl ClockStyle {
    pub fn next(self) -> Self {
        match self {
            Self::Classic => Self::Minimal,
            Self::Minimal => Self::Glass,
            Self::Glass => Self::Immersive,
            Self::Immersive => Self::Classic,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Classic => "Classic",
            Self::Minimal => "Minimal",
            Self::Glass => "Liquid Glass",
            Self::Immersive => "Immersive",
        }
    }
}
impl IconStyle {
    pub fn next(self) -> Self {
        match self {
            Self::Classic => Self::Glass,
            Self::Glass => Self::Minimal,
            Self::Minimal => Self::Monochrome,
            Self::Monochrome => Self::Classic,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Classic => "Classic",
            Self::Glass => "Liquid Glass",
            Self::Minimal => "Minimal",
            Self::Monochrome => "Monochrome",
        }
    }
}
impl ClockFormat {
    pub fn next(self) -> Self {
        match self {
            Self::Auto => Self::Twelve,
            Self::Twelve => Self::TwentyFour,
            Self::TwentyFour => Self::Auto,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Auto => "System",
            Self::Twelve => "12-hour",
            Self::TwentyFour => "24-hour",
        }
    }
}
impl HomeMode {
    pub fn next(self) -> Self {
        match self {
            Self::MostUsed => Self::Favorites,
            Self::Favorites => Self::All,
            Self::All => Self::MostUsed,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::MostUsed => "Most used",
            Self::Favorites => "Favorites",
            Self::All => "All apps",
        }
    }
}
impl Quality {
    pub fn next(self) -> Self {
        match self {
            Self::Auto => Self::Dolby,
            Self::Dolby => Self::Hdr,
            Self::Hdr => Self::Sdr,
            Self::Sdr => Self::Auto,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Auto => "Maximum compatible",
            Self::Dolby => "Dolby Vision",
            Self::Hdr => "HDR",
            Self::Sdr => "SDR",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Usage {
    pub count: u32,
    pub last: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Preferences {
    pub appearance: Appearance,
    pub columns: usize,
    pub aerial: bool,
    pub aerial_delay: u64,
    pub scene_id: String,
    pub shuffle: bool,
    pub legacy_hd: bool,
    pub quality: Quality,
    pub categories: Vec<String>,
    pub media_server: String,
    pub home_mode: HomeMode,
    pub favorites: Vec<String>,
    pub order: Vec<String>,
    pub hidden: Vec<String>,
    pub folders: Vec<Folder>,
    pub usage: HashMap<String, Usage>,
    pub saver_minutes: u64,
    pub oled_dimming: bool,
    pub oled_protection: bool,
    pub reduce_motion: bool,
    pub clock_style: ClockStyle,
    pub clock_format: ClockFormat,
    pub clock_date: bool,
    pub icon_style: IconStyle,
    pub icon_labels: IconLabels,
    pub profile: String,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            appearance: Appearance::Dark,
            columns: 6,
            aerial: true,
            aerial_delay: 3,
            scene_id: String::new(),
            shuffle: true,
            legacy_hd: false,
            quality: Quality::Auto,
            categories: ["Cities", "Landscapes", "Space", "Underwater"]
                .map(str::to_owned)
                .to_vec(),
            media_server: String::new(),
            home_mode: HomeMode::MostUsed,
            favorites: vec![],
            order: vec![],
            hidden: vec![],
            folders: vec![],
            usage: HashMap::new(),
            saver_minutes: 3,
            oled_dimming: true,
            oled_protection: true,
            reduce_motion: false,
            clock_style: ClockStyle::Classic,
            clock_format: ClockFormat::Auto,
            clock_date: true,
            icon_style: IconStyle::Classic,
            icon_labels: IconLabels::Focused,
            profile: "Home".into(),
        }
    }
}

pub fn valid_app_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
}
pub fn valid_input_id(id: &str) -> bool {
    ["com.webos.app.hdmi", "com.palm.app.hdmi"].iter().any(|p| {
        id.strip_prefix(p)
            .is_some_and(|n| ["1", "2", "3", "4"].contains(&n))
    })
}
pub fn valid_media_server(value: &str) -> bool {
    if value.len() > 128 {
        return false;
    }
    let Some(authority) = value.strip_prefix("http://") else {
        return false;
    };
    let Some((host, port)) = authority.split_once(':') else {
        return false;
    };
    if port.is_empty()
        || !port.bytes().all(|b| b.is_ascii_digit())
        || port.parse::<u16>().ok().filter(|p| *p > 0).is_none()
    {
        return false;
    }
    if host == "localhost" {
        return true;
    }
    host.parse::<std::net::Ipv4Addr>()
        .is_ok_and(|ip| ip.is_private() || ip.is_loopback())
}
fn clean_ids(ids: &mut Vec<String>, limit: usize) {
    let mut seen = HashSet::new();
    ids.retain(|id| valid_app_id(id) && seen.insert(id.clone()));
    ids.truncate(limit);
}
fn clean_title(title: &str, limit: usize) -> String {
    title
        .chars()
        .filter(|c| !c.is_control())
        .take(limit)
        .collect()
}
impl Preferences {
    pub fn sanitize(&mut self) {
        if self.columns != 5 {
            self.columns = 6;
        }
        if ![0, 3, 5, 10].contains(&self.aerial_delay) {
            self.aerial_delay = 3;
        }
        if ![0, 1, 3, 5, 10].contains(&self.saver_minutes) {
            self.saver_minutes = 3;
        }
        if self.scene_id.len() > 100 || (!self.scene_id.is_empty() && !valid_app_id(&self.scene_id))
        {
            self.scene_id.clear();
        }
        if !self.media_server.is_empty() && !valid_media_server(&self.media_server) {
            self.media_server.clear();
        }
        self.categories
            .retain(|c| ["Cities", "Landscapes", "Space", "Underwater"].contains(&c.as_str()));
        let mut categories = HashSet::new();
        self.categories.retain(|c| categories.insert(c.clone()));
        clean_ids(&mut self.favorites, 100);
        clean_ids(&mut self.hidden, 500);
        clean_ids(&mut self.order, 500);
        let mut folder_ids = HashSet::new();
        self.folders.retain(|f| {
            f.id.starts_with("folder-")
                && f.id.len() > 7
                && f.id.len() <= 100
                && f.id
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
                && folder_ids.insert(f.id.clone())
        });
        self.folders.truncate(20);
        for f in &mut self.folders {
            f.title = clean_title(&f.title, 32);
            clean_ids(&mut f.apps, 100);
            f.apps.retain(|id| !id.starts_with("folder-"));
        }
        self.usage.retain(|id, u| valid_app_id(id) && u.count > 0);
        for u in self.usage.values_mut() {
            u.count = u.count.min(1_000_000);
        }
        if self.usage.len() > 500 {
            let mut keys: Vec<_> = self.usage.keys().cloned().collect();
            keys.sort_by(|a, b| self.usage[b].last.cmp(&self.usage[a].last).then(a.cmp(b)));
            for key in keys.into_iter().skip(500) {
                self.usage.remove(&key);
            }
        }
        self.profile = clean_title(&self.profile, 32);
        if self.profile.is_empty() {
            self.profile = "Home".into();
        }
    }
    /// Call only after the TV acknowledges an explicit launch request.
    pub fn record_launch(&mut self, id: &str, now_ms: u64) {
        if !valid_app_id(id) {
            return;
        }
        let usage = self.usage.entry(id.into()).or_default();
        usage.count = usage.count.saturating_add(1).min(1_000_000);
        usage.last = now_ms;
        self.sanitize();
    }
}

pub fn starter_favorites(apps: &[App]) -> Vec<String> {
    let brands = ["netflix", "youtube", "apple", "disney", "prime", "live"];
    let mut ids = Vec::new();
    for brand in brands {
        if let Some(app) = apps.iter().find(|a| {
            if a.input || a.utility || a.folder {
                return false;
            }
            let value = (a.id.clone() + " " + &a.title).to_lowercase();
            match brand {
                "apple" => value.contains("apple tv") || value.contains("appletv"),
                "prime" => {
                    value.contains("prime video")
                        || value.contains("primevideo")
                        || a.id == "amazon"
                }
                "live" => value.contains("live tv") || value.contains("livetv"),
                _ => value.contains(brand),
            }
        }) {
            if !ids.contains(&app.id) {
                ids.push(app.id.clone());
            }
        }
    }
    ids
}
pub fn visible_apps(
    apps: &[App],
    prefs: &Preferences,
    query: &str,
    folder: Option<&str>,
) -> Vec<App> {
    let query = query.to_lowercase();
    let grouped: HashSet<_> = prefs.folders.iter().flat_map(|f| f.apps.iter()).collect();
    let folder_ids = folder
        .and_then(|id| prefs.folders.iter().find(|f| f.id == id))
        .map(|f| &f.apps);
    let mut seen = HashSet::new();
    let mut list: Vec<_> = apps
        .iter()
        .filter(|a| valid_app_id(&a.id) && seen.insert(a.id.clone()))
        .filter(|a| {
            if folder.is_some() {
                return folder_ids.is_some_and(|ids| ids.contains(&a.id));
            }
            !prefs.hidden.contains(&a.id) && (!query.is_empty() || !grouped.contains(&a.id))
        })
        .filter(|a| a.title.to_lowercase().contains(&query))
        .cloned()
        .collect();
    if folder.is_none() && query.is_empty() {
        for f in &prefs.folders {
            if !prefs.hidden.contains(&f.id)
                && f.apps.iter().any(|id| apps.iter().any(|a| &a.id == id))
            {
                list.push(App {
                    id: f.id.clone(),
                    title: f.title.clone(),
                    folder: true,
                    ..App::default()
                });
            }
        }
    }
    let rank: HashMap<_, _> = prefs
        .order
        .iter()
        .enumerate()
        .map(|(i, id)| (id, i))
        .collect();
    list.sort_by_key(|a| rank.get(&a.id).copied().unwrap_or(usize::MAX));
    list
}
pub fn home_apps(apps: &[App], prefs: &Preferences) -> Vec<App> {
    let list = visible_apps(apps, prefs, "", None);
    if prefs.home_mode == HomeMode::All {
        return list;
    }
    let seed = if prefs.favorites.is_empty() {
        starter_favorites(apps)
    } else {
        prefs.favorites.clone()
    };
    let rank: HashMap<_, _> = seed.iter().enumerate().map(|(i, id)| (id, i)).collect();
    let mut chosen: Vec<_> = list
        .iter()
        .filter(|a| !a.input && !a.utility && !a.folder && a.id != "settings")
        .filter(|a| {
            rank.contains_key(&a.id)
                || (prefs.home_mode == HomeMode::MostUsed && prefs.usage.contains_key(&a.id))
        })
        .cloned()
        .collect();
    chosen.sort_by(|a, b| {
        let seed_rank = || {
            rank.get(&a.id)
                .copied()
                .unwrap_or(usize::MAX)
                .cmp(&rank.get(&b.id).copied().unwrap_or(usize::MAX))
        };
        if prefs.home_mode == HomeMode::Favorites {
            return seed_rank();
        }
        let ua = prefs.usage.get(&a.id).copied().unwrap_or_default();
        let ub = prefs.usage.get(&b.id).copied().unwrap_or_default();
        ub.count
            .cmp(&ua.count)
            .then(ub.last.cmp(&ua.last))
            .then_with(seed_rank)
    });
    if prefs.home_mode == HomeMode::MostUsed {
        chosen.truncate(6);
    }
    let manual: HashMap<_, _> = prefs
        .order
        .iter()
        .enumerate()
        .map(|(i, id)| (id, i))
        .collect();
    chosen.sort_by_key(|a| manual.get(&a.id).copied().unwrap_or(usize::MAX));
    chosen.extend(list.into_iter().filter(|a| a.folder));
    chosen
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    App(usize),
    Input(usize),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavEffect {
    None,
    Focused,
    InputsRevealed,
    InputsHidden,
    Toolbar,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    None,
    Launch(String),
    OpenFolder(String),
}
#[derive(Debug, Clone)]
pub struct HomeState {
    pub focus: Focus,
    pub input_row_open: bool,
    return_app: usize,
}
impl Default for HomeState {
    fn default() -> Self {
        Self {
            focus: Focus::App(0),
            input_row_open: false,
            return_app: 0,
        }
    }
}
impl HomeState {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn clamp(&mut self, app_count: usize, input_count: usize) {
        self.return_app = self.return_app.min(app_count.saturating_sub(1));
        self.focus = match self.focus {
            Focus::App(i) => Focus::App(i.min(app_count.saturating_sub(1))),
            Focus::Input(i) if input_count > 0 => Focus::Input(i.min(input_count - 1)),
            _ => {
                self.input_row_open = false;
                Focus::App(self.return_app)
            }
        };
    }
    pub fn move_focus(
        &mut self,
        direction: Direction,
        app_count: usize,
        input_count: usize,
        columns: usize,
    ) -> NavEffect {
        self.clamp(app_count, input_count);
        let columns = columns.max(1);
        match self.focus {
            Focus::Input(i) => match direction {
                Direction::Up => {
                    self.back();
                    NavEffect::InputsHidden
                }
                Direction::Left if i > 0 => {
                    self.focus = Focus::Input(i - 1);
                    NavEffect::Focused
                }
                Direction::Right if i + 1 < input_count => {
                    self.focus = Focus::Input(i + 1);
                    NavEffect::Focused
                }
                _ => NavEffect::None,
            },
            Focus::App(i) => {
                if app_count == 0 {
                    return NavEffect::Toolbar;
                }
                match direction {
                    Direction::Up if i < columns => NavEffect::Toolbar,
                    Direction::Down
                        if i / columns == (app_count - 1) / columns && input_count > 0 =>
                    {
                        self.return_app = i;
                        self.input_row_open = true;
                        self.focus = Focus::Input((i % columns).min(input_count - 1));
                        NavEffect::InputsRevealed
                    }
                    Direction::Down if i + columns < app_count => {
                        self.focus = Focus::App(i + columns);
                        NavEffect::Focused
                    }
                    Direction::Down if i / columns < (app_count - 1) / columns => {
                        self.focus = Focus::App(app_count - 1);
                        NavEffect::Focused
                    }
                    _ => {
                        let next = crate::navigate(i, app_count, columns, direction);
                        if next != i {
                            self.focus = Focus::App(next);
                            NavEffect::Focused
                        } else {
                            NavEffect::None
                        }
                    }
                }
            }
        }
    }
    pub fn activate(&self, apps: &[App], inputs: &[App]) -> Action {
        match self.focus {
            Focus::App(i) => match apps.get(i) {
                Some(a) if a.folder && valid_app_id(&a.id) => Action::OpenFolder(a.id.clone()),
                Some(a) if !a.input && valid_app_id(&a.id) => Action::Launch(a.id.clone()),
                _ => Action::None,
            },
            Focus::Input(i) => match inputs.get(i) {
                Some(a) if self.input_row_open && valid_input_id(&a.id) => {
                    Action::Launch(a.id.clone())
                }
                _ => Action::None,
            },
        }
    }
    pub fn back(&mut self) -> bool {
        if !self.input_row_open {
            return false;
        }
        self.input_row_open = false;
        self.focus = Focus::App(self.return_app);
        true
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rest {
    Active,
    Faded,
    Hidden,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OledState {
    pub x: i32,
    pub y: i32,
    pub rest: Rest,
    pub ui_alpha: u8,
    pub overlay_alpha: u8,
}
pub fn oled_state(prefs: &Preferences, idle_ms: u64, elapsed_ms: u64, visible: bool) -> OledState {
    if !visible {
        return OledState {
            x: 0,
            y: 0,
            rest: Rest::Active,
            ui_alpha: 255,
            overlay_alpha: 0,
        };
    }
    let rest = if !prefs.oled_protection {
        Rest::Active
    } else if idle_ms >= 300_000 {
        Rest::Hidden
    } else if idle_ms >= 120_000 {
        Rest::Faded
    } else {
        Rest::Active
    };
    let offsets = [
        (0, 0),
        (4, 0),
        (4, 4),
        (0, 4),
        (-4, 4),
        (-4, 0),
        (-4, -4),
        (0, -4),
        (4, -4),
    ];
    let (x, y) = if prefs.reduce_motion || !prefs.oled_protection {
        (0, 0)
    } else {
        offsets[((elapsed_ms / 60_000) % 9) as usize]
    };
    OledState {
        x,
        y,
        rest,
        ui_alpha: match rest {
            Rest::Active => 255,
            Rest::Faded => 115,
            Rest::Hidden => 0,
        },
        overlay_alpha: if prefs.oled_dimming && idle_ms >= 1_800_000 {
            247
        } else {
            0
        },
    }
}
#[derive(Debug, Clone)]
pub struct IdleTracker {
    pub last_activity_ms: u64,
    pub epoch_ms: u64,
    wake_key: Option<i32>,
}
impl IdleTracker {
    pub fn new(now_ms: u64) -> Self {
        Self {
            last_activity_ms: now_ms,
            epoch_ms: now_ms,
            wake_key: None,
        }
    }
    pub fn state(&self, now_ms: u64, prefs: &Preferences, visible: bool) -> OledState {
        oled_state(
            prefs,
            now_ms.saturating_sub(self.last_activity_ms),
            now_ms.saturating_sub(self.epoch_ms),
            visible,
        )
    }
    /// Returns true when this event only wakes the UI and must not activate it.
    pub fn activity(&mut self, now_ms: u64, prefs: &Preferences, visible: bool) -> bool {
        let old = self.state(now_ms, prefs, visible);
        self.last_activity_ms = now_ms;
        old.rest != Rest::Active || old.overlay_alpha > 0
    }
    pub fn key_down(
        &mut self,
        key: i32,
        repeat: bool,
        now_ms: u64,
        prefs: &Preferences,
        visible: bool,
    ) -> bool {
        if self.wake_key == Some(key) {
            self.last_activity_ms = now_ms;
            return true;
        }
        if self.activity(now_ms, prefs, visible) {
            self.wake_key = Some(key);
            return true;
        }
        // Native repeat messages cannot activate an app after a wake key even
        // if the platform supplies a differently translated repeat keycode.
        if repeat && self.wake_key.is_some() {
            return true;
        }
        false
    }
    pub fn key_up(&mut self, key: i32) {
        if self.wake_key == Some(key) {
            self.wake_key = None;
        }
    }
    pub fn foreground(&mut self, now_ms: u64) {
        self.last_activity_ms = now_ms;
        self.wake_key = None;
    }
    pub fn saver_due(&self, now_ms: u64, prefs: &Preferences) -> bool {
        prefs.saver_minutes > 0
            && now_ms.saturating_sub(self.last_activity_ms) >= prefs.saver_minutes * 60_000
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ClockParts {
    pub hour: u8,
    pub minute: u8,
    pub weekday: u8,
    pub month: u8,
    pub day: u8,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClockFace {
    pub time: String,
    pub period: String,
    pub date: String,
}
pub fn clock_face(parts: ClockParts, prefs: &Preferences, system_12h: bool) -> ClockFace {
    let hour = parts.hour % 24;
    let minute = parts.minute % 60;
    let twelve = match prefs.clock_format {
        ClockFormat::Auto => system_12h,
        ClockFormat::Twelve => true,
        ClockFormat::TwentyFour => false,
    };
    let time = if twelve {
        format!(
            "{}:{:02}",
            if hour % 12 == 0 { 12 } else { hour % 12 },
            minute
        )
    } else {
        format!("{:02}:{:02}", hour, minute)
    };
    let period = if twelve {
        if hour < 12 {
            "AM"
        } else {
            "PM"
        }
    } else {
        ""
    }
    .to_owned();
    let date = if prefs.clock_date {
        let days = [
            "Sunday",
            "Monday",
            "Tuesday",
            "Wednesday",
            "Thursday",
            "Friday",
            "Saturday",
        ];
        let months = [
            "January",
            "February",
            "March",
            "April",
            "May",
            "June",
            "July",
            "August",
            "September",
            "October",
            "November",
            "December",
        ];
        format!(
            "{}, {} {}",
            days[(parts.weekday as usize) % 7],
            months[parts.month.saturating_sub(1).min(11) as usize],
            parts.day.clamp(1, 31)
        )
    } else {
        String::new()
    };
    ClockFace { time, period, date }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Greeting {
    pub period: &'static str,
    pub title: &'static str,
    pub phrase: &'static str,
    pub index: usize,
}
pub fn greeting(hour: u8, choice: u64, previous: Option<usize>) -> Greeting {
    let (period, title, phrases) = match hour % 24 {
        5..=11 => (
            "morning",
            "Good morning.",
            [
                "A fresh start, at your pace.",
                "Something good to start the day.",
                "Ease into the day with a favourite.",
                "A little inspiration for your morning.",
            ],
        ),
        12..=16 => (
            "afternoon",
            "Good afternoon.",
            [
                "A moment to take a break.",
                "Settle in for something you love.",
                "Make a little time for yourself.",
                "A good afternoon for a favourite.",
            ],
        ),
        17..=21 => (
            "evening",
            "Good evening.",
            [
                "Your favourites, and time to unwind.",
                "Settle in. The evening is yours.",
                "A good story to end the day.",
                "Find something worth staying in for.",
            ],
        ),
        _ => (
            "night",
            "Good evening.",
            [
                "One more episode?",
                "A quiet moment, just for you.",
                "A little company for the late hours.",
                "Wind down with a favourite.",
            ],
        ),
    };
    let mut index = (choice % 4) as usize;
    if previous == Some(index) {
        index = (index + 1) % 4;
    }
    Greeting {
        period,
        title,
        phrase: phrases[index],
        index,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShuffleBag {
    ids: Vec<String>,
    remaining: Vec<String>,
    last: Option<String>,
    seed: u64,
}
impl ShuffleBag {
    pub fn new(seed: u64) -> Self {
        Self {
            ids: vec![],
            remaining: vec![],
            last: None,
            seed: seed.max(1),
        }
    }
    fn random(&mut self) -> u64 {
        self.seed = self.seed.max(1);
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 7;
        self.seed ^= self.seed << 17;
        self.seed
    }
    pub fn next(&mut self, ids: &[String], shuffle: bool) -> Option<String> {
        let mut unique = ids.to_vec();
        clean_ids(&mut unique, 10_000);
        if unique.is_empty() {
            return None;
        }
        if self.ids != unique {
            self.ids = unique;
            self.remaining.clear();
            if !self.ids.iter().any(|id| Some(id) == self.last.as_ref()) {
                self.last = None;
            }
        }
        let next = if !shuffle {
            self.remaining.clear();
            let at = self
                .last
                .as_ref()
                .and_then(|last| self.ids.iter().position(|id| id == last))
                .map(|i| (i + 1) % self.ids.len())
                .unwrap_or(0);
            self.ids[at].clone()
        } else {
            let mut seen = HashSet::new();
            self.remaining.retain(|id| {
                self.ids.contains(id) && self.last.as_ref() != Some(id) && seen.insert(id.clone())
            });
            if self.remaining.is_empty() {
                self.remaining = self.ids.clone();
                for i in (1..self.remaining.len()).rev() {
                    let j = (self.random() % (i as u64 + 1)) as usize;
                    self.remaining.swap(i, j);
                }
                if self.remaining.len() > 1 && self.remaining.first() == self.last.as_ref() {
                    self.remaining.swap(0, 1);
                }
            }
            self.remaining.remove(0)
        };
        self.last = Some(next.clone());
        Some(next)
    }
    pub fn remaining(&self) -> &[String] {
        &self.remaining
    }
    pub fn last(&self) -> Option<&str> {
        self.last.as_deref()
    }
    pub fn reset(&mut self) {
        self.remaining.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn app(id: &str, title: &str) -> App {
        App {
            id: id.into(),
            title: title.into(),
            ..App::default()
        }
    }
    #[test]
    fn home_selection_uses_favourites_and_successful_usage_without_inputs_or_utilities() {
        let mut input = app("com.webos.app.hdmi1", "HDMI 1");
        input.input = true;
        let mut utility = app("utility", "Utility");
        utility.utility = true;
        let apps = vec![
            app("netflix", "Netflix"),
            app("youtube", "YouTube"),
            app("unknown", "Other"),
            input,
            utility,
        ];
        let mut prefs = Preferences::default();
        prefs.record_launch("unknown", 42);
        prefs.record_launch("utility", 43);
        let ids: Vec<_> = home_apps(&apps, &prefs).into_iter().map(|a| a.id).collect();
        assert_eq!(ids, vec!["unknown", "netflix", "youtube"]);
        prefs.home_mode = HomeMode::Favorites;
        prefs.favorites = vec!["youtube".into()];
        assert_eq!(home_apps(&apps, &prefs)[0].id, "youtube");
        let no_icon: App =
            serde_json::from_str(r#"{"id":"one","title":"One","icon":null}"#).unwrap();
        assert!(no_icon.icon.is_empty());
    }
    #[test]
    fn folders_search_hidden_apps_and_manual_order_compose_without_duplicate_tiles() {
        let apps = vec![
            app("one", "First"),
            app("two", "Second"),
            app("three", "Third"),
            app("one", "Duplicate"),
        ];
        let mut prefs = Preferences::default();
        prefs.folders = vec![Folder {
            id: "folder-tv".into(),
            title: "TV".into(),
            apps: vec!["one".into()],
        }];
        prefs.hidden = vec!["three".into()];
        prefs.order = vec!["folder-tv".into()];
        assert_eq!(
            visible_apps(&apps, &prefs, "", None)
                .iter()
                .map(|a| a.id.as_str())
                .collect::<Vec<_>>(),
            vec!["folder-tv", "two"]
        );
        assert_eq!(visible_apps(&apps, &prefs, "first", None)[0].id, "one");
        assert_eq!(
            visible_apps(&apps, &prefs, "", Some("folder-tv"))[0].id,
            "one"
        );
        assert!(visible_apps(&apps, &prefs, "", Some("missing")).is_empty());
    }
    #[test]
    fn input_row_requires_down_and_only_explicit_activation_launches_hdmi() {
        let apps = vec![app("one", "One"), app("two", "Two")];
        let inputs = vec![app("com.webos.app.hdmi1", "Console")];
        let mut state = HomeState::new();
        assert!(!state.input_row_open);
        assert_eq!(
            state.move_focus(Direction::Down, 2, 1, 5),
            NavEffect::InputsRevealed
        );
        assert_eq!(state.focus, Focus::Input(0));
        assert_eq!(
            state.activate(&apps, &inputs),
            Action::Launch("com.webos.app.hdmi1".into())
        );
        assert_eq!(
            state.move_focus(Direction::Up, 2, 1, 5),
            NavEffect::InputsHidden
        );
        assert_eq!(state.focus, Focus::App(0));
        state.move_focus(Direction::Right, 2, 1, 5);
        assert_eq!(state.focus, Focus::App(1));
        assert_eq!(state.move_focus(Direction::Right, 2, 1, 5), NavEffect::None);
    }
    #[test]
    fn oled_boundaries_reduce_motion_and_held_wake_key_do_not_activate() {
        let mut prefs = Preferences::default();
        let mut idle = IdleTracker::new(0);
        assert_eq!(idle.state(119_999, &prefs, true).rest, Rest::Active);
        assert_eq!(idle.state(120_000, &prefs, true).rest, Rest::Faded);
        assert_eq!(idle.state(300_000, &prefs, true).ui_alpha, 0);
        assert_eq!(idle.state(1_800_000, &prefs, true).overlay_alpha, 247);
        assert!(idle.key_down(13, false, 1_800_000, &prefs, true));
        assert!(idle.key_down(13, true, 1_800_001, &prefs, true));
        idle.key_up(13);
        assert!(!idle.key_down(13, false, 1_800_002, &prefs, true));
        assert_eq!(idle.state(1_800_002, &prefs, true).rest, Rest::Active);
        prefs.oled_protection = false;
        assert_eq!(
            oled_state(&prefs, 1_800_000, 90_000, true).overlay_alpha,
            247
        );
        prefs.reduce_motion = true;
        assert_eq!(oled_state(&prefs, 0, 90_000, true).x, 0);
        assert_eq!(
            oled_state(&prefs, 1_800_000, 90_000, false).overlay_alpha,
            0
        );
    }
    #[test]
    fn corrupt_or_excessive_saved_preferences_are_bounded() {
        let mut prefs:Preferences=serde_json::from_str(r#"{"columns":1000,"profile":"","mediaServer":"http://example.com:8794","favorites":["safe","safe","x;reboot"],"usage":{"bad/id":{"count":1},"safe":{"count":4294967295}}}"#).unwrap();
        prefs.sanitize();
        assert_eq!(prefs.columns, 6);
        assert_eq!(prefs.profile, "Home");
        assert_eq!(prefs.favorites, vec!["safe"]);
        assert_eq!(prefs.usage.len(), 1);
        assert_eq!(prefs.usage["safe"].count, 1_000_000);
        assert!(prefs.media_server.is_empty());
        prefs.media_server = "http://localhost:8794".into();
        prefs.sanitize();
        assert_eq!(prefs.media_server, "http://localhost:8794");
    }
    #[test]
    fn clocks_cover_midnight_noon_and_explicit_formats_and_greetings_rotate() {
        let parts = ClockParts {
            hour: 0,
            minute: 5,
            weekday: 1,
            month: 11,
            day: 2,
        };
        let mut prefs = Preferences::default();
        prefs.clock_format = ClockFormat::Twelve;
        let clock = clock_face(parts, &prefs, false);
        assert_eq!(clock.time, "12:05");
        assert_eq!(clock.period, "AM");
        assert_eq!(clock.date, "Monday, November 2");
        prefs.clock_format = ClockFormat::TwentyFour;
        assert_eq!(clock_face(parts, &prefs, true).time, "00:05");
        assert_eq!(greeting(5, 0, None).period, "morning");
        assert_eq!(greeting(12, 0, None).period, "afternoon");
        assert_eq!(greeting(22, 0, None).period, "night");
        assert_ne!(greeting(5, 0, Some(0)).index, 0);
    }
    #[test]
    fn shuffled_cycles_and_serialized_progress_never_repeat_before_exhaustion() {
        let ids: Vec<_> = (0..164).map(|i| format!("scene-{i}")).collect();
        let mut bag = ShuffleBag::new(42);
        let mut seen = HashSet::new();
        for i in 0..164 {
            let id = bag.next(&ids, true).unwrap();
            assert!(seen.insert(id));
            if i == 50 {
                bag = serde_json::from_str(&serde_json::to_string(&bag).unwrap()).unwrap();
            }
        }
        let last = bag.last().unwrap().to_owned();
        assert_ne!(bag.next(&ids, true).unwrap(), last);
        assert_eq!(ShuffleBag::new(1).next(&ids, false), Some("scene-0".into()));
    }
    #[test]
    fn corrupt_shuffle_remainders_cannot_repeat_or_escape_the_catalogue() {
        let ids = vec!["one".into(), "two".into(), "three".into()];
        let mut bag = ShuffleBag {
            ids: ids.clone(),
            remaining: vec!["one".into(), "two".into(), "two".into(), "unknown".into()],
            last: Some("one".into()),
            seed: 0,
        };
        assert_eq!(bag.next(&ids, true), Some("two".into()));
        assert!(!bag.remaining().contains(&"two".into()));
        assert_ne!(bag.next(&ids, true), Some("two".into()));
    }
}
