//! Crop-frame geometry, independent of the UI.
//!
//! The frame always has an exact 9:14 aspect ratio. Its state is kept in `f32`
//! image coordinates so dragging stays smooth at any zoom; it is snapped to a
//! whole-pixel `9k × 14k` rectangle only when read via [`Crop::pixels`].

pub const RW: u32 = 9;
pub const RH: u32 = 14;
const ASPECT: f32 = RW as f32 / RH as f32;

/// Smallest and largest `k` for the fixed output size (225×350 ..= 450×700).
pub const OUT_K_MIN: u32 = 25;
pub const OUT_K_MAX: u32 = 50;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Crop {
    pub x: f32,
    pub y: f32,
    pub w: f32,
}

/// Whole-pixel crop rectangle in the original image.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PixelRect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

impl PixelRect {
    pub fn k(&self) -> u32 {
        self.w / RW
    }
}

/// Pixel size of a `9k × 14k` frame.
pub fn frame_size(k: u32) -> (u32, u32) {
    (k * RW, k * RH)
}

/// Whether an image can hold at least one 9×14 frame.
pub fn fits(iw: u32, ih: u32) -> bool {
    iw >= RW && ih >= RH
}

fn max_w(iw: f32, ih: f32) -> f32 {
    iw.min(ih * ASPECT)
}

impl Crop {
    pub fn h(&self) -> f32 {
        self.w / ASPECT
    }

    /// The largest frame that fits, centred in the image.
    pub fn fit(iw: u32, ih: u32) -> Self {
        let (iw, ih) = (iw as f32, ih as f32);
        let w = max_w(iw, ih);
        let h = w / ASPECT;
        Self { x: (iw - w) / 2.0, y: (ih - h) / 2.0, w }
    }

    /// Keeps the frame inside the image and within the size limits.
    pub fn clamp(&mut self, iw: u32, ih: u32) {
        let (iw, ih) = (iw as f32, ih as f32);
        let max = max_w(iw, ih);
        self.w = self.w.clamp((RW as f32).min(max), max);
        self.x = self.x.clamp(0.0, iw - self.w);
        self.y = self.y.clamp(0.0, ih - self.h());
    }

    pub fn move_to(&mut self, x: f32, y: f32, iw: u32, ih: u32) {
        self.x = x;
        self.y = y;
        self.clamp(iw, ih);
    }

    /// Scales the frame by `factor`, keeping its centre where possible.
    pub fn scale(&mut self, factor: f32, iw: u32, ih: u32) {
        let (cx, cy) = (self.x + self.w / 2.0, self.y + self.h() / 2.0);
        let max = max_w(iw as f32, ih as f32);
        self.w = (self.w * factor).clamp((RW as f32).min(max), max);
        self.x = cx - self.w / 2.0;
        self.y = cy - self.h() / 2.0;
        self.clamp(iw, ih);
    }

    /// A frame spanned from the fixed corner `(ax, ay)` towards the pointer `(px, py)`.
    ///
    /// The frame grows by whichever axis the pointer has moved further along,
    /// and never beyond the image edges on the side it grows to.
    pub fn from_anchor(ax: f32, ay: f32, px: f32, py: f32, iw: u32, ih: u32) -> Self {
        let (iw, ih) = (iw as f32, ih as f32);
        let (ax, ay) = (ax.clamp(0.0, iw), ay.clamp(0.0, ih));
        let right = px >= ax;
        let down = py >= ay;
        let room_w = if right { iw - ax } else { ax };
        let room_h = if down { ih - ay } else { ay };
        let w = (px - ax)
            .abs()
            .max((py - ay).abs() * ASPECT)
            .min(room_w)
            .min(room_h * ASPECT)
            .max(RW as f32);
        let h = w / ASPECT;
        let mut c = Self {
            x: if right { ax } else { ax - w },
            y: if down { ay } else { ay - h },
            w,
        };
        c.clamp(iw as u32, ih as u32);
        c
    }

    /// The frame snapped to an exact `9k × 14k` pixel rectangle inside the image.
    pub fn pixels(&self, iw: u32, ih: u32) -> PixelRect {
        let k_max = (iw / RW).min(ih / RH).max(1);
        let k = ((self.w / RW as f32).round() as u32).clamp(1, k_max);
        let (w, h) = frame_size(k);
        let x = (self.x.round().max(0.0) as u32).min(iw.saturating_sub(w));
        let y = (self.y.round().max(0.0) as u32).min(ih.saturating_sub(h));
        PixelRect { x, y, w, h }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutSize {
    /// Keep the crop size, but shrink anything above 450×700 down to 450×700.
    Auto,
    /// Always resize to `9k × 14k`.
    Fixed(u32),
}

impl OutSize {
    pub fn dims(self, crop_k: u32) -> (u32, u32) {
        let k = match self {
            OutSize::Auto => crop_k.min(OUT_K_MAX),
            OutSize::Fixed(k) => k,
        };
        frame_size(k)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exact(p: PixelRect) -> bool {
        p.w % RW == 0 && p.h % RH == 0 && p.w / RW == p.h / RH
    }

    fn inside(p: PixelRect, iw: u32, ih: u32) -> bool {
        p.x + p.w <= iw && p.y + p.h <= ih
    }

    #[test]
    fn fit_is_largest_and_centred() {
        let c = Crop::fit(1920, 1080);
        let p = c.pixels(1920, 1080);
        assert!(exact(p) && inside(p, 1920, 1080));
        assert_eq!(p.h, 1078); // 77 * 14
        let c = Crop::fit(900, 5000);
        assert_eq!(c.pixels(900, 5000).w, 900);
    }

    #[test]
    fn pixels_always_exact_and_inside() {
        for &(iw, ih) in &[(9, 14), (100, 100), (1000, 333), (4000, 6000), (451, 701)] {
            for i in 0..50 {
                let f = i as f32 * 37.3;
                let mut c = Crop { x: f - 200.0, y: f * 1.7 - 300.0, w: f * 3.1 };
                c.clamp(iw, ih);
                let p = c.pixels(iw, ih);
                assert!(exact(p) && inside(p, iw, ih), "{iw}x{ih} {c:?} -> {p:?}");
            }
        }
    }

    #[test]
    fn clamp_keeps_frame_inside_and_minimal() {
        let mut c = Crop { x: -50.0, y: 900.0, w: 1.0 };
        c.clamp(500, 1000);
        assert_eq!(c.w, RW as f32);
        assert!(c.x >= 0.0 && c.y + c.h() <= 1000.0);
        let mut c = Crop { x: 0.0, y: 0.0, w: 10_000.0 };
        c.clamp(500, 1000);
        assert_eq!(c.w, 500.0);
    }

    #[test]
    fn scale_keeps_centre() {
        let mut c = Crop { x: 400.0, y: 400.0, w: 180.0 };
        let centre = (c.x + c.w / 2.0, c.y + c.h() / 2.0);
        c.scale(0.5, 2000, 2000);
        assert_eq!(c.w, 90.0);
        assert!((c.x + c.w / 2.0 - centre.0).abs() < 1e-3);
        assert!((c.y + c.h() / 2.0 - centre.1).abs() < 1e-3);
    }

    #[test]
    fn from_anchor_all_directions() {
        let (iw, ih) = (1000, 1000);
        let c = Crop::from_anchor(500.0, 500.0, 590.0, 510.0, iw, ih);
        assert_eq!((c.x, c.y, c.w), (500.0, 500.0, 90.0));
        let c = Crop::from_anchor(500.0, 500.0, 490.0, 360.0, iw, ih);
        assert_eq!((c.x, c.y, c.w), (410.0, 360.0, 90.0));
        // Limited by the image edge on the growing side.
        let c = Crop::from_anchor(900.0, 100.0, 2000.0, 2000.0, iw, ih);
        assert_eq!((c.x, c.w), (900.0, 100.0));
        let p = c.pixels(iw, ih);
        assert!(exact(p) && inside(p, iw, ih));
    }

    #[test]
    fn output_dims() {
        assert_eq!(OutSize::Auto.dims(26), (234, 364));
        assert_eq!(OutSize::Auto.dims(120), (450, 700));
        assert_eq!(OutSize::Auto.dims(10), (90, 140));
        assert_eq!(OutSize::Fixed(25).dims(120), (225, 350));
    }
}
