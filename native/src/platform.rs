//! Bounded, asynchronous TV requests. The rendering thread never waits for Luna
//! services, disk IO or artwork. Only typed actions can launch an application.
use serde_json::{json, Value};
use std::{
    fs,
    io::{Read, Write},
    net::{Ipv4Addr, SocketAddr, TcpStream},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::mpsc::{self, Receiver, SyncSender},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const APP_ID: &str = "io.github.justmaaarco.aerialhome.nativeprobe";
const MAX_REPLY: usize = 2_200_000;
const MAX_ARTWORK: usize = 1_500_000;
const MAX_PREFS: usize = 262_144;
const MAX_APP_CACHE: usize = 524_288;

#[derive(Clone, Debug)]
pub enum Request {
    Inventory,
    Inputs,
    Capabilities,
    Favorites,
    Artwork { id: String, path: String },
    Launch(String),
    AudioStatus,
    SetVolume(u8),
    SetMute(bool),
    SleepTimer(u16),
    PrepareAerial { helper: String, scene_id: String },
    LoadPreferences,
    SavePreferences(Value),
    Shutdown,
}

#[derive(Debug)]
pub struct Response {
    pub request: Request,
    pub result: Result<Value, String>,
}

pub struct Worker {
    pub sender: SyncSender<Request>,
    pub receiver: Receiver<Response>,
}

impl Worker {
    /// An offline renderer cannot send TV commands or start service threads.
    pub fn idle() -> Self {
        let (sender, requests) = mpsc::sync_channel(1);
        let (responses, receiver) = mpsc::channel();
        drop(requests);
        drop(responses);
        Self { sender, receiver }
    }
    pub fn start() -> Self {
        Self::with_directory(data_directory())
    }

    /// A separate directory is useful for desktop preview/testing. The native
    /// default is inside this app's own installation, never the web app's data.
    pub fn with_directory(directory: PathBuf) -> Self {
        let (sender, requests) = mpsc::sync_channel::<Request>(32);
        let (results, receiver) = mpsc::sync_channel(32);
        let (artwork_sender, artwork_requests) = mpsc::sync_channel::<Request>(16);
        let (media_sender, media_requests) = mpsc::sync_channel::<Request>(2);
        let media_directory = directory.clone();
        let media_results = results.clone();
        thread::Builder::new()
            .name("aerial-media-requests".into())
            .spawn(move || {
                while let Ok(request) = media_requests.recv() {
                    let result = perform(&request, &media_directory);
                    if media_results.send(Response { request, result }).is_err() {
                        break;
                    }
                }
            })
            .expect("Could not start media request worker");
        let artwork_directory = directory.clone();
        let artwork_results = results.clone();
        thread::Builder::new()
            .name("aerial-artwork".into())
            .spawn(move || {
                while let Ok(request) = artwork_requests.recv() {
                    let result = perform(&request, &artwork_directory);
                    if let Err(error) = &result {
                        eprintln!("native artwork unavailable: {error}");
                    }
                    if artwork_results.send(Response { request, result }).is_err() {
                        break;
                    }
                }
            })
            .expect("Could not start artwork worker");
        thread::Builder::new()
            .name("aerial-tv-requests".into())
            .spawn(move || {
                while let Ok(request) = requests.recv() {
                    if matches!(request, Request::Shutdown) {
                        break;
                    }
                    if matches!(request, Request::PrepareAerial { .. }) {
                        if let Err(error) = media_sender.try_send(request) {
                            let request = match error {
                                mpsc::TrySendError::Full(r)
                                | mpsc::TrySendError::Disconnected(r) => r,
                            };
                            if results
                                .send(Response {
                                    request,
                                    result: Err("Aerial helper queue is full; retry later".into()),
                                })
                                .is_err()
                            {
                                break;
                            }
                        }
                        continue;
                    }
                    if matches!(request, Request::Artwork { .. }) {
                        // Never let a full icon queue block an explicit app launch,
                        // volume change or settings save on the control worker.
                        if let Err(error) = artwork_sender.try_send(request) {
                            let request = match error {
                                mpsc::TrySendError::Full(r)
                                | mpsc::TrySendError::Disconnected(r) => r,
                            };
                            if results
                                .send(Response {
                                    request,
                                    result: Err("Artwork queue is full; retry later".into()),
                                })
                                .is_err()
                            {
                                break;
                            }
                        }
                        continue;
                    }
                    let result = perform(&request, &directory);
                    if results.send(Response { request, result }).is_err() {
                        break;
                    }
                }
            })
            .expect("Could not start TV request worker");
        Self { sender, receiver }
    }
}

fn data_directory() -> PathBuf {
    storage_directory().unwrap_or_else(|error| {
        eprintln!("native storage unavailable: {error}; using temporary app data");
        std::env::temp_dir().join(APP_ID)
    })
}

/// Resolve paths in the native jail's namespace. /proc/self/exe can expose the
/// host /var/palm/jail prefix even though that prefix does not exist in-jail.
pub(crate) fn install_directory() -> Result<PathBuf, String> {
    let executable = std::env::current_exe().ok();
    let mut candidates = Vec::new();
    if let Some(home) = std::env::var_os("HOME") {
        candidates.push(PathBuf::from(home));
    }
    if let Some(executable) = &executable {
        if let Some(parent) = executable.parent() {
            candidates.push(parent.to_path_buf());
        }
        let prefix = format!("/var/palm/jail/{APP_ID}/");
        if let Some(path) = executable.to_str().and_then(|p| p.strip_prefix(&prefix)) {
            if let Some(parent) = Path::new(&format!("/{path}")).parent() {
                candidates.push(parent.to_path_buf());
            }
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd);
    }
    candidates.extend([
        PathBuf::from(format!(
            "/media/developer/apps/usr/palm/applications/{APP_ID}"
        )),
        PathBuf::from(format!(
            "/media/cryptofs/apps/usr/palm/applications/{APP_ID}"
        )),
        PathBuf::from(format!("/usr/palm/applications/{APP_ID}")),
    ]);
    for directory in candidates {
        if valid_install_directory(&directory) {
            return Ok(directory);
        }
    }
    Err("Cannot locate this native app's installation manifest".into())
}

fn valid_install_directory(directory: &Path) -> bool {
    let file = directory.join("appinfo.json");
    if fs::metadata(&file).map(|m| m.len() > 16384).unwrap_or(true) {
        return false;
    }
    fs::read(file)
        .ok()
        .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
        .is_some_and(|v| v["id"] == APP_ID && v["type"] == "native")
}

/// Persistent, app-owned data outside the read-only installed payload. Root
/// Home clients reuse an existing app-owned directory and never create one.
pub(crate) fn storage_directory() -> Result<PathBuf, String> {
    #[cfg(all(feature = "sdl", target_os = "linux", target_arch = "arm"))]
    {
        extern "C" {
            fn geteuid() -> u32;
        }
        let user = unsafe { geteuid() };
        return owned_storage_directory(
            &[
                PathBuf::from(format!("/media/developer/{APP_ID}")),
                PathBuf::from(format!("/media/developer/temp/{APP_ID}")),
                PathBuf::from(format!("/media/internal/.{APP_ID}")),
            ],
            user,
            user != 0,
        );
    }
    #[cfg(not(all(feature = "sdl", target_os = "linux", target_arch = "arm")))]
    {
        let base = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|p| PathBuf::from(p).join(".config")))
            .ok_or("No app configuration directory")?;
        let directory = base.join(APP_ID);
        fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
        Ok(directory)
    }
}

#[cfg(unix)]
fn owned_storage_directory(
    candidates: &[PathBuf],
    user: u32,
    create: bool,
) -> Result<PathBuf, String> {
    use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt};
    let mut ordered = candidates.to_vec();
    ordered.sort_by_key(|directory| {
        let meta = fs::symlink_metadata(directory).ok().filter(|m| m.is_dir());
        (
            meta.is_none(),
            user == 0 && meta.as_ref().is_some_and(|m| m.uid() == 0),
        )
    });
    let mut failures = Vec::new();
    for directory in ordered {
        if create && !directory.exists() {
            if let Err(error) = fs::DirBuilder::new().mode(0o700).create(&directory) {
                failures.push(format!("{}: {error}", directory.display()));
                continue;
            }
        }
        let meta = match fs::symlink_metadata(&directory) {
            Ok(m) if m.is_dir() && (user == 0 || m.uid() == user) => m,
            _ => {
                failures.push(format!(
                    "{}: not an app-owned directory",
                    directory.display()
                ));
                continue;
            }
        };
        if user == 0 {
            return Ok(directory);
        }
        if meta.permissions().readonly() {
            continue;
        }
        let probe = directory.join(format!(".write-probe-{}", std::process::id()));
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&probe)
        {
            Ok(mut file) => {
                let write = file
                    .write_all(b"native-storage-check")
                    .and_then(|_| file.sync_all());
                drop(file);
                let remove = fs::remove_file(&probe);
                if write.is_ok() && remove.is_ok() {
                    return Ok(directory);
                }
            }
            Err(error) => failures.push(format!("{}: {error}", directory.display())),
        }
    }
    Err(format!(
        "No writable persistent native data directory: {}",
        failures.join("; ")
    ))
}

pub fn valid_app_id(id: &str) -> bool {
    (1..=200).contains(&id.len())
        && id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c))
        && !id.starts_with('.')
        && !id.ends_with('.')
        && !id.contains("..")
}

pub fn valid_artwork_path(path: &str) -> bool {
    let roots = [
        "/usr/palm/applications/",
        "/mnt/otncabi/usr/palm/applications/",
        "/media/developer/apps/usr/palm/applications/",
        "/media/cryptofs/apps/usr/palm/applications/",
        "/media/system/apps/usr/palm/applications/",
    ];
    let lower = path.to_ascii_lowercase();
    path.len() <= 1024
        && roots.iter().any(|root| path.starts_with(root))
        && !path.split('/').any(|part| part == ".." || part == ".")
        && path
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"/_.@+-".contains(&c))
        && [".png", ".jpg", ".jpeg", ".webp"]
            .iter()
            .any(|ext| lower.ends_with(ext))
}

fn artwork_path(icon: &str, folder: &str) -> Option<String> {
    let icon = icon.strip_prefix("file://").unwrap_or(icon);
    let path = if icon.starts_with('/') {
        icon.to_owned()
    } else {
        format!("{}/{}", folder.trim_end_matches('/'), icon)
    };
    valid_artwork_path(&path).then_some(path)
}

#[derive(Debug, PartialEq)]
struct ArtworkMetadata {
    path: String,
    color: Option<String>,
}

fn artwork_metadata(reply: &Value, fallback: &str) -> Result<ArtworkMetadata, String> {
    let info = &reply["appInfo"];
    let folder = info["folderPath"].as_str().unwrap_or("");
    // Match the existing web launcher: getAppInfo supplies the large artwork,
    // while the launch-point path remains a fallback on older firmware. Each
    // candidate is independently validated, so a bad large icon cannot hide a
    // usable smaller icon or make the root artwork reader access another tree.
    let path = ["extraLargeIcon", "largeIcon"]
        .iter()
        .filter_map(|key| info[*key].as_str())
        .find_map(|icon| artwork_path(icon, folder))
        .or_else(|| valid_artwork_path(fallback).then(|| fallback.to_owned()))
        .ok_or("Unsupported artwork path")?;
    let color = info["iconColor"]
        .as_str()
        .filter(|color| {
            color.len() == 7
                && color.starts_with('#')
                && color[1..].bytes().all(|byte| byte.is_ascii_hexdigit())
        })
        .map(str::to_owned);
    Ok(ArtworkMetadata { path, color })
}

fn hydrate_artwork(id: &str, fallback: &str, directory: &Path) -> Result<Value, String> {
    if !valid_app_id(id) {
        return Err("Invalid application ID".into());
    }
    // This function runs only in the artwork worker. The UI and app-launch
    // worker stay responsive while SAM metadata or the root reader responds.
    let info = luna_fallback("com.webos.applicationManager/getAppInfo", json!({"id":id}))
        .unwrap_or(Value::Null);
    let metadata = artwork_metadata(&info, fallback)?;
    let mut reply = cache_artwork(id, &metadata.path, directory)?;
    if let Some(color) = metadata.color {
        reply["color"] = Value::String(color);
    }
    Ok(reply)
}

fn inventory(reply: Value) -> Value {
    let apps = reply.get("launchPoints").and_then(Value::as_array).into_iter().flatten().filter_map(|a| {
        let id = a.get("id")?.as_str()?;
        let title = a.get("title")?.as_str()?;
        if !valid_app_id(id) || title.is_empty() || a.get("visible")==Some(&Value::Bool(false)) || id==APP_ID || id=="io.github.justmaaarco.aerialhome" { return None; }
        let icon = ["extraLargeIcon", "largeIcon", "icon"].iter().find_map(|k| a.get(k).and_then(Value::as_str)).and_then(|icon| artwork_path(icon, a.get("folderPath").and_then(Value::as_str).unwrap_or("")));
        let color=a.get("iconColor").and_then(Value::as_str).filter(|s| s.len()==7 && s.starts_with('#') && s[1..].bytes().all(|c|c.is_ascii_hexdigit())).unwrap_or("#394b68");
        Some(json!({"id":id,"title":title.chars().take(160).collect::<String>(),"icon":icon,"input":id.contains(".hdmi"),"color":color}))
    }).take(256).collect::<Vec<_>>();
    json!({"apps":apps})
}

fn inputs(reply: Value) -> Value {
    let apps=reply.get("devices").or_else(||reply.get("deviceList")).and_then(Value::as_array).into_iter().flatten().filter_map(|a| {
        let id=a.get("appId").or_else(||a.get("app_id"))?.as_str()?;
        let suffix=id.strip_prefix("com.webos.app.hdmi").or_else(||id.strip_prefix("com.palm.app.hdmi"))?;
        if !matches!(suffix,"1"|"2"|"3"|"4") {return None;}
        let title=a.get("label").or_else(||a.get("name")).and_then(Value::as_str).filter(|s|!s.is_empty()).map(str::to_owned).unwrap_or_else(||format!("HDMI {}",suffix));
        Some(json!({"id":id,"title":title.chars().take(160).collect::<String>(),"icon":null,"input":true,"color":"#27354c"}))
    }).take(4).collect::<Vec<_>>();
    json!({"apps":apps})
}

fn perform(request: &Request, directory: &Path) -> Result<Value, String> {
    match request {
        Request::Inventory => {
            let value = inventory(luna_fallback(
                "com.webos.applicationManager/listLaunchPoints",
                json!({}),
            )?);
            // Optional cache never blocks usable live discovery.
            if let Err(error) = update_cached_group(directory, &value, false) {
                eprintln!("native metadata cache unavailable: {error}");
            }
            Ok(value)
        }
        Request::Inputs => {
            let value = inputs(luna_fallback(
                "com.webos.service.eim/getAllInputStatus",
                json!({}),
            )?);
            if let Err(error) = update_cached_group(directory, &value, true) {
                eprintln!("native input cache unavailable: {error}");
            }
            Ok(value)
        }
        Request::Capabilities => {
            let reply = luna_fallback(
                "com.webos.service.config/getConfigs",
                json!({"configNames":["tv.model.supportHDR","tv.config.supportDolbyHDRContents","tv.hw.panelResolution"]}),
            )?;
            let configs = &reply["configs"];
            Ok(
                json!({"dolbyVision":configs["tv.config.supportDolbyHDRContents"]==true,"hdr10":configs["tv.model.supportHDR"]==true,"uhd":matches!(configs["tv.hw.panelResolution"].as_str(),Some("UD"|"8K"))}),
            )
        }
        Request::Favorites => {
            let reply = luna(
                "org.webosbrew.hbchannel.service/exec",
                json!({"command":"if [ -f /media/developer/apps/usr/palm/applications/com.ecklf.luma/preferences.json ]; then cat /media/developer/apps/usr/palm/applications/com.ecklf.luma/preferences.json; else printf '{}'; fi"}),
            )?;
            let prefs: Value = serde_json::from_str(reply["stdoutString"].as_str().unwrap_or("{}"))
                .map_err(|_| "Invalid favourite response".to_string())?;
            let favorites = prefs["favorites"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .filter(|id| valid_app_id(id))
                .take(64)
                .collect::<Vec<_>>();
            Ok(json!({"favorites":favorites}))
        }
        Request::Artwork { id, path } => hydrate_artwork(id, path, directory),
        Request::Launch(id) => {
            if !valid_app_id(id) {
                return Err("Invalid application ID".into());
            }
            luna_fallback("com.webos.applicationManager/launch", json!({"id":id}))
        }
        Request::AudioStatus => {
            let reply = luna_fallback("com.webos.audio/getVolume", json!({}))?;
            let status = reply.get("volumeStatus").unwrap_or(&reply);
            Ok(
                json!({"volume":status["volume"].as_u64().unwrap_or(0).min(100),"muted":status["muteStatus"].as_bool().unwrap_or(false)}),
            )
        }
        Request::SetVolume(volume) => luna_fallback(
            "com.webos.audio/setVolume",
            json!({"volume":(*volume).min(100)}),
        ),
        Request::SetMute(muted) => {
            luna_fallback("com.webos.audio/setMuted", json!({"muted":muted}))
        }
        Request::SleepTimer(minutes) => {
            if !matches!(minutes, 0 | 15 | 30 | 60 | 90) {
                return Err("Unsupported sleep timer".into());
            }
            luna_fallback(
                "com.webos.service.tvReservationAgent/timer/sleep/add",
                json!({"sleepTimeMinute":minutes}),
            )
        }
        Request::PrepareAerial { helper, scene_id } => prepare_aerial(helper, scene_id),
        Request::LoadPreferences => load_preferences(directory),
        Request::SavePreferences(value) => {
            save_preferences(directory, value)?;
            Ok(json!({"saved":true}))
        }
        Request::Shutdown => Ok(json!({})),
    }
}

fn luna_fallback(method: &str, payload: Value) -> Result<Value, String> {
    match luna(method, payload.clone()) {
        Ok(reply) => Ok(reply),
        Err(direct_error) => {
            // All callers use constant method names and payloads made only from
            // validated app IDs, booleans and bounded numeric values. Never put
            // titles, artwork URLs or arbitrary preferences in this command.
            let safe_method = matches!(
                method,
                "com.webos.applicationManager/listLaunchPoints"
                    | "com.webos.applicationManager/getAppInfo"
                    | "com.webos.service.eim/getAllInputStatus"
                    | "com.webos.service.config/getConfigs"
                    | "com.webos.applicationManager/launch"
                    | "com.webos.audio/getVolume"
                    | "com.webos.audio/setVolume"
                    | "com.webos.audio/setMuted"
                    | "com.webos.service.tvReservationAgent/timer/sleep/add"
            );
            let json = serde_json::to_string(&payload).map_err(|e| e.to_string())?;
            if !safe_method || json.contains('\'') {
                return Err(direct_error);
            }
            let command = format!("luna-send -n 1 -w 4000 luna://{} '{}'", method, json);
            let reply = luna(
                "org.webosbrew.hbchannel.service/exec",
                json!({"command":command}),
            )?;
            if reply.get("errorCode").and_then(Value::as_i64).unwrap_or(0) != 0 {
                return Err("Homebrew request failed".into());
            }
            let value: Value = serde_json::from_str(
                reply["stdoutString"]
                    .as_str()
                    .ok_or("Missing Homebrew response")?,
            )
            .map_err(|_| "Invalid Homebrew response".to_string())?;
            successful(value)
        }
    }
}

fn successful(reply: Value) -> Result<Value, String> {
    if reply.get("returnValue") == Some(&Value::Bool(false)) {
        return Err(reply
            .get("errorText")
            .and_then(Value::as_str)
            .unwrap_or("TV request denied")
            .chars()
            .take(240)
            .collect());
    }
    Ok(reply)
}

fn luna(method: &str, payload: Value) -> Result<Value, String> {
    call_native(method, payload)
}

/// A bounded in-process LS2 round trip in the ARM TV build. Native jails do
/// not permit spawning luna-send, even when its method would otherwise work.
/// Every call owns its GLib context and connection on the calling worker.
pub(crate) fn call_native(method: &str, payload: Value) -> Result<Value, String> {
    let method = method.strip_prefix("luna://").unwrap_or(method);
    if method.is_empty()
        || method.len() > 400
        || !method
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"._-/".contains(&c))
    {
        return Err("Invalid TV service method".into());
    }
    #[cfg(all(feature = "sdl", target_os = "linux", target_arch = "arm"))]
    {
        return successful(native_ls2::request(&format!("luna://{}", method), payload)?);
    }
    #[cfg(not(all(feature = "sdl", target_os = "linux", target_arch = "arm")))]
    {
        luna_process(method, payload)
    }
}

#[cfg(not(all(feature = "sdl", target_os = "linux", target_arch = "arm")))]
fn luna_process(method: &str, payload: Value) -> Result<Value, String> {
    let arguments = [
        "-n".to_string(),
        "1".into(),
        "-w".into(),
        "4500".into(),
        format!("luna://{}", method),
        serde_json::to_string(&payload).map_err(|e| e.to_string())?,
    ];
    let data = run_bounded(
        Path::new("/usr/bin/luna-send"),
        &arguments,
        Duration::from_millis(6500),
        MAX_REPLY,
    )?;
    let reply: Value =
        serde_json::from_slice(&data).map_err(|_| "The TV returned invalid JSON".to_string())?;
    successful(reply)
}

// Public NDK LS2/GLib ABI; uses the installed executable's own role rather than
// impersonating an app ID with LSCallFromApplication. This pattern is also used
// by Luma's native client; all callback storage survives cancellation/unregister.
#[cfg(all(feature = "sdl", target_os = "linux", target_arch = "arm"))]
mod native_ls2 {
    use super::*;
    use std::{
        ffi::{c_char, c_int, c_ulong, c_void, CStr, CString},
        ptr,
    };
    #[repr(C)]
    struct LsError {
        code: c_int,
        message: *mut c_char,
        file: *const c_char,
        line: c_int,
        function: *const c_char,
        padding: *mut c_void,
        magic: c_ulong,
    }
    #[link(name = "luna-service2")]
    extern "C" {
        fn LSErrorInit(error: *mut LsError) -> bool;
        fn LSErrorFree(error: *mut LsError);
        fn LSRegister(name: *const c_char, handle: *mut *mut c_void, error: *mut LsError) -> bool;
        fn LSUnregister(handle: *mut c_void, error: *mut LsError) -> bool;
        fn LSGmainContextAttach(
            handle: *mut c_void,
            context: *mut c_void,
            error: *mut LsError,
        ) -> bool;
        fn LSCallOneReply(
            handle: *mut c_void,
            uri: *const c_char,
            payload: *const c_char,
            callback: extern "C" fn(*mut c_void, *mut c_void, *mut c_void) -> bool,
            userdata: *mut c_void,
            token: *mut c_ulong,
            error: *mut LsError,
        ) -> bool;
        fn LSCallCancel(handle: *mut c_void, token: c_ulong, error: *mut LsError) -> bool;
        fn LSMessageGetPayload(message: *mut c_void) -> *const c_char;
    }
    #[link(name = "glib-2.0")]
    extern "C" {
        fn g_main_context_new() -> *mut c_void;
        fn g_main_context_iteration(context: *mut c_void, may_block: c_int) -> c_int;
        fn g_main_context_unref(context: *mut c_void);
    }
    extern "C" {
        fn strnlen(value: *const c_char, max: usize) -> usize;
    }
    impl LsError {
        fn new() -> Self {
            unsafe {
                let mut e = std::mem::zeroed();
                LSErrorInit(&mut e);
                e
            }
        }
        fn describe(&self, stage: &str) -> String {
            let message = if self.message.is_null() {
                String::new()
            } else {
                unsafe { CStr::from_ptr(self.message) }
                    .to_string_lossy()
                    .chars()
                    .take(240)
                    .collect()
            };
            format!("LS2 {} failed ({}): {}", stage, self.code, message)
        }
    }
    impl Drop for LsError {
        fn drop(&mut self) {
            unsafe {
                LSErrorFree(self);
            }
        }
    }
    #[derive(Default)]
    struct Reply {
        result: Option<Result<Value, String>>,
    }
    extern "C" fn receive(_: *mut c_void, message: *mut c_void, userdata: *mut c_void) -> bool {
        if message.is_null() || userdata.is_null() {
            return true;
        }
        unsafe {
            let reply = &mut *userdata.cast::<Reply>();
            if reply.result.is_some() {
                return true;
            }
            let payload = LSMessageGetPayload(message);
            reply.result = Some(if payload.is_null() {
                Err("TV reply has no payload".into())
            } else {
                let length = strnlen(payload, MAX_REPLY + 1);
                if length > MAX_REPLY {
                    Err("TV response exceeded the size limit".into())
                } else {
                    serde_json::from_slice(std::slice::from_raw_parts(payload.cast::<u8>(), length))
                        .map_err(|_| "The TV returned invalid JSON".into())
                }
            });
        }
        true
    }
    struct Session {
        handle: *mut c_void,
        context: *mut c_void,
        reply: Option<Box<Reply>>,
        token: c_ulong,
    }
    impl Drop for Session {
        fn drop(&mut self) {
            unsafe {
                if !self.handle.is_null() {
                    let mut error = LsError::new();
                    if !LSUnregister(self.handle, &mut error) {
                        // If the hub refuses unregister it may retain the callback.
                        // Preserve the heap/context instead of leaving dangling FFI pointers.
                        std::mem::forget(self.reply.take());
                        return;
                    }
                }
                if !self.context.is_null() {
                    g_main_context_unref(self.context);
                }
            }
        }
    }
    pub(super) fn request(uri: &str, payload: Value) -> Result<Value, String> {
        let uri = CString::new(uri).map_err(|_| "Invalid Luna URI")?;
        let encoded = payload.to_string();
        if encoded.len() > MAX_REPLY {
            return Err("TV request exceeds the size limit".into());
        }
        let payload = CString::new(encoded).map_err(|_| "Invalid Luna payload")?;
        let mut session = Session {
            handle: ptr::null_mut(),
            context: unsafe { g_main_context_new() },
            reply: Some(Box::default()),
            token: 0,
        };
        if session.context.is_null() {
            return Err("Could not allocate TV service context".into());
        }
        let mut error = LsError::new();
        if !unsafe { LSRegister(ptr::null(), &mut session.handle, &mut error) } {
            return Err(error.describe("registration"));
        }
        if !unsafe { LSGmainContextAttach(session.handle, session.context, &mut error) } {
            return Err(error.describe("context attachment"));
        }
        let userdata = (&mut **session.reply.as_mut().unwrap() as *mut Reply).cast();
        if !unsafe {
            LSCallOneReply(
                session.handle,
                uri.as_ptr(),
                payload.as_ptr(),
                receive,
                userdata,
                &mut session.token,
                &mut error,
            )
        } {
            return Err(error.describe("call submission"));
        }
        let deadline = Instant::now() + Duration::from_millis(6500);
        while Instant::now() < deadline {
            unsafe {
                g_main_context_iteration(session.context, 0);
            }
            if let Some(result) = session.reply.as_mut().and_then(|r| r.result.take()) {
                return result;
            }
            thread::sleep(Duration::from_millis(2));
        }
        let mut cancel_error = LsError::new();
        unsafe {
            LSCallCancel(session.handle, session.token, &mut cancel_error);
        }
        Err("The TV service timed out".into())
    }
}

fn run_bounded(
    executable: &Path,
    arguments: &[String],
    timeout: Duration,
    limit: usize,
) -> Result<Vec<u8>, String> {
    let mut child = Command::new(executable)
        .args(arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("TV service unavailable: {}", e))?;
    // Keep stdin open: this firmware's luna-send can exit before making a call
    // when its parent immediately closes standard input.
    let input = child.stdin.take();
    let mut stdout = child.stdout.take().ok_or("Missing service output")?;
    #[cfg(target_os = "linux")]
    {
        use std::os::fd::AsRawFd;
        extern "C" {
            fn fcntl(fd: i32, command: i32, ...) -> i32;
        }
        let flags = unsafe { fcntl(stdout.as_raw_fd(), 3) };
        if flags < 0 || unsafe { fcntl(stdout.as_raw_fd(), 4, flags | 0x800) } < 0 {
            let _ = child.kill();
            let _ = child.wait();
            return Err("Cannot bound TV service reads".into());
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = child.kill();
        let _ = child.wait();
        return Err("TV service calls require a Unix host".into());
    }
    let deadline = Instant::now() + timeout;
    let mut data = Vec::new();
    let mut buffer = [0u8; 4096];
    let result = loop {
        // Firmware wrappers can fork children inheriting stdout. Waiting for
        // EOF (even after the wrapper exits) would hang forever on that pipe.
        // Read nonblocking and accept the first complete JSON reply instead.
        loop {
            if Instant::now() >= deadline {
                break;
            }
            match stdout.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => {
                    data.extend_from_slice(&buffer[..n]);
                    if data.len() > limit {
                        break;
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => break,
            }
        }
        if data.len() > limit {
            break Err("TV response exceeded the size limit".into());
        }
        if serde_json::from_slice::<Value>(&data).is_ok() {
            break Ok(data);
        }
        if Instant::now() >= deadline {
            break Err("The TV did not respond in time".into());
        }
        // Reap a finished wrapper, but a forked child may still be writing its
        // valid reply. Its inherited descriptor remains bounded by the deadline.
        let _ = child.try_wait();
        thread::sleep(Duration::from_millis(10));
    };
    let _ = child.kill();
    let _ = child.wait();
    drop(input);
    result
}

fn load_preferences(directory: &Path) -> Result<Value, String> {
    let path = directory.join("preferences.json");
    let mut value = if path.exists() {
        if fs::metadata(&path).map_err(|e| e.to_string())?.len() > MAX_PREFS as u64 {
            return Err("Saved preferences are too large".into());
        }
        serde_json::from_slice::<Value>(&fs::read(path).map_err(|e| e.to_string())?)
            .map_err(|_| "Saved preferences are invalid".to_string())?
    } else {
        json!({})
    };
    if !value.is_object() {
        return Err("Saved preferences must be an object".into());
    }
    if let Some(apps) = load_apps(directory) {
        value["cachedApps"] = apps;
    }
    Ok(value)
}

fn save_apps(directory: &Path, value: &Value) -> Result<(), String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs();
    let bytes = serde_json::to_vec(&json!({"schema":1,"updatedAt":now,"apps":value["apps"]}))
        .map_err(|e| e.to_string())?;
    if bytes.len() > MAX_APP_CACHE {
        return Err("App cache too large".into());
    }
    fs::create_dir_all(directory).map_err(|e| e.to_string())?;
    let temporary = directory.join("apps.json.pending");
    fs::write(&temporary, bytes).map_err(|e| e.to_string())?;
    fs::rename(temporary, directory.join("apps.json")).map_err(|e| e.to_string())
}

fn load_apps(directory: &Path) -> Option<Value> {
    let path = directory.join("apps.json");
    if fs::metadata(&path).ok()?.len() > MAX_APP_CACHE as u64 {
        return None;
    }
    let cache: Value = serde_json::from_slice(&fs::read(path).ok()?).ok()?;
    let now = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs();
    let saved = cache["updatedAt"].as_u64()?;
    if cache["schema"] != 1 || saved > now + 300 || now.saturating_sub(saved) > 7 * 24 * 3600 {
        return None;
    }
    let apps = cache["apps"].as_array()?;
    if apps.len() > 260 {
        return None;
    }
    // Revalidate persisted IDs/paths instead of trusting a local JSON file as
    // executable input. Cache is advisory; background discovery stays final.
    let source=apps.iter().filter(|app|app["input"]!=true).map(|app|json!({"id":app["id"],"title":app["title"],"icon":app["icon"],"iconColor":app["color"]})).collect::<Vec<_>>();
    let mut combined = inventory(json!({"launchPoints":source}))["apps"]
        .as_array()?
        .iter()
        .filter(|app| app["input"] != true)
        .cloned()
        .collect::<Vec<_>>();
    let input_source = apps
        .iter()
        .filter(|app| app["input"] == true)
        .map(|app| json!({"appId":app["id"],"label":app["title"]}))
        .collect::<Vec<_>>();
    combined.extend(
        inputs(json!({"devices":input_source}))["apps"]
            .as_array()?
            .iter()
            .cloned(),
    );
    Some(Value::Array(combined))
}

fn update_cached_group(directory: &Path, value: &Value, input_group: bool) -> Result<(), String> {
    // Inventory and EIM are independent authorities. A completed refresh
    // replaces its own group while retaining the other group's latest cache.
    let old = load_apps(directory)
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default();
    let mut combined = old
        .into_iter()
        .filter(|app| (app["input"] == true) != input_group)
        .collect::<Vec<_>>();
    combined.extend(
        value["apps"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|app| (app["input"] == true) == input_group)
            .cloned(),
    );
    save_apps(directory, &json!({"apps":combined}))
}

fn save_preferences(directory: &Path, value: &Value) -> Result<(), String> {
    if !value.is_object() {
        return Err("Preferences must be an object".into());
    }
    let mut prefs = value.clone();
    prefs.as_object_mut().unwrap().remove("cachedApps");
    let bytes = serde_json::to_vec(&prefs).map_err(|e| e.to_string())?;
    if bytes.len() > MAX_PREFS {
        return Err("Preferences exceed the size limit".into());
    }
    fs::create_dir_all(directory).map_err(|e| format!("Cannot create app data: {}", e))?;
    let temporary = directory.join("preferences.json.pending");
    let mut file = fs::File::create(&temporary).map_err(|e| e.to_string())?;
    file.write_all(&bytes)
        .and_then(|_| file.sync_all())
        .map_err(|e| e.to_string())?;
    fs::rename(temporary, directory.join("preferences.json")).map_err(|e| e.to_string())
}

fn decode_base64(input: &str) -> Result<Vec<u8>, String> {
    if input.len() > MAX_ARTWORK * 4 / 3 + 8 {
        return Err("Artwork exceeds the size limit".into());
    }
    let mut output = Vec::with_capacity(input.len() * 3 / 4);
    let mut bits = 0u32;
    let mut count = 0;
    let mut padded = false;
    for byte in input.bytes() {
        if byte.is_ascii_whitespace() {
            continue;
        }
        if byte == b'=' {
            padded = true;
            continue;
        }
        if padded {
            return Err("Invalid artwork encoding".into());
        }
        let n = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return Err("Invalid artwork encoding".into()),
        };
        bits = (bits << 6) | n as u32;
        count += 6;
        if count >= 8 {
            count -= 8;
            output.push((bits >> count) as u8);
        }
        if output.len() > MAX_ARTWORK {
            return Err("Artwork exceeds the size limit".into());
        }
    }
    if count == 6 || (count > 0 && bits & ((1 << count) - 1) != 0) {
        return Err("Truncated artwork encoding".into());
    }
    Ok(output)
}

fn helper_address(helper: &str) -> Result<(String, SocketAddr), String> {
    let raw = helper
        .strip_prefix("http://")
        .ok_or("Use a local HTTP helper address")?;
    if raw.len() > 80
        || raw
            .bytes()
            .any(|c| !c.is_ascii_alphanumeric() && !b".:".contains(&c))
    {
        return Err("Invalid helper address".into());
    }
    let (host, port) = raw.rsplit_once(':').ok_or("Helper needs a port number")?;
    let ip = if host == "localhost" {
        Ipv4Addr::LOCALHOST
    } else {
        host.parse::<Ipv4Addr>()
            .map_err(|_| "Helper must use a private IPv4 address")?
    };
    if !(ip.is_private() || ip.is_loopback()) {
        return Err("Helper must be on the local network".into());
    }
    let port = port
        .parse::<u16>()
        .ok()
        .filter(|p| *p > 0)
        .ok_or("Invalid helper port")?;
    Ok((
        format!("http://{}:{}", ip, port),
        SocketAddr::from((ip, port)),
    ))
}

fn unchunk(body: &[u8]) -> Result<Vec<u8>, String> {
    let mut result = Vec::new();
    let mut at = 0;
    loop {
        let end = body[at..]
            .windows(2)
            .position(|w| w == b"\r\n")
            .map(|n| at + n)
            .ok_or("Incomplete helper response")?;
        let line = std::str::from_utf8(&body[at..end]).map_err(|_| "Invalid helper response")?;
        let size = usize::from_str_radix(line.split(';').next().unwrap_or(""), 16)
            .map_err(|_| "Invalid helper chunk")?;
        at = end + 2;
        if size == 0 {
            return Ok(result);
        }
        if size > 64 * 1024
            || at
                .checked_add(size + 2)
                .filter(|n| *n <= body.len())
                .is_none()
        {
            return Err("Invalid helper chunk size".into());
        }
        result.extend_from_slice(&body[at..at + size]);
        if result.len() > 64 * 1024 {
            return Err("Helper response too large".into());
        }
        at += size;
        if &body[at..at + 2] != b"\r\n" {
            return Err("Invalid helper chunk ending".into());
        }
        at += 2;
    }
}

fn prepare_aerial(helper: &str, scene_id: &str) -> Result<Value, String> {
    let (origin, address) = helper_address(helper)?;
    if scene_id.is_empty()
        || scene_id.len() > 120
        || !scene_id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-')
    {
        return Err("Invalid aerial ID".into());
    }
    let deadline = Instant::now() + Duration::from_secs(3);
    let mut stream = TcpStream::connect_timeout(&address, Duration::from_millis(900))
        .map_err(|_| "The aerial helper is unavailable")?;
    stream
        .set_write_timeout(Some(Duration::from_millis(700)))
        .map_err(|e| e.to_string())?;
    write!(
        stream,
        "GET /api/prepare/{} HTTP/1.0\r\nHost: {}\r\nConnection: close\r\n\r\n",
        scene_id, address
    )
    .map_err(|_| "Cannot query the aerial helper")?;
    let mut reply = Vec::new();
    let mut buffer = [0u8; 4096];
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err("The aerial helper timed out".into());
        }
        stream
            .set_read_timeout(Some(remaining.min(Duration::from_millis(700))))
            .map_err(|e| e.to_string())?;
        match stream.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => {
                reply.extend_from_slice(&buffer[..n]);
                if reply.len() > 64 * 1024 {
                    return Err("Helper response too large".into());
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => return Err("The aerial helper timed out".into()),
        }
    }
    let split = reply
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .ok_or("Invalid helper HTTP response")?;
    let headers =
        std::str::from_utf8(&reply[..split]).map_err(|_| "Invalid helper HTTP headers")?;
    let status = headers
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|n| n.parse::<u16>().ok())
        .ok_or("Invalid helper HTTP status")?;
    if status != 200 && status != 429 {
        return Err(format!("Aerial helper returned HTTP {}", status));
    }
    let body = &reply[split + 4..];
    let decoded;
    let body = if headers.lines().any(|line| {
        line.to_ascii_lowercase().starts_with("transfer-encoding:")
            && line.to_ascii_lowercase().contains("chunked")
    }) {
        decoded = unchunk(body)?;
        &decoded[..]
    } else {
        body
    };
    let mut value: Value =
        serde_json::from_slice(body).map_err(|_| "Invalid aerial helper JSON")?;
    if value["state"] == "ready" {
        let url = value["url"].as_str().ok_or("Missing prepared aerial URL")?;
        let hash = url
            .strip_prefix("/media/")
            .and_then(|v| v.strip_suffix(".mp4"))
            .ok_or("Invalid prepared aerial path")?;
        if hash.len() != 64
            || !hash
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            || value["height"].as_u64().unwrap_or(0) < 1440
            || value["range"] != "SDR"
        {
            return Err("Unsupported prepared aerial".into());
        }
        let url = format!("{}{}", origin, url);
        value["url"] = Value::String(url);
    }
    Ok(value)
}

fn cache_artwork(id: &str, path: &str, directory: &Path) -> Result<Value, String> {
    if !valid_app_id(id) || !valid_artwork_path(path) {
        return Err("Unsupported artwork path".into());
    }
    let cache = directory.join("icons");
    fs::create_dir_all(&cache).map_err(|e| e.to_string())?;
    let hash = path.bytes().fold(0xcbf29ce484222325u64, |h, b| {
        (h ^ b as u64).wrapping_mul(0x100000001b3)
    });
    let extension = Path::new(path)
        .extension()
        .and_then(|x| x.to_str())
        .unwrap_or("png");
    let target = cache.join(format!("icon-{:016x}.{}", hash, extension));
    if target.is_file() {
        return Ok(json!({"id":id,"path":target}));
    }
    let direct = fs::canonicalize(path)
        .ok()
        .filter(|p| p.to_str().is_some_and(valid_artwork_path))
        .and_then(|p| {
            let metadata = fs::metadata(&p).ok()?;
            if !metadata.is_file() || metadata.len() > MAX_ARTWORK as u64 {
                return None;
            }
            fs::read(p).ok()
        });
    let bytes = if let Some(bytes) = direct {
        bytes
    } else {
        // Validate the root-visible canonical target too: an application icon
        // may be a symlink that was not visible from this native app's jail.
        let command=format!("p=$(readlink -f '{}') && case \"$p\" in /usr/palm/applications/*|/mnt/otncabi/usr/palm/applications/*|/media/developer/apps/usr/palm/applications/*|/media/cryptofs/apps/usr/palm/applications/*|/media/system/apps/usr/palm/applications/*) [ -f \"$p\" ] && [ $(wc -c < \"$p\") -le {} ] && base64 \"$p\" | tr -d '\\n' ;; *) exit 1 ;; esac",path,MAX_ARTWORK);
        let reply = luna(
            "org.webosbrew.hbchannel.service/exec",
            json!({"command":command}),
        )?;
        decode_base64(
            reply["stdoutString"]
                .as_str()
                .ok_or("Missing artwork response")?,
        )?
    };
    let png = bytes.starts_with(b"\x89PNG\r\n\x1a\n");
    let jpeg = bytes.starts_with(&[0xff, 0xd8, 0xff]);
    let webp = bytes.len() > 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP";
    if bytes.is_empty() || bytes.len() > MAX_ARTWORK || !(png || jpeg || webp) {
        return Err("Unsupported raster artwork".into());
    }
    let temp = cache.join("icon.pending");
    fs::write(&temp, &bytes).map_err(|e| e.to_string())?;
    fs::rename(temp, &target).map_err(|e| e.to_string())?;
    prune_artwork(&cache, &target);
    Ok(json!({"id":id,"path":target}))
}

fn prune_artwork(cache: &Path, current: &Path) {
    let mut files = fs::read_dir(cache)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            let name = path.file_name()?.to_str()?;
            if !name.starts_with("icon-") {
                return None;
            }
            let meta = entry.metadata().ok()?;
            if !meta.is_file() {
                return None;
            }
            Some((path, meta.modified().ok()?, meta.len()))
        })
        .collect::<Vec<_>>();
    files.sort_by_key(|f| f.1);
    let mut bytes = files.iter().map(|f| f.2).sum::<u64>();
    let mut count = files.len();
    for (path, _, size) in files {
        if count <= 32 && bytes <= 12 * 1024 * 1024 {
            break;
        }
        if path == current {
            continue;
        }
        if fs::remove_file(path).is_ok() {
            count -= 1;
            bytes = bytes.saturating_sub(size);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ids_and_artwork_reject_shell_syntax_and_unrelated_files() {
        assert!(valid_app_id("com.webos.app.hdmi1"));
        for id in ["../settings", "x'; reboot", "a\nb", "..", "a/b"] {
            assert!(!valid_app_id(id));
        }
        assert!(valid_artwork_path(
            "/media/developer/apps/usr/palm/applications/com.netflix.app/icon.png"
        ));
        for path in [
            "/etc/passwd",
            "/usr/palm/applications/a/../../etc/password.png",
            "/usr/palm/applications/a/icon';x.png",
            "https://example.com/icon.png",
            "/usr/palm/applications/a/icon.svg",
        ] {
            assert!(!valid_artwork_path(path));
        }
    }
    #[test]
    fn app_info_prefers_large_artwork_and_returns_the_validated_brand_color() {
        let reply = json!({"appInfo": {
            "folderPath":"/usr/palm/applications/com.video",
            "extraLargeIcon":"wide.png", "largeIcon":"large.png",
            "iconColor":"#F50124"
        }});
        assert_eq!(
            artwork_metadata(&reply, "/usr/palm/applications/com.video/icon.png"),
            Ok(ArtworkMetadata {
                path: "/usr/palm/applications/com.video/wide.png".into(),
                color: Some("#F50124".into())
            })
        );
        let reply = json!({"appInfo": {
            "folderPath":"/usr/palm/applications/com.video",
            "extraLargeIcon":"https://untrusted.example/icon.png",
            "largeIcon":"file:///usr/palm/applications/com.video/large.png"
        }});
        assert_eq!(
            artwork_metadata(&reply, "/usr/palm/applications/com.video/icon.png")
                .unwrap()
                .path,
            "/usr/palm/applications/com.video/large.png"
        );
    }
    #[test]
    fn app_info_cannot_redirect_root_reads_outside_application_artwork() {
        let fallback = "/media/cryptofs/apps/usr/palm/applications/com.video/icon.png";
        for reply in [
            Value::Null,
            json!({"appInfo":{"folderPath":"/tmp","extraLargeIcon":"private.png","iconColor":"#fff"}}),
            json!({"appInfo":{"folderPath":"/usr/palm/applications/com.video","extraLargeIcon":"../../private.png","largeIcon":"icon';command.png","iconColor":"#112233;red"}}),
        ] {
            assert_eq!(
                artwork_metadata(&reply, fallback),
                Ok(ArtworkMetadata {
                    path: fallback.into(),
                    color: None
                })
            );
        }
        assert!(artwork_metadata(
            &json!({"appInfo":{"extraLargeIcon":"/etc/private.png"}}),
            "/tmp/private.png"
        )
        .is_err());
    }
    #[test]
    fn inventory_keeps_valid_metadata_and_separates_explicit_inputs() {
        let apps = inventory(
            json!({"launchPoints":[{"id":"com.app.video","title":"Video","folderPath":"/usr/palm/applications/com.app.video","icon":"icon.png"},{"id":APP_ID,"title":"Self"},{"id":"bad/id","title":"Bad"},{"id":"com.hidden","title":"Hidden","visible":false}]}),
        );
        assert_eq!(apps["apps"].as_array().unwrap().len(), 1);
        assert_eq!(apps["apps"][0]["input"], false);
        let sources = inputs(
            json!({"deviceList":[{"appId":"com.webos.app.hdmi1","label":"Console"},{"appId":"com.webos.app.hdmi99","label":"Bad"}]}),
        );
        assert_eq!(sources["apps"].as_array().unwrap().len(), 1);
        assert_eq!(sources["apps"][0]["title"], "Console");
    }
    #[test]
    fn base64_decoding_is_bounded_and_rejects_bad_bytes() {
        assert_eq!(decode_base64("aGVsbG8=").unwrap(), b"hello");
        assert_eq!(decode_base64("TQ==\n").unwrap(), b"M");
        assert!(decode_base64("T$==").is_err());
        assert!(decode_base64("T").is_err());
        assert!(decode_base64("TQ==AAAA").is_err());
    }
    #[test]
    fn preferences_are_atomic_isolated_and_bounded() {
        let dir = std::env::temp_dir().join(format!("aerial-native-prefs-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let original = dir.join("web-preferences.json");
        fs::write(&original, b"unchanged").unwrap();
        save_preferences(&dir, &json!({"clockStyle":"glass"})).unwrap();
        assert_eq!(load_preferences(&dir).unwrap()["clockStyle"], "glass");
        assert_eq!(fs::read(&original).unwrap(), b"unchanged");
        assert!(!dir.join("preferences.json.pending").exists());
        assert!(save_preferences(&dir, &json!({"tooLarge":"x".repeat(MAX_PREFS)})).is_err());
        assert!(save_preferences(&dir, &json!([])).is_err());
        fs::remove_file(dir.join("preferences.json")).unwrap();
        fs::remove_file(original).unwrap();
        fs::remove_dir(dir).unwrap();
    }
    #[test]
    fn invalid_launch_is_rejected_before_calling_any_service() {
        assert!(perform(&Request::Launch("bad';shutdown".into()), Path::new(".")).is_err());
        assert!(perform(&Request::SleepTimer(65535), Path::new(".")).is_err());
    }
    #[test]
    fn helpers_reject_public_hosts_credentials_paths_and_bad_chunk_sizes() {
        assert!(helper_address("http://127.0.0.1:8794").is_ok());
        assert!(helper_address("http://localhost:8794").is_ok());
        for url in [
            "https://localhost:8794",
            "http://example.com:8794",
            "http://8.8.8.8:8794",
            "http://user@localhost:8794",
            "http://localhost:8794/path",
            "http://localhost:0",
        ] {
            assert!(helper_address(url).is_err());
        }
        assert_eq!(unchunk(b"5\r\nhello\r\n0\r\n\r\n").unwrap(), b"hello");
        assert!(unchunk(b"100000000\r\nx\r\n0\r\n\r\n").is_err());
    }
    #[test]
    fn metadata_cache_is_separate_from_preferences_and_revalidated() {
        let dir =
            std::env::temp_dir().join(format!("aerial-native-app-cache-{}", std::process::id()));
        save_apps(&dir,&json!({"apps":[{"id":"com.video","title":"Video","icon":"/usr/palm/applications/com.video/icon.png","color":"#123456"},{"id":"bad/id","title":"Bad"}]})).unwrap();
        let loaded = load_preferences(&dir).unwrap();
        assert_eq!(loaded["cachedApps"].as_array().unwrap().len(), 1);
        assert_eq!(loaded["cachedApps"][0]["id"], "com.video");
        save_preferences(&dir, &loaded).unwrap();
        assert!(dir.join("apps.json").exists());
        assert!(
            serde_json::from_slice::<Value>(&fs::read(dir.join("preferences.json")).unwrap())
                .unwrap()
                .get("cachedApps")
                .is_none()
        );
        fs::remove_file(dir.join("apps.json")).unwrap();
        fs::remove_file(dir.join("preferences.json")).unwrap();
        fs::remove_dir(dir).unwrap();
    }
    #[cfg(target_os = "linux")]
    #[test]
    fn service_reply_does_not_wait_for_inherited_stdout_eof() {
        let start = Instant::now();
        let result = run_bounded(
            Path::new("/bin/sh"),
            &[
                "-c".into(),
                "printf '{\"returnValue\":true}'; sleep 2 & exit 0".into(),
            ],
            Duration::from_millis(250),
            1024,
        )
        .unwrap();
        assert_eq!(
            serde_json::from_slice::<Value>(&result).unwrap()["returnValue"],
            true
        );
        assert!(start.elapsed() < Duration::from_millis(700));
        let start = Instant::now();
        assert!(run_bounded(
            Path::new("/bin/sh"),
            &["-c".into(), "sleep 2 & exit 0".into()],
            Duration::from_millis(100),
            1024
        )
        .is_err());
        assert!(start.elapsed() < Duration::from_millis(700));
    }
    #[cfg(target_os = "linux")]
    #[test]
    fn complete_json_returns_while_wrapper_still_waits_for_stdin() {
        let start = Instant::now();
        let result = run_bounded(
            Path::new("/bin/sh"),
            &["-c".into(), "printf '{\"ok\":true}'; read line".into()],
            Duration::from_millis(250),
            1024,
        )
        .unwrap();
        assert_eq!(
            serde_json::from_slice::<Value>(&result).unwrap()["ok"],
            true
        );
        assert!(start.elapsed() < Duration::from_millis(700));
    }
    #[cfg(unix)]
    #[test]
    fn persistent_storage_is_app_owned_and_rejects_symlink_destinations() {
        use std::os::unix::fs::{symlink, MetadataExt, PermissionsExt};
        let fixture =
            std::env::temp_dir().join(format!("aerial-native-storage-{}", std::process::id()));
        fs::create_dir_all(&fixture).unwrap();
        let user = fs::metadata(&fixture).unwrap().uid();
        let data = fixture.join("owned");
        let fake = fixture.join("symlink");
        let selected = owned_storage_directory(&[data.clone()], user, true).unwrap();
        assert_eq!(selected, data);
        assert_eq!(
            fs::metadata(&data).unwrap().permissions().mode() & 0o777,
            0o700
        );
        assert_eq!(fs::read_dir(&data).unwrap().count(), 0);
        symlink(&data, &fake).unwrap();
        assert!(owned_storage_directory(&[fake.clone()], user, true).is_err());
        assert!(owned_storage_directory(&[data.clone()], user + 1, false).is_err());
        assert_eq!(
            owned_storage_directory(&[fixture.join("must-not-create"), data.clone()], 0, false)
                .unwrap(),
            data
        );
        fs::remove_file(fake).unwrap();
        fs::remove_dir(data).unwrap();
        fs::remove_dir(fixture).unwrap();
    }
    #[test]
    fn installation_manifest_must_match_this_native_app() {
        let fixture =
            std::env::temp_dir().join(format!("aerial-native-install-{}", std::process::id()));
        fs::create_dir_all(&fixture).unwrap();
        let file = fixture.join("appinfo.json");
        fs::write(
            &file,
            json!({"id":"unrelated.app","type":"native"}).to_string(),
        )
        .unwrap();
        assert!(!valid_install_directory(&fixture));
        fs::write(&file, json!({"id":APP_ID,"type":"native"}).to_string()).unwrap();
        assert!(valid_install_directory(&fixture));
        fs::remove_file(file).unwrap();
        fs::remove_dir(fixture).unwrap();
    }
    #[test]
    fn cache_refresh_replaces_its_own_group_and_warm_inputs_survive_inventory() {
        let dir =
            std::env::temp_dir().join(format!("aerial-native-group-cache-{}", std::process::id()));
        update_cached_group(
            &dir,
            &json!({"apps":[{"id":"com.old.video","title":"Old video","input":false}]}),
            false,
        )
        .unwrap();
        update_cached_group(
            &dir,
            &json!({"apps":[{"id":"com.webos.app.hdmi1","title":"Old console","input":true}]}),
            true,
        )
        .unwrap();
        update_cached_group(
            &dir,
            &json!({"apps":[{"id":"com.new.video","title":"New video","input":false}]}),
            false,
        )
        .unwrap();
        let warm = load_preferences(&dir).unwrap();
        let apps = warm["cachedApps"].as_array().unwrap();
        assert_eq!(apps.len(), 2);
        assert!(!apps.iter().any(|a| a["id"] == "com.old.video"));
        assert!(apps.iter().any(|a| a["id"] == "com.new.video"));
        assert!(apps
            .iter()
            .any(|a| a["id"] == "com.webos.app.hdmi1" && a["input"] == true));
        update_cached_group(
            &dir,
            &json!({"apps":[{"id":"com.webos.app.hdmi2","title":"New console","input":true}]}),
            true,
        )
        .unwrap();
        let warm = load_apps(&dir).unwrap();
        let apps = warm.as_array().unwrap();
        assert_eq!(apps.len(), 2);
        assert!(apps.iter().any(|a| a["id"] == "com.new.video"));
        assert!(!apps.iter().any(|a| a["id"] == "com.webos.app.hdmi1"));
        assert!(apps
            .iter()
            .any(|a| a["id"] == "com.webos.app.hdmi2" && a["title"] == "New console"));
        update_cached_group(&dir, &json!({"apps":[]}), true).unwrap();
        let warm = load_apps(&dir).unwrap();
        assert_eq!(warm.as_array().unwrap().len(), 1);
        assert_eq!(warm[0]["id"], "com.new.video");
        let mut raw: Value =
            serde_json::from_slice(&fs::read(dir.join("apps.json")).unwrap()).unwrap();
        raw["updatedAt"] = json!(0);
        fs::write(dir.join("apps.json"), raw.to_string()).unwrap();
        assert!(load_apps(&dir).is_none());
        fs::remove_file(dir.join("apps.json")).unwrap();
        fs::remove_dir(dir).unwrap();
    }
}
