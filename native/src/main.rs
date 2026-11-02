#[cfg(not(feature = "sdl"))]
fn main() {
    println!("Build with --features sdl using the webOS SDL2 toolchain to run the rendering probe.");
}

#[cfg(feature = "sdl")]
mod display {
    use aerial_native_probe::{navigate, percentile, Direction, LABELS};
    use std::{ffi::{c_char, c_int, c_void, CStr, CString}, time::Instant};

    #[repr(C)]
    struct Rect { x: c_int, y: c_int, w: c_int, h: c_int }
    #[repr(C)]
    struct RendererInfo { name: *const c_char, flags: u32, formats: u32, texture_formats: [u32;16], max_w: c_int, max_h: c_int }
    #[repr(C)]
    union Event { kind: u32, bytes: [u8;56], alignment: u64 }
    #[link(name = "SDL2")]
    extern "C" {
        fn SDL_Init(flags: u32) -> c_int;
        fn SDL_Quit();
        fn SDL_GetError() -> *const c_char;
        fn SDL_CreateWindow(title: *const c_char, x: c_int, y: c_int, w: c_int, h: c_int, flags: u32) -> *mut c_void;
        fn SDL_DestroyWindow(window: *mut c_void);
        fn SDL_CreateRenderer(window: *mut c_void, index: c_int, flags: u32) -> *mut c_void;
        fn SDL_DestroyRenderer(renderer: *mut c_void);
        fn SDL_GetRendererInfo(renderer: *mut c_void, info: *mut RendererInfo) -> c_int;
        fn SDL_RenderSetLogicalSize(renderer: *mut c_void, w: c_int, h: c_int) -> c_int;
        fn SDL_SetRenderDrawColor(renderer: *mut c_void, r: u8, g: u8, b: u8, a: u8) -> c_int;
        fn SDL_RenderFillRect(renderer: *mut c_void, rect: *const Rect) -> c_int;
        fn SDL_RenderPresent(renderer: *mut c_void);
        fn SDL_WaitEventTimeout(event: *mut Event, timeout: c_int) -> c_int;
        fn SDL_PollEvent(event: *mut Event) -> c_int;
        fn SDL_GetTicks() -> u32;
    }
    unsafe fn error() -> String { CStr::from_ptr(SDL_GetError()).to_string_lossy().into_owned() }
    unsafe fn rect(r: *mut c_void, x: i32, y: i32, w: i32, h: i32, color: [u8;3]) {
        SDL_SetRenderDrawColor(r, color[0], color[1], color[2], 255);
        SDL_RenderFillRect(r, &Rect { x, y, w, h });
    }
    // Small original bitmap alphabet keeps the probe independent of TV fonts and
    // avoids font loading differences contaminating the first-frame experiment.
    fn glyph(c: char) -> [u8;7] {
        match c {
            'A'=>[14,17,17,31,17,17,17], 'B'=>[30,17,17,30,17,17,30],
            'C'=>[14,17,16,16,16,17,14], 'D'=>[30,17,17,17,17,17,30],
            'E'=>[31,16,16,30,16,16,31], 'F'=>[31,16,16,30,16,16,16],
            'G'=>[14,17,16,23,17,17,14], 'H'=>[17,17,17,31,17,17,17],
            'I'=>[31,4,4,4,4,4,31], 'J'=>[7,2,2,2,18,18,12],
            'K'=>[17,18,20,24,20,18,17], 'L'=>[16,16,16,16,16,16,31],
            'M'=>[17,27,21,21,17,17,17], 'N'=>[17,25,21,19,17,17,17],
            'O'=>[14,17,17,17,17,17,14], 'P'=>[30,17,17,30,16,16,16],
            'Q'=>[14,17,17,17,21,18,13], 'R'=>[30,17,17,30,20,18,17],
            'S'=>[15,16,16,14,1,1,30], 'T'=>[31,4,4,4,4,4,4],
            'U'=>[17,17,17,17,17,17,14], 'V'=>[17,17,17,17,17,10,4],
            'W'=>[17,17,17,21,21,21,10], 'X'=>[17,17,10,4,10,17,17],
            'Y'=>[17,17,10,4,4,4,4], 'Z'=>[31,1,2,4,8,16,31],
            '0'=>[14,17,19,21,25,17,14], '1'=>[4,12,4,4,4,4,14],
            '2'=>[14,17,1,2,4,8,31], '3'=>[30,1,1,14,1,1,30],
            '4'=>[2,6,10,18,31,2,2], '5'=>[31,16,16,30,1,1,30],
            '6'=>[14,16,16,30,17,17,14], '7'=>[31,1,2,4,8,8,8],
            '8'=>[14,17,17,14,17,17,14], '9'=>[14,17,17,15,1,1,14],
            '-'=>[0,0,0,31,0,0,0], ':'=>[0,4,4,0,4,4,0],
            _=>[0;7],
        }
    }
    unsafe fn text(r: *mut c_void, value: &str, x: i32, y: i32, scale: i32, color: [u8;3]) {
        for (i,c) in value.chars().enumerate() {
            for (row,bits) in glyph(c).iter().enumerate() {
                for col in 0..5 {
                    if bits & (1 << (4-col)) != 0 { rect(r,x+i as i32*6*scale+col*scale,y+row as i32*scale,scale,scale,color); }
                }
            }
        }
    }
    unsafe fn draw(r: *mut c_void, focus: usize, selected: bool) {
        rect(r,0,0,1920,1080,[12,20,32]);
        text(r,"AERIAL HOME - NATIVE RUST PROBE",130,100,5,[235,243,251]);
        text(r,"ARROWS TO MOVE - OK TO TEST - BACK TO EXIT",130,180,3,[153,172,190]);
        for (i,label) in LABELS.iter().enumerate() {
            let x=130+(i%5) as i32*335; let y=330+(i/5) as i32*270;
            if i==focus { rect(r,x-8,y-8,308,218,if selected {[90,210,175]} else {[225,239,252]}); }
            rect(r,x,y,292,202,if i==focus {[49,69,94]} else {[29,43,61]});
            text(r,&(i+1).to_string(),x+24,y+24,6,[132,188,223]);
            text(r,label,x+20,y+142,3,[235,243,251]);
        }
        text(r,"ISOLATED RENDERING TEST - DOES NOT LAUNCH APPS OR HDMI",130,940,3,[153,172,190]);
        SDL_RenderPresent(r);
    }
    pub unsafe fn run() -> Result<(), String> {
        let started=Instant::now();
        if SDL_Init(0x20|0x4000)!=0 { return Err(error()); }
        let title=CString::new("Aerial Native Probe").unwrap();
        let window=SDL_CreateWindow(title.as_ptr(),0,0,1920,1080,0x1|0x4);
        if window.is_null() { let e=error();SDL_Quit();return Err(e); }
        let renderer=SDL_CreateRenderer(window,-1,0x2|0x4);
        if renderer.is_null() { let e=error();SDL_DestroyWindow(window);SDL_Quit();return Err(e); }
        let mut info: RendererInfo=std::mem::zeroed();
        SDL_GetRendererInfo(renderer,&mut info);
        let renderer_name=if info.name.is_null(){"unknown".into()}else{CStr::from_ptr(info.name).to_string_lossy().into_owned()};
        SDL_RenderSetLogicalSize(renderer,1920,1080);
        let mut focus=0;let mut selected=false;
        draw(renderer,focus,selected);
        println!("{{\"event\":\"first-present\",\"process_to_present_ms\":{:.3},\"renderer\":\"{}\",\"accelerated\":{},\"video\":false}}",started.elapsed().as_secs_f64()*1000.0,renderer_name,info.flags&2!=0);
        let bench=std::env::args().any(|v|v=="--benchmark");
        let duration=if bench {20} else {300}; // Always return from this test app.
        let mut samples=Vec::new();let mut running=true;let mut step=0;
        while running && started.elapsed().as_secs()<duration {
            let mut event=Event{bytes:[0;56]};let mut dirty=false;let mut queued_at=None;
            let mut available=SDL_WaitEventTimeout(&mut event,if bench {80}else{100});
            while available!=0 {
                match event.kind {
                    0x100=>running=false,
                    0x200=>dirty=true,
                    0x300=>{
                        let bytes=event.bytes;
                        let key=i32::from_ne_bytes(bytes[20..24].try_into().unwrap());
                        queued_at=Some(u32::from_ne_bytes(bytes[4..8].try_into().unwrap()));
                        let direction=match key {1073741904=>Some(Direction::Left),1073741903=>Some(Direction::Right),1073741906=>Some(Direction::Up),1073741905=>Some(Direction::Down),_=>None};
                        if let Some(d)=direction { focus=navigate(focus,LABELS.len(),5,d);selected=false;dirty=true; }
                        else if key==13 {if focus==9 {running=false;}else{selected=!selected;dirty=true;}}
                        else if [27,461,1073742094,(1<<30)|482].contains(&key) {running=false;}
                    },_=>{}
                }
                available=SDL_PollEvent(&mut event);
            }
            if bench {
                focus=navigate(focus,LABELS.len(),5,if (step/4)%2==0 {Direction::Right}else{Direction::Left});
                queued_at=Some(SDL_GetTicks());dirty=true;step+=1;
            }
            if dirty&&running {
                let render_start=Instant::now();draw(renderer,focus,selected);
                let render_ms=render_start.elapsed().as_secs_f64()*1000.0;
                if let Some(at)=queued_at {
                    let queue_ms=SDL_GetTicks().wrapping_sub(at) as f64;
                    samples.push(queue_ms);
                    println!("{{\"event\":\"navigation-present\",\"automatic\":{},\"queue_to_present_ms\":{:.3},\"render_present_ms\":{:.3}}}",bench,queue_ms,render_ms);
                }
            }
        }
        if let (Some(median),Some(p95))=(percentile(&samples,0.5),percentile(&samples,0.95)) {
            println!("{{\"event\":\"summary\",\"automatic\":{},\"samples\":{},\"median_ms\":{:.3},\"p95_ms\":{:.3}}}",bench,samples.len(),median,p95);
        }
        SDL_DestroyRenderer(renderer);SDL_DestroyWindow(window);SDL_Quit();Ok(())
    }
}

#[cfg(feature = "sdl")]
fn main() {
    if let Err(error)=unsafe { display::run() } {eprintln!("Native probe failed: {}",error);std::process::exit(1);}
}
#[cfg(all(target_os = "linux", any(test, all(feature = "sdl", target_arch = "arm"))))]
mod compat;
