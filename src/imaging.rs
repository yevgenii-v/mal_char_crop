//! Decoding, encoding and pixel operations, independent of the UI.

use std::borrow::Cow;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use image::codecs::jpeg::JpegEncoder;
use image::{
    DynamicImage, ImageDecoder, ImageFormat, ImageReader, ImageResult, Rgba, RgbaImage, imageops,
};

use crate::crop::{OutSize, PixelRect};
use crate::rotation::Rotation;

/// 4:4:4 at maximum quality: the closest JPEG gets to lossless after a crop/resize.
const JPEG_QUALITY: u8 = 100;

/// Extensions offered in the open dialog.
pub const OPEN_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "jfif", "png", "webp", "gif", "bmp", "ico", "tif", "tiff", "tga", "qoi",
    "dds", "hdr", "exr", "pnm", "pbm", "pgm", "ppm", "pam", "ff",
];

/// Output format: JPEG sources stay JPEG, everything else becomes lossless PNG.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SaveFormat {
    Jpeg,
    Png,
}

impl SaveFormat {
    pub fn of_source(format: Option<ImageFormat>) -> Self {
        match format {
            Some(ImageFormat::Jpeg) => Self::Jpeg,
            _ => Self::Png,
        }
    }

    /// Format implied by a chosen file name, if it is one we write.
    pub fn of_path(path: &Path) -> Option<Self> {
        match path.extension()?.to_string_lossy().to_lowercase().as_str() {
            "jpg" | "jpeg" => Some(Self::Jpeg),
            "png" => Some(Self::Png),
            _ => None,
        }
    }

    pub fn ext(self) -> &'static str {
        match self {
            Self::Jpeg => "jpg",
            Self::Png => "png",
        }
    }
}

/// Reads an image (format detected from its contents), applying its EXIF orientation.
pub fn load(path: &Path) -> ImageResult<(RgbaImage, SaveFormat)> {
    let reader = ImageReader::open(path)?.with_guessed_format()?;
    let format = reader.format();
    let mut decoder = reader.into_decoder()?;
    let orientation = decoder.orientation()?;
    let mut img = DynamicImage::from_decoder(decoder)?;
    img.apply_orientation(orientation);
    Ok((img.into_rgba8(), SaveFormat::of_source(format)))
}

pub fn save(path: &Path, img: RgbaImage, format: SaveFormat) -> ImageResult<()> {
    match format {
        SaveFormat::Jpeg => {
            let out = BufWriter::new(File::create(path)?);
            let rgb = DynamicImage::ImageRgba8(img).into_rgb8();
            JpegEncoder::new_with_quality(out, JPEG_QUALITY).encode_image(&rgb)
        }
        SaveFormat::Png => img.save_with_format(path, ImageFormat::Png),
    }
}

/// Cuts `px` (in rotated space) out of `src` and scales it to the size chosen by `out`.
pub fn render(src: &RgbaImage, rot: Rotation, px: PixelRect, out: OutSize) -> RgbaImage {
    let (w, h) = out.dims(px.k());
    if rot.is_identity() {
        let cut = imageops::crop_imm(src, px.x, px.y, px.w, px.h);
        return if (w, h) == (px.w, px.h) {
            cut.to_image()
        } else {
            imageops::resize(&*cut, w, h, imageops::FilterType::Lanczos3)
        };
    }
    // Resample at full crop resolution first so the final downscale stays Lanczos.
    let cut = rotated_cut(src, rot, px, px.w, px.h);
    if (w, h) == (px.w, px.h) {
        cut
    } else {
        imageops::resize(&cut, w, h, imageops::FilterType::Lanczos3)
    }
}

/// A quick, low-quality downscale of `px` (in rotated space) that fits in `max_w × max_h`.
pub fn thumbnail(
    src: &RgbaImage,
    rot: Rotation,
    px: PixelRect,
    max_w: u32,
    max_h: u32,
) -> RgbaImage {
    if rot.is_identity() {
        let cut = imageops::crop_imm(src, px.x, px.y, px.w, px.h);
        return imageops::thumbnail(&*cut, max_w, max_h);
    }
    // Supersample a little to limit aliasing, then box-downscale.
    let s = (4.0 * max_w as f32 / px.w as f32).min(4.0 * max_h as f32 / px.h as f32).min(1.0);
    let (w, h) = (((px.w as f32 * s) as u32).max(1), ((px.h as f32 * s) as u32).max(1));
    imageops::thumbnail(&rotated_cut(src, rot, px, w, h), max_w, max_h)
}

/// Samples the space rectangle `px` into a `w × h` image; empty corners become transparent.
fn rotated_cut(src: &RgbaImage, rot: Rotation, px: PixelRect, w: u32, h: u32) -> RgbaImage {
    let (sx, sy) = (px.w as f32 / w as f32, px.h as f32 / h as f32);
    RgbaImage::from_fn(w, h, |i, j| {
        let q = (px.x as f32 + (i as f32 + 0.5) * sx, px.y as f32 + (j as f32 + 0.5) * sy);
        let (x, y) = rot.to_source(q);
        // Pixel centres sit at half-integers in source coordinates.
        sample(src, x - 0.5, y - 0.5)
    })
}

/// Bilinear sample at pixel-centre coordinates, clamped within half a pixel of the edge.
fn sample(src: &RgbaImage, x: f32, y: f32) -> Rgba<u8> {
    let (w, h) = (src.width() as f32, src.height() as f32);
    if !(-0.5..=w - 0.5).contains(&x) || !(-0.5..=h - 0.5).contains(&y) {
        return Rgba([0; 4]);
    }
    imageops::interpolate_bilinear(src, x.clamp(0.0, w - 1.0), y.clamp(0.0, h - 1.0))
        .unwrap_or(Rgba([0; 4]))
}

/// `img` shrunk so that neither side exceeds `max_side`, or borrowed as is if it already fits.
pub fn fit_within(img: &RgbaImage, max_side: u32) -> Cow<'_, RgbaImage> {
    let (w, h) = img.dimensions();
    if w.max(h) <= max_side {
        return Cow::Borrowed(img);
    }
    let s = max_side as f32 / w.max(h) as f32;
    Cow::Owned(imageops::thumbnail(img, (w as f32 * s) as u32, (h as f32 * s) as u32))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_from_path() {
        assert_eq!(SaveFormat::of_path(Path::new("a/b.JPEG")), Some(SaveFormat::Jpeg));
        assert_eq!(SaveFormat::of_path(Path::new("b.png")), Some(SaveFormat::Png));
        assert_eq!(SaveFormat::of_path(Path::new("b.webp")), None);
        assert_eq!(SaveFormat::of_path(Path::new("b")), None);
    }

    #[test]
    fn render_resizes_only_when_needed() {
        let src = RgbaImage::new(900, 1400);
        let rot = Rotation::new(0.0, 900, 1400);
        let px = PixelRect { x: 0, y: 0, w: 900, h: 1400 };
        assert_eq!(render(&src, rot, px, OutSize::Auto).dimensions(), (450, 700));
        let px = PixelRect { x: 9, y: 14, w: 90, h: 140 };
        assert_eq!(render(&src, rot, px, OutSize::Auto).dimensions(), (90, 140));
        assert_eq!(render(&src, rot, px, OutSize::Fixed(25)).dimensions(), (225, 350));
    }

    #[test]
    fn rotated_render_matches_right_angle() {
        // 90° clockwise: the source's left column becomes the result's top row.
        let src = RgbaImage::from_fn(28, 18, |x, _| Rgba([x as u8, 0, 0, 255]));
        let rot = Rotation::new(90.0, 28, 18);
        let px = PixelRect { x: 0, y: 0, w: 18, h: 28 };
        let img = render(&src, rot, px, OutSize::Fixed(2));
        assert_eq!(img.dimensions(), (18, 28));
        let exact = rotated_cut(&src, rot, px, 18, 28);
        assert_eq!(exact.get_pixel(5, 0).0, [0, 0, 0, 255]);
        assert_eq!(exact.get_pixel(5, 27).0, [27, 0, 0, 255]);
    }

    #[test]
    fn empty_corners_are_transparent() {
        let src = RgbaImage::from_pixel(100, 100, Rgba([255; 4]));
        let rot = Rotation::new(45.0, 100, 100);
        let px = PixelRect { x: 0, y: 0, w: 90, h: 140 };
        let cut = rotated_cut(&src, rot, px, 9, 14);
        assert_eq!(cut.get_pixel(0, 0).0[3], 0);
        assert_eq!(cut.get_pixel(4, 7).0, [255; 4]);
    }

    #[test]
    fn fit_within_borrows_small_images() {
        let img = RgbaImage::new(100, 50);
        assert!(matches!(fit_within(&img, 100), Cow::Borrowed(_)));
        assert_eq!(fit_within(&img, 10).dimensions(), (10, 5));
    }
}
