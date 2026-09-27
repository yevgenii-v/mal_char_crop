//! Reading images from the system clipboard.

use std::path::PathBuf;

use image::RgbaImage;

use crate::i18n::Strings;

pub enum Pasted {
    Image(RgbaImage),
    /// Files copied in a file manager.
    Files(Vec<PathBuf>),
}

pub enum PasteError {
    Unavailable(arboard::Error),
    BadImage,
    Empty,
}

impl PasteError {
    pub fn message(&self, t: &Strings) -> String {
        match self {
            Self::Unavailable(e) => (t.clipboard_unavailable)(e),
            Self::BadImage => t.clipboard_bad_image.into(),
            Self::Empty => t.clipboard_empty.into(),
        }
    }
}

pub fn read() -> Result<Pasted, PasteError> {
    let mut cb = arboard::Clipboard::new().map_err(PasteError::Unavailable)?;
    if let Ok(data) = cb.get_image() {
        let (w, h) = (data.width as u32, data.height as u32);
        return RgbaImage::from_raw(w, h, data.bytes.into_owned())
            .map(Pasted::Image)
            .ok_or(PasteError::BadImage);
    }
    let text = cb.get_text().unwrap_or_default();
    let files: Vec<_> = parse_paths(&text).filter(|p| p.is_file()).collect();
    if files.is_empty() {
        return Err(PasteError::Empty);
    }
    Ok(Pasted::Files(files))
}

/// File managers put copied files on the clipboard as paths or `file://` URIs, one per line.
fn parse_paths(text: &str) -> impl Iterator<Item = PathBuf> + '_ {
    text.lines().map(str::trim).filter(|l| !l.is_empty()).map(|line| {
        match line.strip_prefix("file://") {
            Some(uri) => uri_to_path(uri),
            None => PathBuf::from(line),
        }
    })
}

#[cfg(unix)]
fn uri_to_path(uri: &str) -> PathBuf {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;
    PathBuf::from(OsString::from_vec(percent_decode(uri)))
}

/// `file:///C:/x.png` carries a slash before the drive letter.
#[cfg(not(unix))]
fn uri_to_path(uri: &str) -> PathBuf {
    let path = String::from_utf8_lossy(&percent_decode(uri)).into_owned();
    match path.as_bytes() {
        [b'/', drive, b':', ..] if drive.is_ascii_alphabetic() => PathBuf::from(&path[1..]),
        _ => PathBuf::from(path),
    }
}

fn percent_decode(s: &str) -> Vec<u8> {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%'
            && let Some(v) = s.get(i + 1..i + 3).and_then(|h| u8::from_str_radix(h, 16).ok())
        {
            out.push(v);
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_escapes() {
        assert_eq!(percent_decode("a%20b%zz%4"), b"a b%zz%4");
        assert_eq!(percent_decode("%D0%AF"), "Я".as_bytes());
    }

    #[test]
    fn parses_uris_and_plain_paths() {
        let text = "copy\nfile:///home/u/%D0%AF%201.png\n\n/tmp/50%25.png\n";
        let paths: Vec<_> = parse_paths(text).collect();
        assert_eq!(
            paths,
            [
                PathBuf::from("copy"),
                PathBuf::from("/home/u/Я 1.png"),
                // Plain paths are taken literally.
                PathBuf::from("/tmp/50%25.png"),
            ]
        );
    }
}
