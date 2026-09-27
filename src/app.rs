use std::path::{Path, PathBuf};

use eframe::egui::{self, Color32, Event, InputState, Key, Modifiers, Vec2};
use image::RgbaImage;

use crate::clipboard::{self, Pasted};
use crate::crop::{self, OutSize};
use crate::dialog::{DialogKind, PendingDialog};
use crate::i18n::{Lang, Strings};
use crate::imaging::{self, SaveFormat};
use crate::tab::{Drag, Tab};

const WARN: Color32 = Color32::from_rgb(230, 160, 60);
/// Rotation per `[`/`]` press, and with Shift held.
const ROTATE_STEP: f32 = 1.0;
const ROTATE_FINE_STEP: f32 = 0.1;

/// Actions reachable from both the toolbar and the keyboard.
#[derive(Clone, Copy)]
enum Command {
    Open,
    Paste,
    Save,
    SaveAs,
    CloseTab,
    NextTab,
    PrevTab,
}

pub struct App {
    tabs: Vec<Tab>,
    active: usize,
    next_id: u64,
    drag: Drag,
    out: OutSize,
    dialog: Option<PendingDialog>,
    lang: Lang,
    /// `None` shows the welcome hint.
    status: Option<String>,
}

impl App {
    pub fn new(ctx: &egui::Context, initial: Vec<PathBuf>) -> Self {
        let mut app = Self {
            tabs: Vec::new(),
            active: 0,
            next_id: 0,
            drag: Drag::None,
            out: OutSize::Auto,
            dialog: None,
            lang: Lang::detect(),
            status: None,
        };
        for p in initial {
            app.open_path(ctx, &p);
        }
        app
    }

    fn t(&self) -> &'static Strings {
        self.lang.tr()
    }

    fn set_status(&mut self, s: impl Into<String>) {
        self.status = Some(s.into());
    }

    fn tab(&self) -> Option<&Tab> {
        self.tabs.get(self.active)
    }

    fn run(&mut self, ctx: &egui::Context, cmd: Command) {
        match cmd {
            Command::Open => self.open_dialog(ctx),
            Command::Paste => self.paste(ctx),
            Command::Save => self.quick_save(ctx),
            Command::SaveAs => self.save_dialog(ctx),
            Command::CloseTab => self.close_tab(self.active),
            Command::NextTab => self.cycle_tab(true),
            Command::PrevTab => self.cycle_tab(false),
        }
    }

    // ---- tabs ----------------------------------------------------------

    fn activate(&mut self, i: usize) {
        if i < self.tabs.len() && i != self.active {
            self.active = i;
            self.drag = Drag::None;
        }
    }

    fn close_tab(&mut self, i: usize) {
        if i >= self.tabs.len() {
            return;
        }
        self.tabs.remove(i);
        if i < self.active || self.active >= self.tabs.len() {
            self.active = self.active.saturating_sub(1);
        }
        self.drag = Drag::None;
    }

    fn cycle_tab(&mut self, forward: bool) {
        let n = self.tabs.len();
        if n > 1 {
            self.activate(if forward { (self.active + 1) % n } else { (self.active + n - 1) % n });
        }
    }

    // ---- loading & saving ----------------------------------------------

    fn open_path(&mut self, ctx: &egui::Context, path: &Path) {
        if let Some(i) = self.tabs.iter().position(|t| t.path.as_deref() == Some(path)) {
            self.activate(i);
            return;
        }
        match imaging::load(path) {
            Ok((rgba, format)) => self.add_tab(ctx, rgba, Some(path.to_path_buf()), format),
            Err(e) => self.set_status((self.t().open_failed)(&path.display(), &e)),
        }
    }

    fn add_tab(
        &mut self,
        ctx: &egui::Context,
        rgba: RgbaImage,
        path: Option<PathBuf>,
        format: SaveFormat,
    ) {
        let (w, h) = rgba.dimensions();
        if !crop::fits(w, h) {
            self.set_status((self.t().too_small)(w, h));
            return;
        }
        self.next_id += 1;
        let tab = Tab::new(ctx, self.next_id, rgba, path, format);
        self.set_status(format!("{} — {w}×{h}", tab.name(self.t())));
        self.tabs.push(tab);
        self.active = self.tabs.len() - 1;
        self.drag = Drag::None;
    }

    fn paste(&mut self, ctx: &egui::Context) {
        match clipboard::read() {
            Ok(Pasted::Image(rgba)) => self.add_tab(ctx, rgba, None, SaveFormat::Png),
            Ok(Pasted::Files(paths)) => {
                for p in paths {
                    self.open_path(ctx, &p);
                }
            }
            Err(e) => self.set_status(e.message(self.t())),
        }
    }

    fn save_tab(&mut self, id: u64, path: PathBuf) {
        let Some(tab) = self.tabs.iter().find(|t| t.id == id) else {
            self.set_status(self.t().tab_closed);
            return;
        };
        // An unsupported or missing extension gets the tab's own format.
        let (path, format) = match SaveFormat::of_path(&path) {
            Some(f) => (path, f),
            None => (path.with_extension(tab.format.ext()), tab.format),
        };
        let img = tab.render(self.out);
        let (w, h) = img.dimensions();
        let t = self.t();
        self.set_status(match imaging::save(&path, img, format) {
            Ok(()) => (t.saved)(w, h, &path.display()),
            Err(e) => (t.save_failed)(&e),
        });
    }

    fn quick_save(&mut self, ctx: &egui::Context) {
        let Some(tab) = self.tab() else { return };
        match tab.default_save_path() {
            Some(p) => self.save_tab(tab.id, p),
            None => self.save_dialog(ctx),
        }
    }

    fn dialog_dir(&self) -> Option<&Path> {
        self.tab()?.path.as_ref()?.parent()
    }

    fn open_dialog(&mut self, ctx: &egui::Context) {
        if self.dialog.is_none() {
            self.dialog = Some(PendingDialog::open(ctx, self.dialog_dir(), self.t().images));
        }
    }

    fn save_dialog(&mut self, ctx: &egui::Context) {
        let Some(tab) = self.tab() else { return };
        if self.dialog.is_some() {
            return;
        }
        let name = tab
            .default_save_path()
            .and_then(|p| Some(p.file_name()?.to_string_lossy().into_owned()))
            .unwrap_or_else(|| format!("mal.{}", tab.format.ext()));
        let dialog = PendingDialog::save(ctx, tab.id, self.dialog_dir(), name, tab.format);
        self.dialog = Some(dialog);
    }

    fn poll_dialog(&mut self, ctx: &egui::Context) {
        let Some(paths) = self.dialog.as_ref().and_then(PendingDialog::result) else { return };
        let Some(dialog) = self.dialog.take() else { return };
        match dialog.kind {
            DialogKind::Open => {
                for p in paths {
                    self.open_path(ctx, &p);
                }
            }
            DialogKind::Save(id) => {
                if let Some(p) = paths.into_iter().next() {
                    self.save_tab(id, p);
                }
            }
        }
    }

    // ---- input ---------------------------------------------------------

    fn handle_input(&mut self, ctx: &egui::Context) {
        let (dropped, commands) = ctx.input_mut(|i| {
            let dropped: Vec<_> = i.raw.dropped_files.iter().map(|f| f.path().to_path_buf()).collect();
            (dropped, shortcuts(i))
        });
        for p in dropped {
            self.open_path(ctx, &p);
        }
        for cmd in commands {
            self.run(ctx, cmd);
        }

        // Plain keys must not fire while typing in a text field.
        if ctx.memory(|m| m.focused().is_some()) {
            return;
        }
        let Some(tab) = self.tabs.get_mut(self.active) else { return };
        let (fit, nudge, rotate) = ctx.input(|i| {
            let step = if i.modifiers.shift { 10.0 } else { 1.0 };
            let turn = if i.modifiers.shift { ROTATE_FINE_STEP } else { ROTATE_STEP };
            let mut rotate = 0.0;
            if i.key_pressed(Key::OpenBracket) {
                rotate -= turn;
            }
            if i.key_pressed(Key::CloseBracket) {
                rotate += turn;
            }
            let mut d = Vec2::ZERO;
            for (key, dir) in [
                (Key::ArrowLeft, Vec2::LEFT),
                (Key::ArrowRight, Vec2::RIGHT),
                (Key::ArrowUp, Vec2::UP),
                (Key::ArrowDown, Vec2::DOWN),
            ] {
                if i.key_pressed(key) {
                    d += dir * step;
                }
            }
            (i.key_pressed(Key::F), d, rotate)
        });
        if fit {
            tab.fit_view();
        }
        if nudge != Vec2::ZERO {
            tab.nudge(nudge);
        }
        if rotate != 0.0 {
            tab.rotate_by(rotate);
        }
    }

    // ---- UI ------------------------------------------------------------

    fn toolbar(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let t = self.t();
        ui.horizontal(|ui| {
            if ui.button(t.open).on_hover_text("Ctrl+O").clicked() {
                self.run(&ctx, Command::Open);
            }
            if ui.button(t.paste).on_hover_text("Ctrl+V").clicked() {
                self.run(&ctx, Command::Paste);
            }
            ui.add_enabled_ui(self.tab().is_some(), |ui| {
                let hint = self
                    .tab()
                    .and_then(Tab::default_save_path)
                    .map_or("Ctrl+S".into(), |p| format!("Ctrl+S → {}", p.display()));
                if ui.button(t.save).on_hover_text(hint).clicked() {
                    self.run(&ctx, Command::Save);
                }
                if ui.button(t.save_as).on_hover_text("Ctrl+Shift+S").clicked() {
                    self.run(&ctx, Command::SaveAs);
                }
            });
            ui.separator();
            out_size_picker(ui, t, &mut self.out);
            if let Some(tab) = self.tabs.get_mut(self.active) {
                ui.separator();
                angle_picker(ui, t, tab);
                ui.separator();
                crop_summary(ui, t, tab, self.out);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                lang_picker(ui, t, &mut self.lang);
            });
        });
    }

    fn tab_bar(&mut self, ui: &mut egui::Ui) {
        let mut activate = None;
        let mut close = None;
        let t = self.t();
        egui::ScrollArea::horizontal().show(ui, |ui| {
            ui.horizontal(|ui| {
                for (i, tab) in self.tabs.iter().enumerate() {
                    let hover = tab
                        .path
                        .as_ref()
                        .map_or(tab.name(t).to_owned(), |p| p.display().to_string());
                    let resp = ui.selectable_label(i == self.active, tab.name(t)).on_hover_text(hover);
                    if resp.clicked() {
                        activate = Some(i);
                    }
                    if resp.middle_clicked() {
                        close = Some(i);
                    }
                    if ui.small_button("×").on_hover_text(t.close_tab).clicked() {
                        close = Some(i);
                    }
                    ui.add_space(6.0);
                }
            });
        });
        if let Some(i) = activate {
            self.activate(i);
        }
        if let Some(i) = close {
            self.close_tab(i);
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.poll_dialog(&ctx);
        self.handle_input(&ctx);

        let t = self.t();
        egui::Panel::top("toolbar").show(ui, |ui| self.toolbar(ui));
        if !self.tabs.is_empty() {
            egui::Panel::top("tabs").show(ui, |ui| self.tab_bar(ui));
        }
        egui::Panel::bottom("status").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(self.status.as_deref().unwrap_or(t.welcome));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.weak(t.help);
                });
            });
        });
        egui::CentralPanel::no_frame().show(ui, |ui| match self.tabs.get_mut(self.active) {
            Some(tab) => tab.canvas(ui, &mut self.drag),
            None => {
                let rect = ui.available_rect_before_wrap();
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    t.drop_here,
                    egui::FontId::proportional(20.0),
                    ui.visuals().weak_text_color(),
                );
            }
        });
    }
}

/// Consumes the keyboard shortcuts pressed this frame.
fn shortcuts(i: &mut InputState) -> Vec<Command> {
    let ctrl = Modifiers::COMMAND;
    let ctrl_shift = Modifiers::COMMAND | Modifiers::SHIFT;
    // Shift variants come first: a plain-Ctrl shortcut also matches with Shift held.
    let table = [
        (ctrl_shift, Key::S, Command::SaveAs),
        (ctrl, Key::S, Command::Save),
        (ctrl, Key::O, Command::Open),
        (ctrl, Key::W, Command::CloseTab),
        (ctrl_shift, Key::Tab, Command::PrevTab),
        (ctrl, Key::PageUp, Command::PrevTab),
        (ctrl, Key::Tab, Command::NextTab),
        (ctrl, Key::PageDown, Command::NextTab),
    ];
    // egui-winit swallows the Ctrl+V key press, but the release still arrives.
    let paste = i.events.iter().any(|e| match e {
        Event::Key { key: Key::V, pressed: false, modifiers, .. } => modifiers.command,
        Event::Paste(_) => true,
        _ => false,
    });
    let mut commands: Vec<_> = paste.then_some(Command::Paste).into_iter().collect();
    commands.extend(
        table.into_iter().filter(|&(mods, key, _)| i.consume_key(mods, key)).map(|(.., cmd)| cmd),
    );
    commands
}

fn size_label(k: u32) -> String {
    let (w, h) = crop::frame_size(k);
    format!("{w}×{h}")
}

fn lang_picker(ui: &mut egui::Ui, t: &Strings, lang: &mut Lang) {
    egui::ComboBox::from_id_salt("lang")
        .selected_text(lang.native_name())
        .show_ui(ui, |ui| {
            for l in Lang::ALL {
                ui.selectable_value(lang, l, l.native_name());
            }
        })
        .response
        .on_hover_text(t.language);
}

fn out_size_picker(ui: &mut egui::Ui, t: &Strings, out: &mut OutSize) {
    ui.label(t.size);
    let mut auto = *out == OutSize::Auto;
    ui.selectable_value(&mut auto, true, t.auto)
        .on_hover_text((t.auto_hint)(&size_label(crop::OUT_K_MAX)));
    ui.selectable_value(&mut auto, false, t.fixed);
    match (auto, *out) {
        (true, _) => *out = OutSize::Auto,
        (false, OutSize::Auto) => *out = OutSize::Fixed(crop::OUT_K_MIN),
        _ => {}
    }
    let OutSize::Fixed(k) = out else { return };
    ui.add(
        egui::DragValue::new(k)
            .range(crop::OUT_K_MIN..=crop::OUT_K_MAX)
            .speed(0.1)
            .custom_formatter(|k, _| size_label(k as u32))
            .custom_parser(|s| {
                let w: f64 = s.split(['×', 'x', 'X']).next()?.trim().parse().ok()?;
                Some((w / crop::RW as f64).round())
            }),
    )
    .on_hover_text(t.fixed_hint);
    for preset in [crop::OUT_K_MIN, crop::OUT_K_MAX] {
        if ui.selectable_label(*k == preset, size_label(preset)).clicked() {
            *k = preset;
        }
    }
}

fn angle_picker(ui: &mut egui::Ui, t: &Strings, tab: &mut Tab) {
    ui.label(t.angle);
    let mut deg = tab.angle();
    let angle = egui::DragValue::new(&mut deg)
        .range(-180.0..=180.0)
        .speed(0.1)
        .fixed_decimals(1)
        .suffix("°");
    let resp = ui.add(angle).on_hover_text(t.angle_hint);
    if resp.changed() {
        tab.set_angle(deg);
    }
    let quarter_turns = [("−90°", t.ccw, -90.0), ("+90°", t.cw, 90.0)];
    for (label, hint, delta) in quarter_turns {
        if ui.small_button(label).on_hover_text(hint).clicked() {
            tab.rotate_by(delta);
        }
    }
    if ui.add_enabled(tab.angle() != 0.0, egui::Button::new("0°").small()).clicked() {
        tab.set_angle(0.0);
    }
}

/// "Crop W×H → W×H", highlighted when the result will be upscaled or undersized.
fn crop_summary(ui: &mut egui::Ui, t: &Strings, tab: &Tab, out: OutSize) {
    let px = tab.pixels();
    let (ow, oh) = out.dims(px.k());
    let text = (t.crop)(px.w, px.h, ow, oh);
    let (min_w, _) = crop::frame_size(crop::OUT_K_MIN);
    if tab.has_gaps() {
        let hint = if tab.format == SaveFormat::Png { t.gaps_transparent } else { t.gaps_black };
        ui.colored_label(WARN, text).on_hover_text(hint);
    } else if ow > px.w {
        ui.colored_label(WARN, text)
            .on_hover_text(t.upscaled);
    } else if ow < min_w {
        ui.colored_label(WARN, text)
            .on_hover_text((t.below_min)(&size_label(crop::OUT_K_MIN)));
    } else {
        ui.label(text);
    }
}
