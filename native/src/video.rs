//! LG hardware media backend. The C wrapper allocates its object inside LG's
//! library, avoiding guessed C++ layouts. All blocking calls run off the UI
//! thread. Playing is reported only after uMS confirms play + a VDEC resource.
use serde_json::{json, Value};
#[cfg(all(feature = "sdl", target_os = "linux", target_arch = "arm"))]
use std::sync::mpsc;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    mpsc::{Receiver, SyncSender},
    Arc,
};

#[derive(Clone, Debug, PartialEq)]
pub enum VideoEvent {
    Loading,
    Ready,
    Playing,
    Paused,
    Stopped,
    Ended,
    Error(String),
}

enum Command {
    Load(u64, String),
    Window(String),
    Play,
    Pause,
    Stop,
    Visible(bool),
    Exit,
}
pub struct Video {
    commands: SyncSender<Command>,
    events: Receiver<VideoEvent>,
    generation: Arc<AtomicU64>,
    exiting: Arc<AtomicBool>,
}
impl Video {
    pub fn new(app_id: &str) -> Result<Self, String> {
        if app_id.is_empty()
            || !app_id
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c))
        {
            return Err("Invalid media app ID".into());
        }
        #[cfg(all(feature = "sdl", target_os = "linux", target_arch = "arm"))]
        {
            let api = unsafe { backend::Api::open()? };
            let (commands, receive) = mpsc::sync_channel(8);
            let (send, events) = mpsc::sync_channel(16);
            let generation = Arc::new(AtomicU64::new(0));
            let exiting = Arc::new(AtomicBool::new(false));
            let (worker_generation, worker_exiting, app) =
                (generation.clone(), exiting.clone(), app_id.to_owned());
            std::thread::Builder::new()
                .name("aerial-hardware-video".into())
                .spawn(move || {
                    backend::run(api, &app, receive, send, worker_generation, worker_exiting)
                })
                .map_err(|e| e.to_string())?;
            Ok(Self {
                commands,
                events,
                generation,
                exiting,
            })
        }
        #[cfg(not(all(feature = "sdl", target_os = "linux", target_arch = "arm")))]
        {
            Err("LG hardware video is only available in the ARM webOS build".into())
        }
    }
    pub fn load(&self, url: &str) -> Result<(), String> {
        validate_url(url)?;
        let generation = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        self.send(Command::Load(generation, url.into()))
    }
    pub fn play(&self) -> Result<(), String> {
        self.send(Command::Play)
    }
    pub fn pause(&self) -> Result<(), String> {
        self.send(Command::Pause)
    }
    pub fn stop(&self) -> Result<(), String> {
        self.generation.fetch_add(1, Ordering::SeqCst);
        self.send(Command::Stop)
    }
    pub fn set_visible(&self, visible: bool) -> Result<(), String> {
        self.send(Command::Visible(visible))
    }
    /// Supply the ID created/placed by SDL_webOSCreateExportedWindow on the UI
    /// thread. LG's native media process imports this compositor-owned window.
    pub fn set_window_id(&self, id: &str) -> Result<(), String> {
        if id.is_empty()
            || id.len() > 256
            || !id
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c))
        {
            return Err("Invalid native video window ID".into());
        }
        self.send(Command::Window(id.into()))
    }
    pub fn poll(&self) -> Vec<VideoEvent> {
        self.events.try_iter().collect()
    }
    fn send(&self, command: Command) -> Result<(), String> {
        self.commands
            .try_send(command)
            .map_err(|_| "Video command queue is busy".into())
    }
}
impl Drop for Video {
    fn drop(&mut self) {
        self.exiting.store(true, Ordering::SeqCst);
        let _ = self.commands.try_send(Command::Exit);
    }
}

fn validate_url(url: &str) -> Result<(), String> {
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .ok_or("Aerial media must use HTTP or HTTPS")?;
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    if url.len() > 4096
        || authority.is_empty()
        || authority.contains('@')
        || url
            .chars()
            .any(|c| c.is_control() || c.is_whitespace() || c == '\\')
    {
        return Err("Invalid aerial URL".into());
    }
    Ok(())
}

#[derive(Debug, PartialEq)]
enum PipelineState {
    Missing,
    Loading,
    Playing,
    Paused,
    Ended,
}
fn own_pipeline<'a>(value: &'a Value, app_id: &str, url: &str) -> Option<&'a Value> {
    value.as_array()?.iter().find(|p| {
        p.get("uri").and_then(Value::as_str) == Some(url)
            && (p.get("appId").and_then(Value::as_str) == Some(app_id)
                || p.get("currentAppId").and_then(Value::as_str) == Some(app_id))
    })
}
fn pipeline_state(value: &Value, app_id: &str, url: &str) -> PipelineState {
    let Some(p) = own_pipeline(value, app_id, url) else {
        return PipelineState::Missing;
    };
    let state = p.get("mediaState").and_then(Value::as_str).unwrap_or("");
    let hardware = p
        .get("resource")
        .and_then(Value::as_array)
        .map(|r| {
            r.iter()
                .any(|r| r.get("resource").and_then(Value::as_str) == Some("VDEC"))
        })
        .unwrap_or(false);
    match state {
        "play" | "playing" if hardware => PipelineState::Playing,
        "pause" | "paused" => PipelineState::Paused,
        "eos" | "endOfStream" | "ended" | "stop" | "stopped" => PipelineState::Ended,
        _ => PipelineState::Loading,
    }
}

#[derive(Debug, PartialEq)]
struct PlaybackProgress {
    position_ms: Option<u64>,
    ended: bool,
}
fn playback_progress(reply: &Value) -> Result<PlaybackProgress, String> {
    // LG returns a JSON document inside the data string, while OSE variants
    // may expose the document directly. The snapshot includes retained EOS
    // events even when getActivePipelines still labels the pipeline "play".
    let data = match reply.get("data") {
        Some(Value::String(text)) => {
            serde_json::from_str(text).map_err(|_| "Invalid media progress snapshot")?
        }
        Some(value) => value.clone(),
        None => reply.clone(),
    };
    let position_ms = data["currentTime"]["currentTime"]
        .as_u64()
        .or_else(|| data["currentTime"].as_u64());
    let duration_ms = data["sourceInfo"]["programInfo"][0]["duration"].as_u64();
    let eos = data
        .get("endOfStream")
        .map(|event| event.is_object() || event.as_bool() == Some(true))
        .unwrap_or(false);
    let reached_end = match (position_ms, duration_ms) {
        (Some(position), Some(duration)) if duration > 0 => position >= duration,
        _ => false,
    };
    Ok(PlaybackProgress {
        position_ms,
        ended: eos || reached_end,
    })
}

#[cfg(all(feature = "sdl", target_os = "linux", target_arch = "arm"))]
mod backend {
    use super::*;
    use std::{
        ffi::{c_char, c_int, c_void, CStr, CString},
        time::{Duration, Instant},
    };
    #[link(name = "dl")]
    extern "C" {
        fn dlopen(name: *const c_char, flags: c_int) -> *mut c_void;
        fn dlsym(handle: *mut c_void, name: *const c_char) -> *mut c_void;
        fn dlclose(handle: *mut c_void) -> c_int;
        fn dlerror() -> *const c_char;
    }
    type Create = unsafe extern "C" fn() -> *mut c_void;
    type Action = unsafe extern "C" fn(*mut c_void) -> c_int;
    type Foreground = unsafe extern "C" fn(*mut c_void) -> bool;
    type Load = unsafe extern "C" fn(*mut c_void, *mut c_char, c_int, *mut c_char) -> c_int;
    pub(super) struct Api {
        c: *mut c_void,
        cpp: *mut c_void,
        create: Create,
        load: Load,
        play: Action,
        pause: Action,
        unload: Action,
        destroy: Action,
        foreground: Foreground,
        background: Foreground,
    }
    // Library handles and function pointers are moved once, before constructing
    // the player, and are then used exclusively by the dedicated media thread.
    unsafe impl Send for Api {}
    impl Api {
        pub unsafe fn open() -> Result<Self, String> {
            unsafe fn library(name: &[u8]) -> Result<*mut c_void, String> {
                let p = dlopen(name.as_ptr().cast(), 1);
                if p.is_null() {
                    let e = dlerror();
                    Err(if e.is_null() {
                        "LG media library unavailable".into()
                    } else {
                        CStr::from_ptr(e).to_string_lossy().into_owned()
                    })
                } else {
                    Ok(p)
                }
            }
            unsafe fn symbol<T: Copy>(handle: *mut c_void, name: &[u8]) -> Result<T, String> {
                let p = dlsym(handle, name.as_ptr().cast());
                if p.is_null() {
                    return Err(format!(
                        "Missing LG media API: {}",
                        String::from_utf8_lossy(&name[..name.len() - 1])
                    ));
                }
                Ok(std::mem::transmute_copy(&p))
            }
            let c = library(b"libumedia_api_c.so.1\0")?;
            let cpp = match library(b"libumedia_api.so.1\0") {
                Ok(p) => p,
                Err(e) => {
                    dlclose(c);
                    return Err(e);
                }
            };
            let result = (|| {
                Ok(Self {
                    c,
                    cpp,
                    create: symbol(c, b"uMediaClientCreatePrivate\0")?,
                    load: symbol(c, b"uMediaClientLoad\0")?,
                    play: symbol(c, b"uMediaClientPlay\0")?,
                    pause: symbol(c, b"uMediaClientPause\0")?,
                    unload: symbol(c, b"uMediaClientUnload\0")?,
                    destroy: symbol(c, b"uMediaClientDestroy\0")?,
                    foreground: symbol(
                        cpp,
                        b"_ZN12uMediaServer12uMediaClient16notifyForegroundEv\0",
                    )?,
                    background: symbol(
                        cpp,
                        b"_ZN12uMediaServer12uMediaClient16notifyBackgroundEv\0",
                    )?,
                })
            })();
            if result.is_err() {
                dlclose(c);
                dlclose(cpp);
            }
            result
        }
    }
    impl Drop for Api {
        fn drop(&mut self) {
            unsafe {
                dlclose(self.c);
                dlclose(self.cpp);
            }
        }
    }

    fn request(method: &str, payload: Value) -> Result<Value, String> {
        crate::platform::call_native(method, payload)
    }
    fn status(homebrew: &mut bool) -> Result<Value, String> {
        if !*homebrew {
            if let Ok(reply) = request("com.webos.media/getActivePipelines", json!({})) {
                if reply.is_array() {
                    return Ok(reply);
                }
            }
            *homebrew = true;
        }
        // Fixed command only: no URLs, titles or preferences enter a shell.
        let reply = request(
            "org.webosbrew.hbchannel.service/exec",
            json!({"command":"luna-send -n 1 -w 4000 luna://com.webos.media/getActivePipelines '{}'"}),
        )?;
        let value: Value = serde_json::from_str(
            reply["stdoutString"]
                .as_str()
                .ok_or("Missing Homebrew media status")?,
        )
        .map_err(|_| "Invalid Homebrew media status")?;
        if value.is_array() {
            Ok(value)
        } else {
            Err("Homebrew media status was not a pipeline list".into())
        }
    }
    fn progress(pipeline: &Value, homebrew: &mut bool) -> Result<PlaybackProgress, String> {
        let id = pipeline["id"].as_str().ok_or("No native media ID")?;
        // The fallback command accepts only an opaque ID issued by uMS. Never
        // interpolate a source URL, title, application label or preference.
        if id.is_empty()
            || id.len() > 256
            || !id
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c))
        {
            return Err("Invalid native media ID".into());
        }
        if !*homebrew {
            if let Ok(reply) = request("com.webos.media/getPipelineState", json!({"mediaId":id})) {
                return playback_progress(&reply);
            }
            *homebrew = true;
        }
        let command = format!(
            "luna-send -n 1 -w 4000 luna://com.webos.media/getPipelineState '{{\"mediaId\":\"{id}\"}}'"
        );
        let reply = request(
            "org.webosbrew.hbchannel.service/exec",
            json!({"command":command}),
        )?;
        let snapshot: Value = serde_json::from_str(
            reply["stdoutString"]
                .as_str()
                .ok_or("Missing media progress snapshot")?,
        )
        .map_err(|_| "Invalid Homebrew media progress")?;
        playback_progress(&snapshot)
    }
    pub(super) fn run(
        api: Api,
        app: &str,
        receive: Receiver<Command>,
        events: SyncSender<VideoEvent>,
        generation: Arc<AtomicU64>,
        exiting: Arc<AtomicBool>,
    ) {
        let emit = |event| {
            let _ = events.try_send(event);
        };
        let player = unsafe { (api.create)() };
        if player.is_null() {
            emit(VideoEvent::Error(
                "LG could not create a hardware media player".into(),
            ));
            return;
        }
        let mut visible = true;
        let mut loaded = false;
        let mut wanted: Option<(u64, String)> = None;
        let mut window_id: Option<String> = None;
        let mut last_state = PipelineState::Missing;
        let mut misses = 0;
        let mut loaded_at = Instant::now();
        let mut ever_played = false;
        let mut homebrew_status = false;
        let mut homebrew_progress = false;
        let mut last_position = None;
        let mut progressed_at = Instant::now();
        let mut play_requested = true;
        let mut progress_warning = false;
        let mut status_warning = false;
        while !exiting.load(Ordering::SeqCst) {
            if wanted
                .as_ref()
                .map(|(g, _)| *g != generation.load(Ordering::SeqCst))
                .unwrap_or(false)
            {
                if loaded {
                    unsafe {
                        (api.unload)(player);
                    }
                }
                loaded = false;
                wanted = None;
                last_state = PipelineState::Missing;
            }
            match receive.recv_timeout(Duration::from_secs(2)) {
                Ok(Command::Exit) => break,
                Ok(Command::Load(g, url)) => {
                    if g != generation.load(Ordering::SeqCst) {
                        continue;
                    }
                    if loaded {
                        unsafe {
                            (api.unload)(player);
                        }
                    }
                    loaded = false;
                    last_state = PipelineState::Missing;
                    misses = 0;
                    ever_played = false;
                    last_position = None;
                    progressed_at = Instant::now();
                    play_requested = true;
                    progress_warning = false;
                    status_warning = false;
                    wanted = Some((g, url));
                }
                Ok(Command::Window(id)) => {
                    window_id = Some(id);
                }
                Ok(Command::Visible(value)) => {
                    visible = value;
                    if loaded {
                        loaded_at = Instant::now();
                        progressed_at = Instant::now();
                        unsafe {
                            if visible {
                                play_requested = true;
                                (api.foreground)(player);
                                (api.play)(player);
                            } else {
                                (api.pause)(player);
                                (api.background)(player);
                            }
                        }
                    }
                }
                Ok(Command::Play) => {
                    play_requested = true;
                    progressed_at = Instant::now();
                    if loaded && visible {
                        unsafe {
                            (api.foreground)(player);
                            (api.play)(player);
                        }
                    }
                }
                Ok(Command::Pause) => {
                    play_requested = false;
                    if loaded {
                        unsafe {
                            (api.pause)(player);
                        }
                    }
                }
                Ok(Command::Stop) => {
                    if loaded {
                        unsafe {
                            (api.unload)(player);
                        }
                    }
                    loaded = false;
                    wanted = None;
                    last_state = PipelineState::Missing;
                    emit(VideoEvent::Stopped);
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
                Err(mpsc::RecvTimeoutError::Timeout) => {}
            }
            if !visible {
                continue;
            }
            if !loaded {
                if let Some((g, url)) = &wanted {
                    if *g != generation.load(Ordering::SeqCst) {
                        continue;
                    }
                    let Some(window) = &window_id else {
                        wanted = None;
                        emit(VideoEvent::Error(
                            "No exported native video window is available".into(),
                        ));
                        continue;
                    };
                    emit(VideoEvent::Loading);
                    let uri = CString::new(url.as_str()).unwrap();
                    // No transport override: LG infers URI transport itself.
                    let payload=CString::new(json!({"option":{"appId":app,"windowId":window,"needAudio":false,"preload":"auto"}}).to_string()).unwrap();
                    let accepted = unsafe {
                        (api.load)(
                            player,
                            uri.as_ptr() as *mut c_char,
                            1,
                            payload.as_ptr() as *mut c_char,
                        )
                    };
                    if *g != generation.load(Ordering::SeqCst) {
                        unsafe {
                            (api.unload)(player);
                        }
                        wanted = None;
                        continue;
                    }
                    if accepted == 0 {
                        wanted = None;
                        emit(VideoEvent::Error("LG hardware media load failed".into()));
                        continue;
                    }
                    loaded = true;
                    loaded_at = Instant::now();
                    progressed_at = Instant::now();
                    emit(VideoEvent::Ready);
                    unsafe {
                        (api.foreground)(player);
                    }
                    unsafe {
                        (api.play)(player);
                    }
                }
            }
            if loaded {
                let reply = status(&mut homebrew_status);
                if let Err(e) = &reply {
                    if !status_warning {
                        eprintln!("hardware-video: status unavailable: {e}");
                        status_warning = true;
                    }
                }
                let mut ended = false;
                if let (Some((_, url)), Ok(reply)) = (&wanted, reply) {
                    status_warning = false;
                    let state = pipeline_state(&reply, app, url);
                    if state == PipelineState::Missing && last_state == PipelineState::Playing {
                        misses += 1;
                        if misses >= 3 {
                            ended = true;
                        }
                    } else {
                        misses = 0;
                        if state != last_state {
                            match state {
                                PipelineState::Playing => {
                                    if !ever_played {
                                        progressed_at = Instant::now();
                                    }
                                    ever_played = true;
                                    emit(VideoEvent::Playing);
                                }
                                PipelineState::Paused => emit(VideoEvent::Paused),
                                PipelineState::Ended => {
                                    ended = true;
                                }
                                _ => {}
                            }
                            last_state = state;
                        }
                    }
                    if ever_played && !ended {
                        if let Some(pipeline) = own_pipeline(&reply, app, url) {
                            match progress(pipeline, &mut homebrew_progress) {
                                Ok(progress) => {
                                    progress_warning = false;
                                    if let Some(position) = progress.position_ms {
                                        if last_position != Some(position) {
                                            last_position = Some(position);
                                            progressed_at = Instant::now();
                                        }
                                    }
                                    ended = progress.ended;
                                }
                                Err(e) => {
                                    if !progress_warning {
                                        eprintln!("hardware-video: progress unavailable: {e}");
                                        progress_warning = true;
                                    }
                                }
                            }
                        }
                    }
                }
                let stalled = ever_played
                    && play_requested
                    && progressed_at.elapsed() > Duration::from_secs(30);
                let unconfirmed = !ever_played && loaded_at.elapsed() > Duration::from_secs(25);
                if ended || stalled || unconfirmed {
                    unsafe {
                        (api.unload)(player);
                    }
                    loaded = false;
                    wanted = None;
                    last_state = PipelineState::Missing;
                    if ended {
                        emit(VideoEvent::Ended);
                    } else {
                        emit(VideoEvent::Error(if stalled {
                            "Hardware aerial playback stopped making progress".into()
                        } else {
                            "The TV did not confirm hardware video playback".into()
                        }));
                    }
                }
            }
        }
        unsafe {
            (api.background)(player);
            if loaded {
                (api.unload)(player);
            }
            (api.destroy)(player);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn an_accepted_request_without_own_hardware_pipeline_is_not_playback() {
        let url = "https://sylvan.apple.com/a.mov";
        assert_eq!(
            pipeline_state(&json!([]), "test.app", url),
            PipelineState::Missing
        );
        let mut value = json!([{"appId":"other.app","uri":url,"mediaState":"play","resource":[{"resource":"VDEC"}]}]);
        assert_eq!(
            pipeline_state(&value, "test.app", url),
            PipelineState::Missing
        );
        value[0]["appId"] = json!("test.app");
        value[0]["resource"] = json!([]);
        assert_eq!(
            pipeline_state(&value, "test.app", url),
            PipelineState::Loading
        );
        value[0]["resource"] = json!([{"resource":"VDEC"}]);
        assert_eq!(
            pipeline_state(&value, "test.app", url),
            PipelineState::Playing
        );
    }
    #[test]
    fn uri_transport_rejects_credentials_controls_and_non_network_sources() {
        assert!(validate_url("https://sylvan.apple.com/movie.mov").is_ok());
        for bad in [
            "file:///etc/passwd",
            "https://user:password@example.com/a",
            "http://",
            "https://example.com/a\n",
            "https://example.com\\a",
        ] {
            assert!(validate_url(bad).is_err(), "{bad}");
        }
    }
    #[test]
    fn retained_eos_is_completion_even_when_active_pipeline_still_says_play() {
        let snapshot = json!({"data":json!({
            "media_state":"play",
            "currentTime":{"currentTime":501980},
            "sourceInfo":{"programInfo":[{"duration":502368}]},
            "endOfStream":{"mediaId":"_test"}
        }).to_string()});
        assert_eq!(
            playback_progress(&snapshot),
            Ok(PlaybackProgress {
                position_ms: Some(501980),
                ended: true
            })
        );
    }
    #[test]
    fn media_progress_distinguishes_start_and_completed_duration() {
        let mut snapshot = json!({"data":{
            "currentTime":{"currentTime":0},
            "sourceInfo":{"programInfo":[{"duration":502368}]}
        }});
        assert_eq!(
            playback_progress(&snapshot),
            Ok(PlaybackProgress {
                position_ms: Some(0),
                ended: false
            })
        );
        snapshot["data"]["currentTime"]["currentTime"] = json!(502368);
        assert!(playback_progress(&snapshot).unwrap().ended);
        assert!(playback_progress(&json!({"data":"truncated {"})).is_err());
    }
}
