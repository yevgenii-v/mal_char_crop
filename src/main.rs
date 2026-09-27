mod app;
mod clipboard;
mod crop;
mod dialog;
mod imaging;
mod rotation;
mod tab;

use eframe::egui;

/// Set on the re-spawned copy so it does not detach again.
const DETACHED_ENV: &str = "MAL_CROP_DETACHED";

/// When a release build is started from a terminal, re-launches itself in a
/// separate process group with no terminal I/O and exits, so closing the
/// terminal does not kill the window. Returns `true` if this process should exit.
fn detach_from_terminal() -> bool {
    use std::io::IsTerminal;
    use std::os::unix::process::CommandExt;
    use std::process::{Command, Stdio};

    if cfg!(debug_assertions)
        || std::env::var_os(DETACHED_ENV).is_some()
        || !std::io::stdin().is_terminal()
    {
        return false;
    }
    let Ok(exe) = std::env::current_exe() else { return false };
    Command::new(exe)
        .args(std::env::args_os().skip(1))
        .env(DETACHED_ENV, "1")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn()
        .is_ok()
}

fn main() -> eframe::Result {
    if detach_from_terminal() {
        return Ok(());
    }
    let initial = std::env::args_os().skip(1).map(std::path::PathBuf::from).collect();

    let mut options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("MAL Crop 9:14")
            .with_app_id("mal-crop")
            .with_inner_size([1000.0, 760.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };
    // winit has no file drag-and-drop on Wayland, so run through XWayland
    // unless native Wayland is explicitly requested.
    if std::env::var_os("MAL_CROP_WAYLAND").is_none() {
        options.event_loop_builder = Some(Box::new(|builder| {
            use winit::platform::x11::EventLoopBuilderExtX11;
            builder.with_x11();
        }));
    }

    eframe::run_native(
        "mal-crop",
        options,
        Box::new(|cc| {
            use_symbol_fallback(&cc.egui_ctx);
            Ok(Box::new(app::App::new(&cc.egui_ctx, initial)))
        }),
    )
}

/// The default proportional font lacks arrows such as "→"; fall back to the
/// bundled monospace font, which has them, instead of drawing a box.
fn use_symbol_fallback(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
        family.push("Hack".to_owned());
    }
    ctx.set_fonts(fonts);
}
