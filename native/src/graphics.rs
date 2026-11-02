//! Cached native textures and text. All SDL work stays on the event thread.
use aerial_native_probe::text_cache::{clean_text, TextCache};
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    ffi::{c_char, c_int, c_void, CStr, CString},
    path::PathBuf,
    ptr,
};
#[repr(C)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}
#[repr(C)]
#[derive(Clone, Copy)]
struct Color {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}
#[repr(C)]
struct Surface {
    flags: u32,
    format: *mut c_void,
    w: i32,
    h: i32,
    pitch: i32,
    pixels: *mut u8,
}
#[repr(C)]
pub union Event {
    pub kind: u32,
    pub bytes: [u8; 56],
    pub alignment: u64,
}
#[repr(C)]
struct Info {
    name: *const c_char,
    flags: u32,
    n: u32,
    formats: [u32; 16],
    w: i32,
    h: i32,
}
#[link(name = "SDL2")]
extern "C" {
    fn SDL_Init(flags: u32) -> c_int;
    fn SDL_WasInit(flags: u32) -> u32;
    fn SDL_Quit();
    fn SDL_GetError() -> *const c_char;
    fn SDL_SetHint(name: *const c_char, value: *const c_char) -> i32;
    fn SDL_GL_SetAttribute(attr: i32, value: i32) -> i32;
    fn SDL_CreateWindow(t: *const c_char, x: i32, y: i32, w: i32, h: i32, f: u32) -> *mut c_void;
    fn SDL_DestroyWindow(w: *mut c_void);
    fn SDL_CreateRenderer(w: *mut c_void, i: i32, f: u32) -> *mut c_void;
    fn SDL_CreateSoftwareRenderer(surface: *mut c_void) -> *mut c_void;
    fn SDL_CreateRGBSurfaceWithFormat(
        flags: u32,
        w: i32,
        h: i32,
        depth: i32,
        format: u32,
    ) -> *mut c_void;
    fn SDL_DestroyRenderer(r: *mut c_void);
    fn SDL_GetRendererInfo(r: *mut c_void, i: *mut Info) -> c_int;
    fn SDL_RenderSetLogicalSize(r: *mut c_void, w: i32, h: i32) -> c_int;
    fn SDL_SetRenderDrawColor(r: *mut c_void, a: u8, b: u8, c: u8, d: u8) -> c_int;
    fn SDL_SetRenderDrawBlendMode(r: *mut c_void, m: i32) -> c_int;
    fn SDL_RenderClear(r: *mut c_void) -> c_int;
    fn SDL_RenderFillRect(r: *mut c_void, rect: *const Rect) -> c_int;
    fn SDL_RenderDrawLine(r: *mut c_void, x1: i32, y1: i32, x2: i32, y2: i32) -> c_int;
    fn SDL_RenderSetClipRect(r: *mut c_void, rect: *const Rect) -> c_int;
    fn SDL_RenderCopy(r: *mut c_void, t: *mut c_void, src: *const Rect, dst: *const Rect) -> c_int;
    fn SDL_RenderPresent(r: *mut c_void);
    fn SDL_CreateTextureFromSurface(r: *mut c_void, s: *mut c_void) -> *mut c_void;
    fn SDL_CreateRGBSurfaceWithFormatFrom(
        pixels: *mut c_void,
        w: i32,
        h: i32,
        depth: i32,
        pitch: i32,
        format: u32,
    ) -> *mut c_void;
    fn SDL_QueryTexture(
        t: *mut c_void,
        f: *mut u32,
        a: *mut i32,
        w: *mut i32,
        h: *mut i32,
    ) -> c_int;
    fn SDL_SetTextureAlphaMod(t: *mut c_void, a: u8) -> c_int;
    fn SDL_SetTextureColorMod(t: *mut c_void, r: u8, g: u8, b: u8) -> c_int;
    fn SDL_DestroyTexture(t: *mut c_void);
    fn SDL_FreeSurface(s: *mut c_void);
    fn SDL_ConvertSurfaceFormat(surface: *mut c_void, format: u32, flags: u32) -> *mut Surface;
    pub fn SDL_WaitEventTimeout(e: *mut Event, t: i32) -> i32;
    pub fn SDL_PollEvent(e: *mut Event) -> i32;
    pub fn SDL_GetTicks() -> u32;
    fn SDL_RenderReadPixels(
        r: *mut c_void,
        rect: *const Rect,
        format: u32,
        pixels: *mut c_void,
        pitch: i32,
    ) -> i32;
    fn SDL_webOSCreateExportedWindow(kind: i32) -> *const c_char;
    fn SDL_webOSSetExportedWindow(id: *const c_char, src: *mut Rect, dst: *mut Rect) -> i32;
    fn SDL_webOSDestroyExportedWindow(id: *const c_char);
}
#[link(name = "SDL2_ttf")]
extern "C" {
    fn TTF_Init() -> i32;
    fn TTF_OpenFont(p: *const c_char, s: i32) -> *mut c_void;
    fn TTF_CloseFont(f: *mut c_void);
    fn TTF_SizeUTF8(f: *mut c_void, text: *const c_char, width: *mut i32, height: *mut i32) -> i32;
    fn TTF_RenderUTF8_Blended(f: *mut c_void, t: *const c_char, c: Color) -> *mut c_void;
    fn TTF_Quit();
}
#[link(name = "SDL2_image")]
extern "C" {
    fn IMG_LoadTexture(r: *mut c_void, p: *const c_char) -> *mut c_void;
    fn IMG_Load(p: *const c_char) -> *mut c_void;
}
struct Texture {
    ptr: *mut c_void,
    w: i32,
    h: i32,
}
#[derive(Hash, PartialEq, Eq)]
struct ShapeKey {
    w: i32,
    h: i32,
    r: i32,
    stroke: i32,
    top: [u8; 4],
    bottom: [u8; 4],
}
pub struct Graphics {
    window: *mut c_void,
    offscreen_surface: *mut c_void,
    backdrop: Option<Texture>,
    r: *mut c_void,
    exported: Option<CString>,
    fonts: HashMap<(i32, u16), *mut c_void>,
    font_root: PathBuf,
    text: TextCache<(i32, u16), Texture>,
    glass_text: TextCache<(i32, u16, bool), Texture>,
    text_widths: TextCache<(i32, u16), i32>,
    icons: HashMap<String, Texture>,
    shapes: RefCell<HashMap<ShapeKey, Texture>>,
    shape_bytes: Cell<usize>,
    pub backend: String,
    pub alpha: u8,
    pub shift: (i32, i32),
    pub light: bool,
}
unsafe fn err() -> String {
    CStr::from_ptr(SDL_GetError())
        .to_string_lossy()
        .into_owned()
}
impl Graphics {
    pub unsafe fn new() -> Result<Self, String> {
        SDL_SetHint(c"SDL_RENDER_SCALE_QUALITY".as_ptr(), c"1".as_ptr());
        if SDL_Init(0x20 | 0x4000) != 0 {
            return Err(err());
        }
        if TTF_Init() != 0 {
            return Err(err());
        }
        SDL_GL_SetAttribute(3, 8);
        let name = CString::new("Aerial Home Native").unwrap();
        let w = SDL_CreateWindow(name.as_ptr(), 0, 0, 1920, 1080, 5);
        if w.is_null() {
            return Err(err());
        }
        let r = SDL_CreateRenderer(w, -1, 6);
        if r.is_null() {
            SDL_DestroyWindow(w);
            return Err(err());
        }
        SDL_RenderSetLogicalSize(r, 1920, 1080);
        SDL_SetRenderDrawBlendMode(r, 1);
        let mut info: Info = std::mem::zeroed();
        SDL_GetRendererInfo(r, &mut info);
        let backend = if info.name.is_null() {
            "unknown".into()
        } else {
            CStr::from_ptr(info.name).to_string_lossy().into_owned()
        };
        let raw = SDL_webOSCreateExportedWindow(0);
        let exported = if raw.is_null() {
            None
        } else {
            Some(CStr::from_ptr(raw).to_owned())
        };
        if let Some(id) = &exported {
            let mut src = Rect {
                x: 0,
                y: 0,
                w: 1920,
                h: 1080,
            };
            let mut dst = Rect {
                x: 0,
                y: 0,
                w: 1920,
                h: 1080,
            };
            SDL_webOSSetExportedWindow(id.as_ptr(), &mut src, &mut dst);
        }
        Ok(Self {
            window: w,
            offscreen_surface: ptr::null_mut(),
            backdrop: None,
            r,
            exported,
            fonts: HashMap::new(),
            font_root: crate::platform::install_directory()?.join("fonts"),
            text: TextCache::new(1025),
            glass_text: TextCache::new(96),
            text_widths: TextCache::new(2048),
            icons: HashMap::new(),
            shapes: RefCell::new(HashMap::new()),
            shape_bytes: Cell::new(0),
            backend,
            alpha: 255,
            shift: (0, 0),
            light: false,
        })
    }
    /// Render an owned memory surface without initializing video, creating a
    /// window, registering an app or touching the TV's foreground surface.
    pub unsafe fn new_offscreen() -> Result<Self, String> {
        if SDL_Init(0) != 0 {
            return Err(err());
        }
        if TTF_Init() != 0 {
            return Err(err());
        }
        let surface = SDL_CreateRGBSurfaceWithFormat(0, 1920, 1080, 32, 0x16762004);
        if surface.is_null() {
            return Err(err());
        }
        let r = SDL_CreateSoftwareRenderer(surface);
        if r.is_null() {
            SDL_FreeSurface(surface);
            return Err(err());
        }
        SDL_RenderSetLogicalSize(r, 1920, 1080);
        SDL_SetRenderDrawBlendMode(r, 1);
        Ok(Self {
            window: ptr::null_mut(),
            offscreen_surface: surface,
            backdrop: None,
            r,
            exported: None,
            fonts: HashMap::new(),
            font_root: crate::platform::install_directory()?.join("fonts"),
            text: TextCache::new(1025),
            glass_text: TextCache::new(96),
            text_widths: TextCache::new(2048),
            icons: HashMap::new(),
            shapes: RefCell::new(HashMap::new()),
            shape_bytes: Cell::new(0),
            backend: "software-preview".into(),
            alpha: 255,
            shift: (0, 0),
            light: false,
        })
    }
    pub unsafe fn preview_background(&mut self, path: &str) -> Result<(), String> {
        if self.offscreen_surface.is_null() {
            return Err("A preview background requires an offscreen renderer".into());
        }
        let path = CString::new(path).map_err(|_| "Invalid preview background path")?;
        let texture = IMG_LoadTexture(self.r, path.as_ptr());
        if texture.is_null() {
            return Err(err());
        }
        let (mut w, mut h) = (0, 0);
        SDL_QueryTexture(texture, ptr::null_mut(), ptr::null_mut(), &mut w, &mut h);
        if let Some(old) = self.backdrop.replace(Texture { ptr: texture, w, h }) {
            SDL_DestroyTexture(old.ptr);
        }
        Ok(())
    }
    pub unsafe fn verify_offscreen(&self) -> Result<(), String> {
        if self.offscreen_surface.is_null()
            || !self.window.is_null()
            || self.exported.is_some()
            || SDL_WasInit(0x20) != 0
        {
            return Err("Offscreen preview unexpectedly initialized video".into());
        }
        Ok(())
    }
    pub unsafe fn begin(&mut self, video: bool) {
        SDL_RenderSetClipRect(self.r, ptr::null());
        self.alpha = 255;
        self.shift = (0, 0);
        if self.light {
            SDL_SetRenderDrawColor(self.r, 219, 231, 243, if video { 0 } else { 255 });
        } else {
            SDL_SetRenderDrawColor(self.r, 12, 21, 34, if video { 0 } else { 255 });
        }
        SDL_RenderClear(self.r);
        if let Some(t) = &self.backdrop {
            SDL_RenderCopy(
                self.r,
                t.ptr,
                ptr::null(),
                &Rect {
                    x: 0,
                    y: 0,
                    w: 1920,
                    h: 1080,
                },
            );
            return;
        }
        if !video {
            self.vertical_gradient(0, 0, 1920, 520, [53, 75, 112, 255], [31, 53, 68, 255]);
            self.vertical_gradient(0, 520, 1920, 560, [31, 53, 68, 255], [24, 36, 57, 255]);
        }
    }
    pub unsafe fn present(&self) {
        SDL_RenderPresent(self.r);
    }
    pub fn video_window(&self) -> Option<&str> {
        self.exported.as_ref().and_then(|s| s.to_str().ok())
    }
    pub unsafe fn rect(&self, x: i32, y: i32, w: i32, h: i32, c: [u8; 4]) {
        if w <= 0 || h <= 0 {
            return;
        }
        SDL_SetRenderDrawColor(
            self.r,
            c[0],
            c[1],
            c[2],
            ((c[3] as u16 * self.alpha as u16) / 255) as u8,
        );
        SDL_RenderFillRect(
            self.r,
            &Rect {
                x: x + self.shift.0,
                y: y + self.shift.1,
                w,
                h,
            },
        );
    }
    pub unsafe fn set_clip(&self, rect: Option<(i32, i32, i32, i32)>) {
        let rect = rect.map(|(x, y, w, h)| Rect {
            x: x + self.shift.0,
            y: y + self.shift.1,
            w,
            h,
        });
        SDL_RenderSetClipRect(self.r, rect.as_ref().map_or(ptr::null(), |r| r));
    }
    pub unsafe fn line(&self, x1: i32, y1: i32, x2: i32, y2: i32, c: [u8; 4]) {
        SDL_SetRenderDrawColor(
            self.r,
            c[0],
            c[1],
            c[2],
            ((c[3] as u16 * self.alpha as u16) / 255) as u8,
        );
        SDL_RenderDrawLine(
            self.r,
            x1 + self.shift.0,
            y1 + self.shift.1,
            x2 + self.shift.0,
            y2 + self.shift.1,
        );
    }
    unsafe fn cached_shape(
        &self,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        r: i32,
        stroke: i32,
        top: [u8; 4],
        bottom: [u8; 4],
    ) {
        if self.alpha == 0 || w <= 0 || h <= 0 {
            return;
        }
        let key = ShapeKey {
            w,
            h,
            r,
            stroke,
            top,
            bottom,
        };
        let mut cache = self.shapes.borrow_mut();
        if !cache.contains_key(&key) {
            let Some(mut pixels) = crate::raster::rounded(w, h, r, stroke, top, bottom) else {
                return;
            };
            let surface = SDL_CreateRGBSurfaceWithFormatFrom(
                pixels.as_mut_ptr().cast(),
                w,
                h,
                32,
                w * 4,
                0x16762004,
            );
            if surface.is_null() {
                return;
            }
            let texture = SDL_CreateTextureFromSurface(self.r, surface);
            SDL_FreeSurface(surface);
            if texture.is_null() {
                return;
            }
            if cache.len() >= 192 || self.shape_bytes.get() + pixels.len() > 48 * 1024 * 1024 {
                for (_, t) in cache.drain() {
                    SDL_DestroyTexture(t.ptr);
                }
                self.shape_bytes.set(0);
            }
            self.shape_bytes.set(self.shape_bytes.get() + pixels.len());
            cache.insert(key, Texture { ptr: texture, w, h });
        }
        let key = ShapeKey {
            w,
            h,
            r,
            stroke,
            top,
            bottom,
        };
        let t = &cache[&key];
        SDL_SetTextureAlphaMod(t.ptr, self.alpha);
        SDL_SetTextureColorMod(t.ptr, 255, 255, 255);
        SDL_RenderCopy(
            self.r,
            t.ptr,
            ptr::null(),
            &Rect {
                x: x + self.shift.0,
                y: y + self.shift.1,
                w,
                h,
            },
        );
    }
    pub unsafe fn vertical_gradient(
        &self,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        top: [u8; 4],
        bottom: [u8; 4],
    ) {
        self.cached_shape(x, y, w, h, 0, 0, top, bottom);
    }
    pub unsafe fn round_gradient(
        &self,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        r: i32,
        top: [u8; 4],
        bottom: [u8; 4],
    ) {
        self.cached_shape(x, y, w, h, r, 0, top, bottom);
    }
    pub unsafe fn round_stroke(
        &self,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        r: i32,
        width: i32,
        c: [u8; 4],
    ) {
        self.cached_shape(x, y, w, h, r, width, c, c);
    }
    pub unsafe fn round(&self, x: i32, y: i32, w: i32, h: i32, r: i32, c: [u8; 4]) {
        self.cached_shape(x, y, w, h, r, 0, c, c);
    }
    pub unsafe fn glass(&self, x: i32, y: i32, w: i32, h: i32, focus: bool) {
        let color = if focus {
            [245, 248, 255, 255]
        } else if self.light {
            [228, 237, 251, 184]
        } else {
            [63, 73, 91, 164]
        };
        self.round(x, y, w, h, 28, color);
        self.round_stroke(x, y, w, h, 28, 1, [255, 255, 255, 74]);
    }
    unsafe fn font(&mut self, size: i32, weight: u16) -> *mut c_void {
        let key = (size.abs(), weight);
        if let Some(f) = self.fonts.get(&key) {
            return *f;
        }
        let named = match weight {
            300 => "Light",
            400 => "Regular",
            500 => "Medium",
            600 => "SemiBold",
            700 => "Bold",
            _ => "Regular",
        };
        let mut names = vec![];
        if size.abs() >= 48 {
            names.push(self.font_root.join(format!("Inter-Display-{weight}.ttf")));
        }
        names.push(self.font_root.join(format!("Inter-{weight}.ttf")));
        names.push(self.font_root.join(format!("Inter-{named}.ttf")));
        names.extend([
            PathBuf::from("/usr/share/fonts/LG_Smart_UI-Regular.ttf"),
            PathBuf::from("/usr/share/fonts/DejaVuSans.ttf"),
        ]);
        for name in names {
            let Ok(c) = CString::new(name.to_string_lossy().as_bytes()) else {
                continue;
            };
            let f = TTF_OpenFont(c.as_ptr(), size.abs());
            if !f.is_null() {
                self.fonts.insert(key, f);
                return f;
            }
        }
        ptr::null_mut()
    }
    pub unsafe fn text_width(&mut self, s: &str, size: i32, weight: u16) -> i32 {
        let s = clean_text(s);
        let key = (size.abs(), weight);
        if let Some(width) = self.text_widths.get(key, &s) {
            return *width;
        }
        let f = self.font(size, weight);
        let Ok(c) = CString::new(s.as_bytes()) else {
            return 0;
        };
        if f.is_null() {
            return 0;
        }
        let (mut w, mut h) = (0, 0);
        if TTF_SizeUTF8(f, c.as_ptr(), &mut w, &mut h) == 0 {
            if self.text_widths.is_full() {
                self.text_widths.drain().for_each(drop);
            }
            let _ = self.text_widths.try_insert(key, &s, w);
            w
        } else {
            0
        }
    }
    pub unsafe fn text_width_tracking(
        &mut self,
        s: &str,
        size: i32,
        weight: u16,
        tracking: f32,
    ) -> i32 {
        (self.text_width(s, size, weight) as f32
            + tracking * s.chars().count().saturating_sub(1) as f32)
            .round() as i32
    }
    pub unsafe fn text_tracking(
        &mut self,
        s: &str,
        size: i32,
        weight: u16,
        tracking: f32,
        x: i32,
        y: i32,
        c: [u8; 4],
        max_width: i32,
    ) {
        if tracking == 0.0 {
            self.text_weight(s, size, weight, x, y, c, max_width);
            return;
        }
        for (i, (byte, ch)) in s.char_indices().enumerate() {
            let prefix = &s[..byte + ch.len_utf8()];
            let mut bytes = [0; 4];
            let glyph = ch.encode_utf8(&mut bytes);
            let at = self.text_width(prefix, size, weight) - self.text_width(glyph, size, weight)
                + (i as f32 * tracking).round() as i32;
            if at >= max_width {
                break;
            }
            if !ch.is_whitespace() {
                self.text_weight(glyph, size, weight, x + at, y, c, max_width - at);
            }
        }
    }
    pub unsafe fn text(&mut self, s: &str, size: i32, x: i32, y: i32, c: [u8; 4], max_width: i32) {
        self.text_weight(
            s,
            size.abs(),
            if size < 0 { 300 } else { 400 },
            x,
            y,
            c,
            max_width,
        );
    }
    /// Static glass ink is rasterized once per glyph/size/theme. No per-frame
    /// backdrop sampling, clipping bands or text-outline redraw is required.
    pub unsafe fn text_glass(
        &mut self,
        s: &str,
        size: i32,
        weight: u16,
        x: i32,
        y: i32,
        max_width: i32,
    ) {
        if self.alpha == 0 || s.is_empty() {
            return;
        }
        let key = (size, weight, self.light);
        if self.glass_text.get(key, s).is_none() {
            let font = self.font(size, weight);
            if font.is_null() {
                return;
            }
            let Ok(text) = CString::new(s) else {
                return;
            };
            let raw = TTF_RenderUTF8_Blended(
                font,
                text.as_ptr(),
                Color {
                    r: 255,
                    g: 255,
                    b: 255,
                    a: 255,
                },
            );
            if raw.is_null() {
                return;
            }
            let surface = SDL_ConvertSurfaceFormat(raw, 0x16762004, 0);
            SDL_FreeSurface(raw);
            if surface.is_null() {
                return;
            }
            let source = &*surface;
            let (w, h) = (source.w + 2, source.h + 2);
            if w <= 2 || h <= 2 || w > 2048 || h > 2048 {
                SDL_FreeSurface(surface.cast());
                return;
            }
            let mut pixels = vec![0u8; (w * h * 4) as usize];
            let light = [
                (0.08, [255, 255, 255, 237]),
                (0.34, [230, 242, 255, 150]),
                (0.47, [255, 255, 255, 223]),
                (0.68, [185, 219, 247, 102]),
                (0.94, [255, 255, 255, 219]),
            ];
            let dark = [
                (0.08, [23, 48, 75, 222]),
                (0.34, [46, 75, 106, 140]),
                (0.47, [23, 48, 75, 217]),
                (0.68, [69, 99, 125, 136]),
                (0.94, [23, 48, 75, 219]),
            ];
            let stops = if self.light { dark } else { light };
            let edge = if self.light {
                [24, 54, 85, 112]
            } else {
                [242, 248, 255, 156]
            };
            let sample = |cx: i32, cy: i32| -> u8 {
                if cx < 0 || cy < 0 || cx >= source.w || cy >= source.h {
                    0
                } else {
                    *source
                        .pixels
                        .offset((cy * source.pitch + cx * 4 + 3) as isize)
                }
            };
            for row in 0..h {
                for col in 0..w {
                    let a = sample(col - 1, row - 1) as f32 / 255.0;
                    let outer = [
                        sample(col - 2, row - 1),
                        sample(col, row - 1),
                        sample(col - 1, row - 2),
                        sample(col - 1, row),
                    ]
                    .into_iter()
                    .max()
                    .unwrap_or(0) as f32
                        / 255.0;
                    if a == 0.0 && outer == 0.0 {
                        continue;
                    }
                    let p = (row as f32 + 0.15 * col as f32) / (h as f32 + 0.15 * w as f32);
                    let (lo, hi) = stops
                        .windows(2)
                        .find(|v| p >= v[0].0 && p <= v[1].0)
                        .map(|v| (v[0], v[1]))
                        .unwrap_or_else(|| {
                            if p < stops[0].0 {
                                (stops[0], stops[0])
                            } else {
                                (stops[4], stops[4])
                            }
                        });
                    let t = if hi.0 > lo.0 {
                        (p - lo.0) / (hi.0 - lo.0)
                    } else {
                        0.0
                    };
                    let mut color = [0f32; 4];
                    for c in 0..4 {
                        color[c] = lo.1[c] as f32 + (hi.1[c] as f32 - lo.1[c] as f32) * t;
                    }
                    let fill = a * color[3] / 255.0;
                    let stroke = outer * edge[3] as f32 / 255.0 * (1.0 - fill);
                    let alpha = fill + stroke;
                    let at = ((row * w + col) * 4) as usize;
                    for c in 0..3 {
                        pixels[at + c] = ((color[c] * fill + edge[c] as f32 * stroke)
                            / alpha.max(0.0001))
                        .round() as u8;
                    }
                    pixels[at + 3] = (alpha * 255.0).round() as u8;
                }
            }
            SDL_FreeSurface(surface.cast());
            let surface = SDL_CreateRGBSurfaceWithFormatFrom(
                pixels.as_mut_ptr().cast(),
                w,
                h,
                32,
                w * 4,
                0x16762004,
            );
            if surface.is_null() {
                return;
            }
            let texture = SDL_CreateTextureFromSurface(self.r, surface);
            SDL_FreeSurface(surface);
            if texture.is_null() {
                return;
            }
            if self.glass_text.is_full() {
                for old in self.glass_text.drain() {
                    SDL_DestroyTexture(old.ptr);
                }
            }
            if let Err(old) = self
                .glass_text
                .try_insert(key, s, Texture { ptr: texture, w, h })
            {
                SDL_DestroyTexture(old.ptr);
                return;
            }
        }
        let Some(t) = self.glass_text.get(key, s) else {
            return;
        };
        let w = t.w.min(max_width + 2);
        SDL_SetTextureAlphaMod(t.ptr, self.alpha);
        SDL_SetTextureColorMod(t.ptr, 255, 255, 255);
        SDL_RenderCopy(
            self.r,
            t.ptr,
            &Rect {
                x: 0,
                y: 0,
                w,
                h: t.h,
            },
            &Rect {
                x: x - 1 + self.shift.0,
                y: y - 1 + self.shift.1,
                w,
                h: t.h,
            },
        );
    }
    pub unsafe fn text_weight(
        &mut self,
        s: &str,
        size: i32,
        weight: u16,
        x: i32,
        y: i32,
        c: [u8; 4],
        max_width: i32,
    ) {
        if s.is_empty() || self.alpha == 0 {
            return;
        }
        let s = clean_text(s);
        let key = (size, weight);
        if self.text.get(key, &s).is_none() {
            if self.text.is_full() {
                for t in self.text.drain() {
                    SDL_DestroyTexture(t.ptr);
                }
            }
            let f = self.font(size, weight);
            if f.is_null() {
                return;
            }
            let cs = CString::new(s.as_bytes()).unwrap();
            let surf = TTF_RenderUTF8_Blended(
                f,
                cs.as_ptr(),
                Color {
                    r: 255,
                    g: 255,
                    b: 255,
                    a: 255,
                },
            );
            if surf.is_null() {
                return;
            }
            let t = SDL_CreateTextureFromSurface(self.r, surf);
            SDL_FreeSurface(surf);
            if t.is_null() {
                return;
            }
            let (mut w, mut h) = (0, 0);
            SDL_QueryTexture(t, ptr::null_mut(), ptr::null_mut(), &mut w, &mut h);
            if let Err(old) = self.text.try_insert(key, &s, Texture { ptr: t, w, h }) {
                SDL_DestroyTexture(old.ptr);
                return;
            }
        }
        let c = if self.light && c[0] > 150 {
            [20, 32, 51, c[3]]
        } else {
            c
        };
        let Some(t) = self.text.get(key, &s) else {
            return;
        };
        let w = t.w.min(max_width.max(1));
        SDL_SetTextureAlphaMod(t.ptr, ((c[3] as u16 * self.alpha as u16) / 255) as u8);
        SDL_SetTextureColorMod(t.ptr, c[0], c[1], c[2]);
        SDL_RenderCopy(
            self.r,
            t.ptr,
            &Rect {
                x: 0,
                y: 0,
                w,
                h: t.h,
            },
            &Rect {
                x: x + self.shift.0,
                y: y + self.shift.1,
                w,
                h: t.h,
            },
        );
    }
    /// Cache a contained image with the same rounded overflow mask as the web
    /// tile. The small raster is created once; focus scaling stays on the GPU.
    pub unsafe fn icon_rounded(
        &mut self,
        path: &str,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        r: i32,
        mono: bool,
    ) -> bool {
        if w <= 0 || h <= 0 {
            return false;
        }
        let cw = 320usize;
        let ch = ((h as f32 / w as f32) * cw as f32)
            .round()
            .clamp(1.0, 512.0) as usize;
        let radius = (r as f32 / w as f32 * cw as f32).clamp(0.0, cw.min(ch) as f32 / 2.0);
        let key = format!("{path}#rounded-{ch}-{}-{mono}", radius.round() as i32);
        if !self.icons.contains_key(&key) {
            if self.icons.len() >= 192 {
                for (_, t) in self.icons.drain() {
                    SDL_DestroyTexture(t.ptr);
                }
            }
            let Ok(p) = CString::new(path) else {
                return false;
            };
            let original = IMG_Load(p.as_ptr());
            if original.is_null() {
                return false;
            }
            let converted = SDL_ConvertSurfaceFormat(original, 0x16762004, 0);
            SDL_FreeSurface(original);
            if converted.is_null() {
                return false;
            }
            let source = &*converted;
            if source.w <= 0
                || source.h <= 0
                || source.w > 2048
                || source.h > 2048
                || source.pitch < source.w * 4
            {
                SDL_FreeSurface(converted.cast());
                return false;
            }
            let scale = (cw as f32 / source.w as f32).min(ch as f32 / source.h as f32);
            let iw = (source.w as f32 * scale).round().max(1.0) as usize;
            let ih = (source.h as f32 * scale).round().max(1.0) as usize;
            let ox = (cw - iw.min(cw)) / 2;
            let oy = (ch - ih.min(ch)) / 2;
            let mut pixels = vec![0u8; cw * ch * 4];
            for py in oy..(oy + ih).min(ch) {
                for px in ox..(ox + iw).min(cw) {
                    let sx =
                        (((px - ox) as f32 + 0.5) / scale - 0.5).clamp(0.0, (source.w - 1) as f32);
                    let sy =
                        (((py - oy) as f32 + 0.5) / scale - 0.5).clamp(0.0, (source.h - 1) as f32);
                    let (ix, iy) = (sx as i32, sy as i32);
                    let (fx, fy) = (sx - ix as f32, sy - iy as f32);
                    let mut value = [0u8; 4];
                    for c in 0..4 {
                        let at = |a: i32, b: i32| {
                            *source.pixels.offset(
                                (b.min(source.h - 1) * source.pitch
                                    + a.min(source.w - 1) * 4
                                    + c as i32) as isize,
                            ) as f32
                        };
                        value[c] = ((at(ix, iy) * (1.0 - fx) + at(ix + 1, iy) * fx) * (1.0 - fy)
                            + (at(ix, iy + 1) * (1.0 - fx) + at(ix + 1, iy + 1) * fx) * fy)
                            .round() as u8;
                    }
                    if mono {
                        let grey =
                            ((value[0] as u32 * 54 + value[1] as u32 * 183 + value[2] as u32 * 19)
                                / 256) as u8;
                        value[0] = grey;
                        value[1] = grey;
                        value[2] = grey;
                    }
                    let (cx, cy) = (px as f32 + 0.5, py as f32 + 0.5);
                    let dx = (radius - cx).max(cx - (cw as f32 - radius)).max(0.0);
                    let dy = (radius - cy).max(cy - (ch as f32 - radius)).max(0.0);
                    let coverage = if radius == 0.0 {
                        1.0
                    } else {
                        (radius + 0.5 - (dx * dx + dy * dy).sqrt()).clamp(0.0, 1.0)
                    };
                    value[3] = (value[3] as f32 * coverage).round() as u8;
                    pixels[(py * cw + px) * 4..(py * cw + px) * 4 + 4].copy_from_slice(&value);
                }
            }
            crate::raster::bleed_contained_rgb(&mut pixels, cw, ch, ox, oy, iw.min(cw), ih.min(ch));
            SDL_FreeSurface(converted.cast());
            let surface = SDL_CreateRGBSurfaceWithFormatFrom(
                pixels.as_mut_ptr().cast(),
                cw as i32,
                ch as i32,
                32,
                cw as i32 * 4,
                0x16762004,
            );
            if surface.is_null() {
                return false;
            }
            let t = SDL_CreateTextureFromSurface(self.r, surface);
            SDL_FreeSurface(surface);
            if t.is_null() {
                return false;
            }
            self.icons.insert(
                key.clone(),
                Texture {
                    ptr: t,
                    w: cw as i32,
                    h: ch as i32,
                },
            );
        }
        let t = &self.icons[&key];
        SDL_SetTextureAlphaMod(t.ptr, self.alpha);
        SDL_SetTextureColorMod(t.ptr, 255, 255, 255);
        SDL_RenderCopy(
            self.r,
            t.ptr,
            ptr::null(),
            &Rect {
                x: x + self.shift.0,
                y: y + self.shift.1,
                w,
                h,
            },
        );
        true
    }
    pub unsafe fn icon(&mut self, path: &str, x: i32, y: i32, w: i32, h: i32, mono: bool) -> bool {
        let key = if mono {
            format!("{path}#mono")
        } else {
            path.into()
        };
        if !self.icons.contains_key(&key) {
            if self.icons.len() >= 96 {
                return false;
            }
            let Ok(p) = CString::new(path) else {
                return false;
            };
            let t = if mono {
                let raw = IMG_Load(p.as_ptr());
                if raw.is_null() {
                    return false;
                }
                let surface = SDL_ConvertSurfaceFormat(raw, 0x16762004, 0);
                SDL_FreeSurface(raw);
                if surface.is_null() {
                    return false;
                }
                let s = &*surface;
                if s.w <= 0 || s.h <= 0 || s.w > 2048 || s.h > 2048 || s.pitch < s.w * 4 {
                    SDL_FreeSurface(surface.cast());
                    return false;
                }
                for iy in 0..s.h {
                    for ix in 0..s.w {
                        let px = s.pixels.offset((iy * s.pitch + ix * 4) as isize);
                        let grey = (((*px as u32) * 54
                            + (*px.add(1) as u32) * 183
                            + (*px.add(2) as u32) * 19)
                            / 256) as u8;
                        *px = grey;
                        *px.add(1) = grey;
                        *px.add(2) = grey;
                    }
                }
                let t = SDL_CreateTextureFromSurface(self.r, surface.cast());
                SDL_FreeSurface(surface.cast());
                t
            } else {
                IMG_LoadTexture(self.r, p.as_ptr())
            };
            if t.is_null() {
                return false;
            }
            let (mut tw, mut th) = (0, 0);
            SDL_QueryTexture(t, ptr::null_mut(), ptr::null_mut(), &mut tw, &mut th);
            self.icons.insert(
                key.clone(),
                Texture {
                    ptr: t,
                    w: tw,
                    h: th,
                },
            );
        }
        let t = &self.icons[&key];
        let scale = (w as f32 / t.w.max(1) as f32).min(h as f32 / t.h.max(1) as f32);
        let (tw, th) = ((t.w as f32 * scale) as i32, (t.h as f32 * scale) as i32);
        SDL_SetTextureAlphaMod(t.ptr, self.alpha);
        SDL_SetTextureColorMod(t.ptr, 255, 255, 255);
        SDL_RenderCopy(
            self.r,
            t.ptr,
            ptr::null(),
            &Rect {
                x: x + (w - tw) / 2 + self.shift.0,
                y: y + (h - th) / 2 + self.shift.1,
                w: tw,
                h: th,
            },
        );
        true
    }
    pub unsafe fn screenshot(&self, path: &str) {
        let mut p = vec![0u8; 1920 * 1080 * 4];
        if SDL_RenderReadPixels(
            self.r,
            ptr::null(),
            0x16762004,
            p.as_mut_ptr().cast(),
            1920 * 4,
        ) == 0
        {
            let mut ppm = b"P6\n1920 1080\n255\n".to_vec();
            for c in p.chunks_exact(4) {
                ppm.extend_from_slice(&[c[0], c[1], c[2]]);
            }
            let _ = std::fs::write(path, ppm);
        }
    }
}
impl Drop for Graphics {
    fn drop(&mut self) {
        unsafe {
            for t in self.text.drain() {
                SDL_DestroyTexture(t.ptr);
            }
            for t in self.glass_text.drain() {
                SDL_DestroyTexture(t.ptr);
            }
            for (_, t) in self.icons.drain() {
                SDL_DestroyTexture(t.ptr);
            }
            if let Some(t) = self.backdrop.take() {
                SDL_DestroyTexture(t.ptr);
            }
            for (_, t) in self.shapes.get_mut().drain() {
                SDL_DestroyTexture(t.ptr);
            }
            for (_, f) in self.fonts.drain() {
                TTF_CloseFont(f);
            }
            if let Some(id) = &self.exported {
                SDL_webOSDestroyExportedWindow(id.as_ptr());
            }
            SDL_DestroyRenderer(self.r);
            if !self.window.is_null() {
                SDL_DestroyWindow(self.window);
            }
            if !self.offscreen_surface.is_null() {
                SDL_FreeSurface(self.offscreen_surface);
            }
            TTF_Quit();
            SDL_Quit();
        }
    }
}
