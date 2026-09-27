//! One open image: its crop frame, view and the canvas that edits them.

use std::path::PathBuf;
use std::time::Duration;

use eframe::egui::{
    self, Color32, CursorIcon, Painter, PointerButton, Pos2, Rect, Response, Sense, Stroke,
    StrokeKind, TextureHandle, TextureOptions, Vec2, pos2, vec2,
};
use image::RgbaImage;

use crate::crop::{Crop, OutSize, PixelRect};
use crate::i18n::Strings;
use crate::imaging::{self, SaveFormat};
use crate::rotation::Rotation;

const PREVIEW_SIZE: Vec2 = vec2(126.0, 196.0);
/// How long the frame must stay unchanged before the preview is rebuilt, in seconds.
const PREVIEW_DELAY: f64 = 0.12;
/// Pointer distance from a corner that grabs it, in screen pixels.
const HANDLE_RADIUS: f32 = 12.0;
const CANVAS_MARGIN: f32 = 12.0;
const ZOOM_RANGE: std::ops::RangeInclusive<f32> = 0.01..=40.0;
/// Frame scale per wheel scroll point.
const WHEEL_SCALE: f32 = 0.0015;
/// Degrees of rotation per Shift+wheel scroll point.
const WHEEL_ROTATE: f32 = 0.02;
/// Frame colour when it covers an empty corner of the rotated image.
const GAP_COLOR: Color32 = Color32::from_rgb(230, 160, 60);
const FULL_UV: Rect = Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0));

#[derive(Clone, Copy)]
pub enum Drag {
    None,
    /// Grab offset from the frame's top-left corner, in image pixels.
    Move(Vec2),
    /// The fixed corner while resizing or drawing a new frame.
    Span(Pos2),
    Pan,
}

/// Maps between image and screen coordinates.
struct View {
    /// Screen area of the canvas; updated every frame.
    canvas: Rect,
    /// Screen pixels per image pixel; recomputed every frame while `fitted`.
    zoom: f32,
    /// Image point shown at the centre of the canvas.
    center: Pos2,
    fitted: bool,
}

impl View {
    fn to_screen(&self, p: Pos2) -> Pos2 {
        self.canvas.center() + (p - self.center) * self.zoom
    }

    fn to_image(&self, s: Pos2) -> Pos2 {
        self.center + (s - self.canvas.center()) / self.zoom
    }

    fn rect_to_screen(&self, min: Pos2, max: Pos2) -> Rect {
        Rect::from_min_max(self.to_screen(min), self.to_screen(max))
    }

    fn fit(&mut self, image: Vec2) {
        let avail = self.canvas.shrink(CANVAS_MARGIN).size();
        self.zoom = (avail.x / image.x).min(avail.y / image.y);
        self.center = (image / 2.0).to_pos2();
    }

    /// Zooms by `factor`, keeping the image point under `pointer` in place.
    fn zoom_at(&mut self, pointer: Pos2, factor: f32) {
        let anchor = self.to_image(pointer);
        self.zoom = (self.zoom * factor).clamp(*ZOOM_RANGE.start(), *ZOOM_RANGE.end());
        self.center = anchor - (pointer - self.canvas.center()) / self.zoom;
        self.fitted = false;
    }

    fn pan(&mut self, screen_delta: Vec2) {
        self.center -= screen_delta / self.zoom;
        self.fitted = false;
    }
}

struct Preview {
    px: PixelRect,
    rotation: Rotation,
    tex: TextureHandle,
}

pub struct Tab {
    pub id: u64,
    /// File name; `None` for a pasted image.
    file_name: Option<String>,
    pub path: Option<PathBuf>,
    pub format: SaveFormat,
    pub rgba: RgbaImage,
    tex: TextureHandle,
    rotation: Rotation,
    /// Frame in rotated-space coordinates.
    crop: Crop,
    view: View,
    preview: Option<Preview>,
    /// When the frame first differed from the preview.
    changed_at: Option<f64>,
}

impl Tab {
    pub fn new(
        ctx: &egui::Context,
        id: u64,
        rgba: RgbaImage,
        path: Option<PathBuf>,
        format: SaveFormat,
    ) -> Self {
        // Large images are shown downscaled to fit the GPU texture limit;
        // cropping always reads the original pixels.
        let max_side = ctx.input(|i| i.max_texture_side) as u32;
        let tex = texture(ctx, "image", &imaging::fit_within(&rgba, max_side));
        let file_name =
            path.as_ref().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().into_owned());
        let (w, h) = rgba.dimensions();
        Self {
            id,
            file_name,
            path,
            format,
            rgba,
            tex,
            rotation: Rotation::new(0.0, w, h),
            crop: Crop::fit(w, h),
            view: View { canvas: Rect::NOTHING, zoom: 1.0, center: Pos2::ZERO, fitted: true },
            preview: None,
            changed_at: None,
        }
    }

    pub fn name<'a>(&'a self, t: &'a Strings) -> &'a str {
        self.file_name.as_deref().unwrap_or(t.clipboard)
    }

    /// Size of the rotated space the frame lives in.
    fn dims(&self) -> (u32, u32) {
        self.rotation.space_dims()
    }

    fn size(&self) -> Vec2 {
        let (w, h) = self.dims();
        vec2(w as f32, h as f32)
    }

    /// The crop frame snapped to whole pixels.
    pub fn pixels(&self) -> PixelRect {
        let (iw, ih) = self.dims();
        self.crop.pixels(iw, ih)
    }

    /// `<name>_mal.<ext>` next to the source file.
    pub fn default_save_path(&self) -> Option<PathBuf> {
        let src = self.path.as_ref()?;
        let stem = src.file_stem()?.to_string_lossy();
        Some(src.with_file_name(format!("{stem}_mal.{}", self.format.ext())))
    }

    /// Cuts the frame out of the (rotated) image at the size chosen by `out`.
    pub fn render(&self, out: OutSize) -> RgbaImage {
        imaging::render(&self.rgba, self.rotation, self.pixels(), out)
    }

    pub fn angle(&self) -> f32 {
        self.rotation.deg()
    }

    /// Rotates the image to `deg`, keeping the content under the frame and the
    /// view centre in place.
    pub fn set_angle(&mut self, deg: f32) {
        let (w, h) = self.rgba.dimensions();
        let (old, new) = (self.rotation, Rotation::new(deg, w, h));
        if old == new {
            return;
        }
        let remap = |q: Pos2| {
            let (x, y) = new.to_space(old.to_source((q.x, q.y)));
            pos2(x, y)
        };
        let half = vec2(self.crop.w, self.crop.h()) / 2.0;
        let center = remap(pos2(self.crop.x, self.crop.y) + half);
        self.view.center = remap(self.view.center);
        self.rotation = new;
        let (iw, ih) = self.dims();
        self.crop.move_to(center.x - half.x, center.y - half.y, iw, ih);
    }

    pub fn rotate_by(&mut self, deg: f32) {
        self.set_angle(self.angle() + deg);
    }

    /// Whether the frame reaches into an empty corner of the rotated image.
    pub fn has_gaps(&self) -> bool {
        let px = self.pixels();
        let (x0, y0) = (px.x as f32, px.y as f32);
        let (x1, y1) = (x0 + px.w as f32, y0 + px.h as f32);
        // The image is convex, so checking the frame's corners is enough.
        ![(x0, y0), (x1, y0), (x1, y1), (x0, y1)].into_iter().all(|c| self.rotation.covers(c))
    }

    pub fn fit_view(&mut self) {
        self.view.fitted = true;
    }

    /// Moves the snapped frame by `d` image pixels.
    pub fn nudge(&mut self, d: Vec2) {
        let (iw, ih) = self.dims();
        let px = self.pixels();
        self.crop.move_to(px.x as f32 + d.x, px.y as f32 + d.y, iw, ih);
    }

    /// Screen rectangle of the snapped frame.
    fn frame_rect(&self) -> Rect {
        let px = self.pixels();
        self.view.rect_to_screen(
            pos2(px.x as f32, px.y as f32),
            pos2((px.x + px.w) as f32, (px.y + px.h) as f32),
        )
    }

    // ---- canvas --------------------------------------------------------

    pub fn canvas(&mut self, ui: &mut egui::Ui, drag: &mut Drag) {
        let rect = ui.available_rect_before_wrap();
        let resp = ui.allocate_rect(rect, Sense::click_and_drag());
        self.view.canvas = rect;
        if self.view.fitted {
            self.view.fit(self.size());
        }

        self.handle_wheel(ui, &resp);
        self.handle_pointer(ui, &resp, drag);

        let painter = ui.painter_at(rect);
        self.paint(&painter);
        self.refresh_preview(ui.ctx(), matches!(drag, Drag::None));
        self.paint_preview(&painter);
    }

    /// Ctrl+wheel zooms the view around the pointer, Shift+wheel rotates the
    /// image, plain wheel resizes the frame.
    fn handle_wheel(&mut self, ui: &egui::Ui, resp: &Response) {
        let Some(hover) = resp.hover_pos() else { return };
        let (zoom_delta, scroll, shift) =
            ui.input(|i| (i.zoom_delta(), i.smooth_scroll_delta, i.modifiers.shift));
        if zoom_delta != 1.0 {
            self.view.zoom_at(hover, zoom_delta);
        }
        // Some platforms turn Shift+wheel into horizontal scrolling, so take both axes.
        if shift {
            let d = scroll.x + scroll.y;
            if d != 0.0 {
                self.rotate_by(-d * WHEEL_ROTATE);
            }
            return;
        }
        let scroll = scroll.y;
        if scroll != 0.0 {
            let (iw, ih) = self.dims();
            self.crop.scale((-scroll * WHEEL_SCALE).exp(), iw, ih);
        }
    }

    /// Drag a corner to resize, drag inside to move, drag outside to draw a new
    /// frame, click outside to centre the frame there; middle or right drag pans.
    fn handle_pointer(&mut self, ui: &egui::Ui, resp: &Response, drag: &mut Drag) {
        let (iw, ih) = self.dims();
        let frame = self.frame_rect();
        let corners = corners(frame);
        let hit_corner = |p: Pos2| corners.iter().position(|c| c.distance(p) <= HANDLE_RADIUS);

        if resp.drag_started_by(PointerButton::Primary)
            && let Some(origin) = ui.input(|i| i.pointer.press_origin())
        {
            let p = self.view.to_image(origin);
            *drag = if let Some(i) = hit_corner(origin) {
                Drag::Span(self.view.to_image(corners[(i + 2) % 4]))
            } else if frame.contains(origin) {
                Drag::Move(p - pos2(self.crop.x, self.crop.y))
            } else {
                Drag::Span(p.clamp(Pos2::ZERO, self.size().to_pos2()))
            };
        } else if resp.drag_started_by(PointerButton::Middle)
            || resp.drag_started_by(PointerButton::Secondary)
        {
            *drag = Drag::Pan;
        }

        if resp.dragged()
            && let Some(pointer) = resp.interact_pointer_pos()
        {
            let p = self.view.to_image(pointer);
            match *drag {
                Drag::Move(off) => self.crop.move_to(p.x - off.x, p.y - off.y, iw, ih),
                Drag::Span(a) => self.crop = Crop::from_anchor(a.x, a.y, p.x, p.y, iw, ih),
                Drag::Pan => self.view.pan(resp.drag_delta()),
                Drag::None => {}
            }
        }
        if resp.drag_stopped() {
            *drag = Drag::None;
        }

        if resp.clicked()
            && let Some(pos) = resp.interact_pointer_pos()
            && !frame.contains(pos)
        {
            let p = self.view.to_image(pos);
            self.crop.move_to(p.x - self.crop.w / 2.0, p.y - self.crop.h() / 2.0, iw, ih);
        }

        if let Some(hover) = resp.hover_pos() {
            let icon = match (*drag, hit_corner(hover)) {
                (Drag::Pan, _) => CursorIcon::Grabbing,
                (Drag::Move(_), _) => CursorIcon::Move,
                (Drag::Span(_), _) => CursorIcon::Crosshair,
                (Drag::None, Some(0 | 2)) => CursorIcon::ResizeNwSe,
                (Drag::None, Some(_)) => CursorIcon::ResizeNeSw,
                (Drag::None, None) if frame.contains(hover) => CursorIcon::Move,
                (Drag::None, None) => CursorIcon::Crosshair,
            };
            ui.ctx().set_cursor_icon(icon);
        }
    }

    fn paint(&self, painter: &Painter) {
        // The image as a rotated quad: GPU rotation keeps angle changes instant.
        let (w, h) = self.rgba.dimensions();
        let (w, h) = (w as f32, h as f32);
        let mut mesh = egui::Mesh::with_texture(self.tex.id());
        let src_corners = [(0.0, 0.0), (w, 0.0), (w, h), (0.0, h)];
        let uvs = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];
        for (p, (u, v)) in src_corners.into_iter().zip(uvs) {
            let (x, y) = self.rotation.to_space(p);
            mesh.vertices.push(egui::epaint::Vertex {
                pos: self.view.to_screen(pos2(x, y)),
                uv: pos2(u, v),
                color: Color32::WHITE,
            });
        }
        mesh.add_triangle(0, 1, 2);
        mesh.add_triangle(0, 2, 3);
        painter.add(egui::Shape::mesh(mesh));

        // Dim everything outside the frame.
        let frame = self.frame_rect();
        let area = self.view.canvas;
        let dim = Color32::from_black_alpha(160);
        for r in [
            Rect::from_x_y_ranges(area.x_range(), area.top()..=frame.top()),
            Rect::from_x_y_ranges(area.x_range(), frame.bottom()..=area.bottom()),
            Rect::from_x_y_ranges(area.left()..=frame.left(), frame.y_range()),
            Rect::from_x_y_ranges(frame.right()..=area.right(), frame.y_range()),
        ] {
            painter.rect_filled(r, 0.0, dim);
        }

        // Rule-of-thirds grid.
        let thin = Stroke::new(1.0, Color32::from_white_alpha(70));
        for t in [1.0 / 3.0, 2.0 / 3.0] {
            let x = frame.left() + frame.width() * t;
            let y = frame.top() + frame.height() * t;
            painter.line_segment([pos2(x, frame.top()), pos2(x, frame.bottom())], thin);
            painter.line_segment([pos2(frame.left(), y), pos2(frame.right(), y)], thin);
        }

        let color = if self.has_gaps() { GAP_COLOR } else { Color32::WHITE };
        painter.rect_stroke(frame, 0.0, Stroke::new(1.5, color), StrokeKind::Outside);
        for c in corners(frame) {
            painter.rect_filled(Rect::from_center_size(c, Vec2::splat(8.0)), 1.0, color);
        }
    }

    /// Rebuilds the result preview once the frame has changed and settled.
    fn refresh_preview(&mut self, ctx: &egui::Context, idle: bool) {
        let px = self.pixels();
        let rotation = self.rotation;
        if self.preview.as_ref().is_some_and(|p| p.px == px && p.rotation == rotation) {
            return;
        }
        let now = ctx.input(|i| i.time);
        let since = *self.changed_at.get_or_insert(now);
        if idle && now - since > PREVIEW_DELAY {
            let (w, h) = (PREVIEW_SIZE.x as u32, PREVIEW_SIZE.y as u32);
            let thumb = imaging::thumbnail(&self.rgba, rotation, px, w, h);
            self.preview = Some(Preview { px, rotation, tex: texture(ctx, "preview", &thumb) });
            self.changed_at = None;
        } else {
            ctx.request_repaint_after(Duration::from_secs_f64(PREVIEW_DELAY + 0.01));
        }
    }

    fn paint_preview(&self, painter: &Painter) {
        let Some(preview) = &self.preview else { return };
        let corner = self.view.canvas.right_bottom() - Vec2::splat(CANVAS_MARGIN);
        let r = Rect::from_min_size(corner - PREVIEW_SIZE, PREVIEW_SIZE);
        painter.rect_filled(r.expand(3.0), 3.0, Color32::from_black_alpha(200));
        painter.image(preview.tex.id(), r, FULL_UV, Color32::WHITE);
    }
}

/// Corners clockwise from the top-left, so opposite corners are two apart.
fn corners(r: Rect) -> [Pos2; 4] {
    [r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom()]
}

fn texture(ctx: &egui::Context, name: &str, img: &RgbaImage) -> TextureHandle {
    let size = [img.width() as usize, img.height() as usize];
    let color = egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw());
    ctx.load_texture(name, color, TextureOptions::LINEAR)
}
