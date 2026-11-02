#[cfg(feature = "sdl")]
mod clock;
#[cfg(all(
    target_os = "linux",
    any(test, all(feature = "sdl", target_arch = "arm"))
))]
mod compat;
#[cfg(feature = "sdl")]
mod graphics;
#[cfg(unix)]
mod home;
#[cfg(feature = "sdl")]
mod launcher;
mod platform;
#[cfg(any(feature = "sdl", test))]
mod raster;
mod video;
#[cfg(feature = "sdl")]
fn main() {
    let args: Vec<String> = std::env::args().collect();
    if let Some(index) = args.iter().position(|a| a == "--preview") {
        println!("AERIAL_HOME_OFFSCREEN_PREVIEW_V1");
        std::env::set_var("SDL_VIDEODRIVER", "aerial-preview-disabled");
        let Some(output) = args.get(index + 1) else {
            eprintln!("--preview requires an output path");
            std::process::exit(2);
        };
        let screen = args
            .iter()
            .position(|a| a == "--screen")
            .and_then(|i| args.get(i + 1))
            .map(String::as_str)
            .unwrap_or("home");
        if let Err(error) = unsafe { launcher::preview(output, screen) } {
            eprintln!("Native preview: {error}");
            std::process::exit(1);
        }
        return;
    }
    if std::env::args().any(|a| a == "--home") {
        let _ = home::notify();
        return;
    }
    if let Err(e) = unsafe { launcher::run() } {
        eprintln!("Aerial Home Native: {e}");
        std::process::exit(1);
    }
}
#[cfg(not(feature = "sdl"))]
fn main() {
    println!("Build with --features sdl for the native TV launcher.");
}
