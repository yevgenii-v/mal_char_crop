//! Rotation of the image under the always-upright crop frame, independent of the UI.
//!
//! The crop frame lives in "space" coordinates: the axis-aligned bounding box of
//! the source image rotated by the angle about its centre. At 0° space and source
//! coordinates are identical.

/// Tolerance for [`Rotation::covers`], in pixels, to absorb rounding of the space size.
const COVER_EPS: f32 = 0.5;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rotation {
    /// Clockwise on screen, normalised to `(-180, 180]`.
    deg: f32,
    sin: f32,
    cos: f32,
    src: (f32, f32),
    space: (f32, f32),
}

impl Rotation {
    pub fn new(deg: f32, w: u32, h: u32) -> Self {
        let deg = normalize(deg);
        // Exact values for right angles, so they map pixels onto pixels.
        let (sin, cos) = match deg {
            0.0 => (0.0, 1.0),
            90.0 => (1.0, 0.0),
            -90.0 => (-1.0, 0.0),
            180.0 => (0.0, -1.0),
            _ => deg.to_radians().sin_cos(),
        };
        let (w, h) = (w as f32, h as f32);
        let space = (w * cos.abs() + h * sin.abs(), w * sin.abs() + h * cos.abs());
        Self { deg, sin, cos, src: (w, h), space }
    }

    pub fn deg(&self) -> f32 {
        self.deg
    }

    pub fn is_identity(&self) -> bool {
        self.deg == 0.0
    }

    /// Whole-pixel size of the rotated bounding box.
    pub fn space_dims(&self) -> (u32, u32) {
        (self.space.0.round() as u32, self.space.1.round() as u32)
    }

    pub fn to_space(&self, (x, y): (f32, f32)) -> (f32, f32) {
        let (dx, dy) = (x - self.src.0 / 2.0, y - self.src.1 / 2.0);
        (
            dx * self.cos - dy * self.sin + self.space.0 / 2.0,
            dx * self.sin + dy * self.cos + self.space.1 / 2.0,
        )
    }

    pub fn to_source(&self, (x, y): (f32, f32)) -> (f32, f32) {
        let (dx, dy) = (x - self.space.0 / 2.0, y - self.space.1 / 2.0);
        (
            dx * self.cos + dy * self.sin + self.src.0 / 2.0,
            -dx * self.sin + dy * self.cos + self.src.1 / 2.0,
        )
    }

    /// Whether the space point lies on the image rather than in an empty corner.
    pub fn covers(&self, q: (f32, f32)) -> bool {
        let (x, y) = self.to_source(q);
        (-COVER_EPS..=self.src.0 + COVER_EPS).contains(&x)
            && (-COVER_EPS..=self.src.1 + COVER_EPS).contains(&y)
    }
}

fn normalize(deg: f32) -> f32 {
    let d = deg.rem_euclid(360.0);
    if d > 180.0 { d - 360.0 } else { d }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: (f32, f32), b: (f32, f32)) -> bool {
        (a.0 - b.0).abs() < 1e-2 && (a.1 - b.1).abs() < 1e-2
    }

    #[test]
    fn normalises_angle() {
        assert_eq!(Rotation::new(-180.0, 10, 10).deg(), 180.0);
        assert_eq!(Rotation::new(270.0, 10, 10).deg(), -90.0);
        assert_eq!(Rotation::new(725.0, 10, 10).deg(), 5.0);
    }

    #[test]
    fn identity_and_right_angles() {
        let r = Rotation::new(0.0, 300, 200);
        assert_eq!(r.space_dims(), (300, 200));
        assert_eq!(r.to_space((12.0, 34.0)), (12.0, 34.0));
        let r = Rotation::new(90.0, 300, 200);
        assert_eq!(r.space_dims(), (200, 300));
        // Clockwise: the top-left source corner ends up top-right.
        assert_eq!(r.to_space((0.0, 0.0)), (200.0, 0.0));
    }

    #[test]
    fn round_trip() {
        for deg in [-170.0, -33.3, 7.5, 45.0, 123.0] {
            let r = Rotation::new(deg, 640, 480);
            for p in [(0.0, 0.0), (640.0, 480.0), (100.0, 400.0), (320.0, 240.0)] {
                assert!(close(r.to_source(r.to_space(p)), p), "{deg}° {p:?}");
            }
        }
    }

    #[test]
    fn corners_touch_bounding_box() {
        let r = Rotation::new(30.0, 400, 300);
        let (sw, sh) = r.space;
        for p in [(0.0, 0.0), (400.0, 0.0), (400.0, 300.0), (0.0, 300.0)] {
            let (x, y) = r.to_space(p);
            assert!((-1e-2..=sw + 1e-2).contains(&x) && (-1e-2..=sh + 1e-2).contains(&y));
        }
        assert!(r.covers((sw / 2.0, sh / 2.0)));
        assert!(!r.covers((0.0, 0.0)));
    }
}
