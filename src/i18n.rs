//! UI translations. Latin and Cyrillic scripts only: egui has no complex text
//! shaping, so Bengali, Devanagari or Arabic would render incorrectly.

use std::fmt::Display;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lang {
    En,
    Id,
    Pt,
    De,
    Es,
    Fil,
    Ms,
    Uk,
}

impl Lang {
    pub const ALL: [Lang; 8] =
        [Lang::En, Lang::Id, Lang::Pt, Lang::De, Lang::Es, Lang::Fil, Lang::Ms, Lang::Uk];

    /// The language's name in itself, for the picker.
    pub fn native_name(self) -> &'static str {
        match self {
            Lang::En => "English",
            Lang::Id => "Bahasa Indonesia",
            Lang::Pt => "Português (Brasil)",
            Lang::De => "Deutsch",
            Lang::Es => "Español",
            Lang::Fil => "Filipino",
            Lang::Ms => "Bahasa Melayu",
            Lang::Uk => "Українська",
        }
    }

    /// Language from `MAL_CROP_LANG` or the POSIX locale variables; English if unknown.
    pub fn detect() -> Self {
        let var = |name| std::env::var(name).ok().filter(|v| !v.is_empty());
        if let Some(lang) = var("MAL_CROP_LANG").and_then(|v| Self::from_locale(&v)) {
            return lang;
        }
        // gettext precedence: LANGUAGE (a list) wins unless the locale is C/POSIX.
        let Some(locale) = ["LC_ALL", "LC_MESSAGES", "LANG"].into_iter().find_map(var) else {
            return Lang::En;
        };
        let posix = locale == "C" || locale == "POSIX" || locale.starts_with("C.");
        var("LANGUAGE")
            .filter(|_| !posix)
            .and_then(|list| list.split(':').find_map(Self::from_locale))
            .or_else(|| Self::from_locale(&locale))
            .unwrap_or(Lang::En)
    }

    /// Parses `uk_UA.UTF-8`, `pt-BR`, `de@euro` and the like.
    fn from_locale(locale: &str) -> Option<Self> {
        let code = locale.split(['_', '-', '.', '@']).next()?.to_ascii_lowercase();
        Some(match code.as_str() {
            "en" => Lang::En,
            "id" | "in" => Lang::Id,
            "pt" => Lang::Pt,
            "de" => Lang::De,
            "es" => Lang::Es,
            "fil" | "tl" => Lang::Fil,
            "ms" => Lang::Ms,
            "uk" => Lang::Uk,
            _ => return None,
        })
    }

    pub fn tr(self) -> &'static Strings {
        match self {
            Lang::En => &EN,
            Lang::Id => &ID,
            Lang::Pt => &PT,
            Lang::De => &DE,
            Lang::Es => &ES,
            Lang::Fil => &FIL,
            Lang::Ms => &MS,
            Lang::Uk => &UK,
        }
    }
}

type Arg<'a> = &'a dyn Display;

pub struct Strings {
    pub welcome: &'static str,
    pub help: &'static str,
    pub open_failed: fn(path: Arg, err: Arg) -> String,
    pub too_small: fn(w: u32, h: u32) -> String,
    pub tab_closed: &'static str,
    pub saved: fn(w: u32, h: u32, path: Arg) -> String,
    pub save_failed: fn(err: Arg) -> String,
    pub open: &'static str,
    pub paste: &'static str,
    pub save: &'static str,
    pub save_as: &'static str,
    pub close_tab: &'static str,
    pub drop_here: &'static str,
    pub language: &'static str,
    pub size: &'static str,
    pub auto: &'static str,
    pub auto_hint: fn(max: Arg) -> String,
    pub fixed: &'static str,
    pub fixed_hint: &'static str,
    pub angle: &'static str,
    pub angle_hint: &'static str,
    pub ccw: &'static str,
    pub cw: &'static str,
    pub crop: fn(w: u32, h: u32, ow: u32, oh: u32) -> String,
    pub gaps_transparent: &'static str,
    pub gaps_black: &'static str,
    pub upscaled: &'static str,
    pub below_min: fn(min: Arg) -> String,
    pub images: &'static str,
    pub clipboard: &'static str,
    pub clipboard_unavailable: fn(err: Arg) -> String,
    pub clipboard_bad_image: &'static str,
    pub clipboard_empty: &'static str,
}

static EN: Strings = Strings {
    welcome: "Open an image: Ctrl+O, drop a file into the window, or Ctrl+V",
    help: "Wheel — frame size · Ctrl+wheel — zoom · Shift+wheel, [ ] — rotate · \
        Right/middle drag — pan · F — fit · arrows — ±1px",
    open_failed: |p, e| format!("Could not open {p}: {e}"),
    too_small: |w, h| format!("Image {w}×{h} is too small for 9:14"),
    tab_closed: "The tab has already been closed",
    saved: |w, h, p| format!("Saved {w}×{h} → {p}"),
    save_failed: |e| format!("Save failed: {e}"),
    open: "Open",
    paste: "Paste",
    save: "Save",
    save_as: "Save as…",
    close_tab: "Close (Ctrl+W)",
    drop_here: "Drop an image here",
    language: "Language",
    size: "Size:",
    auto: "Auto",
    auto_hint: |max| format!("As cropped, but no larger than {max}"),
    fixed: "Fixed",
    fixed_hint: "Width is a multiple of 9, height a multiple of 14",
    angle: "Angle:",
    angle_hint: "Rotate the image clockwise",
    ccw: "Counterclockwise",
    cw: "Clockwise",
    crop: |w, h, ow, oh| format!("Crop {w}×{h} → {ow}×{oh}"),
    gaps_transparent: "The frame extends past the rotated image — corners will be transparent",
    gaps_black: "The frame extends past the rotated image — corners will be black",
    upscaled: "The crop is smaller than the output size — it will be upscaled",
    below_min: |min| format!("Smaller than {min}"),
    images: "Images",
    clipboard: "clipboard",
    clipboard_unavailable: |e| format!("Clipboard unavailable: {e}"),
    clipboard_bad_image: "Invalid image in the clipboard",
    clipboard_empty: "No image in the clipboard",
};

static ID: Strings = Strings {
    welcome: "Buka gambar: Ctrl+O, seret berkas ke jendela, atau Ctrl+V",
    help: "Roda — ukuran bingkai · Ctrl+roda — zoom · Shift+roda, [ ] — putar · \
        Seret klik kanan/tengah — geser · F — paskan · panah — ±1px",
    open_failed: |p, e| format!("Gagal membuka {p}: {e}"),
    too_small: |w, h| format!("Gambar {w}×{h} terlalu kecil untuk 9:14"),
    tab_closed: "Tab sudah ditutup",
    saved: |w, h, p| format!("Tersimpan {w}×{h} → {p}"),
    save_failed: |e| format!("Gagal menyimpan: {e}"),
    open: "Buka",
    paste: "Tempel",
    save: "Simpan",
    save_as: "Simpan sebagai…",
    close_tab: "Tutup (Ctrl+W)",
    drop_here: "Seret gambar ke sini",
    language: "Bahasa",
    size: "Ukuran:",
    auto: "Otomatis",
    auto_hint: |max| format!("Sesuai potongan, maksimal {max}"),
    fixed: "Tetap",
    fixed_hint: "Lebar kelipatan 9, tinggi kelipatan 14",
    angle: "Sudut:",
    angle_hint: "Putar gambar searah jarum jam",
    ccw: "Berlawanan arah jarum jam",
    cw: "Searah jarum jam",
    crop: |w, h, ow, oh| format!("Potong {w}×{h} → {ow}×{oh}"),
    gaps_transparent: "Bingkai melewati tepi gambar yang diputar — sudutnya akan transparan",
    gaps_black: "Bingkai melewati tepi gambar yang diputar — sudutnya akan hitam",
    upscaled: "Potongan lebih kecil dari ukuran keluaran — akan diperbesar",
    below_min: |min| format!("Lebih kecil dari {min}"),
    images: "Gambar",
    clipboard: "papan klip",
    clipboard_unavailable: |e| format!("Papan klip tidak tersedia: {e}"),
    clipboard_bad_image: "Gambar di papan klip tidak valid",
    clipboard_empty: "Tidak ada gambar di papan klip",
};

static PT: Strings = Strings {
    welcome: "Abra uma imagem: Ctrl+O, arraste um arquivo para a janela ou Ctrl+V",
    help: "Roda — tamanho do quadro · Ctrl+roda — zoom · Shift+roda, [ ] — girar · \
        Arrastar com botão direito/do meio — mover · F — ajustar · setas — ±1px",
    open_failed: |p, e| format!("Não foi possível abrir {p}: {e}"),
    too_small: |w, h| format!("A imagem {w}×{h} é pequena demais para 9:14"),
    tab_closed: "A aba já foi fechada",
    saved: |w, h, p| format!("Salvo {w}×{h} → {p}"),
    save_failed: |e| format!("Erro ao salvar: {e}"),
    open: "Abrir",
    paste: "Colar",
    save: "Salvar",
    save_as: "Salvar como…",
    close_tab: "Fechar (Ctrl+W)",
    drop_here: "Arraste uma imagem para cá",
    language: "Idioma",
    size: "Tamanho:",
    auto: "Automático",
    auto_hint: |max| format!("Como recortado, mas no máximo {max}"),
    fixed: "Fixo",
    fixed_hint: "Largura múltipla de 9, altura múltipla de 14",
    angle: "Ângulo:",
    angle_hint: "Girar a imagem no sentido horário",
    ccw: "Sentido anti-horário",
    cw: "Sentido horário",
    crop: |w, h, ow, oh| format!("Recorte {w}×{h} → {ow}×{oh}"),
    gaps_transparent: "O quadro ultrapassa a borda da imagem girada — os cantos ficarão transparentes",
    gaps_black: "O quadro ultrapassa a borda da imagem girada — os cantos ficarão pretos",
    upscaled: "O recorte é menor que o tamanho de saída — será ampliado",
    below_min: |min| format!("Menor que {min}"),
    images: "Imagens",
    clipboard: "área de transferência",
    clipboard_unavailable: |e| format!("Área de transferência indisponível: {e}"),
    clipboard_bad_image: "Imagem inválida na área de transferência",
    clipboard_empty: "Não há imagem na área de transferência",
};

static DE: Strings = Strings {
    welcome: "Bild öffnen: Ctrl+O, Datei ins Fenster ziehen oder Ctrl+V",
    help: "Mausrad — Rahmengröße · Ctrl+Mausrad — Zoom · Shift+Mausrad, [ ] — Drehen · \
        Rechts/Mitte ziehen — Verschieben · F — Einpassen · Pfeiltasten — ±1px",
    open_failed: |p, e| format!("{p} konnte nicht geöffnet werden: {e}"),
    too_small: |w, h| format!("Bild {w}×{h} ist zu klein für 9:14"),
    tab_closed: "Der Tab wurde bereits geschlossen",
    saved: |w, h, p| format!("Gespeichert {w}×{h} → {p}"),
    save_failed: |e| format!("Fehler beim Speichern: {e}"),
    open: "Öffnen",
    paste: "Einfügen",
    save: "Speichern",
    save_as: "Speichern unter…",
    close_tab: "Schließen (Ctrl+W)",
    drop_here: "Bild hierher ziehen",
    language: "Sprache",
    size: "Größe:",
    auto: "Auto",
    auto_hint: |max| format!("Wie zugeschnitten, aber höchstens {max}"),
    fixed: "Fest",
    fixed_hint: "Breite ein Vielfaches von 9, Höhe ein Vielfaches von 14",
    angle: "Winkel:",
    angle_hint: "Bild im Uhrzeigersinn drehen",
    ccw: "Gegen den Uhrzeigersinn",
    cw: "Im Uhrzeigersinn",
    crop: |w, h, ow, oh| format!("Zuschnitt {w}×{h} → {ow}×{oh}"),
    gaps_transparent: "Der Rahmen ragt über das gedrehte Bild hinaus — die Ecken werden transparent",
    gaps_black: "Der Rahmen ragt über das gedrehte Bild hinaus — die Ecken werden schwarz",
    upscaled: "Der Zuschnitt ist kleiner als die Ausgabegröße — er wird hochskaliert",
    below_min: |min| format!("Kleiner als {min}"),
    images: "Bilder",
    clipboard: "Zwischenablage",
    clipboard_unavailable: |e| format!("Zwischenablage nicht verfügbar: {e}"),
    clipboard_bad_image: "Ungültiges Bild in der Zwischenablage",
    clipboard_empty: "Kein Bild in der Zwischenablage",
};

static ES: Strings = Strings {
    welcome: "Abre una imagen: Ctrl+O, arrastra un archivo a la ventana o Ctrl+V",
    help: "Rueda — tamaño del marco · Ctrl+rueda — zoom · Shift+rueda, [ ] — girar · \
        Arrastrar con clic derecho/central — desplazar · F — ajustar · flechas — ±1px",
    open_failed: |p, e| format!("No se pudo abrir {p}: {e}"),
    too_small: |w, h| format!("La imagen {w}×{h} es demasiado pequeña para 9:14"),
    tab_closed: "La pestaña ya se cerró",
    saved: |w, h, p| format!("Guardado {w}×{h} → {p}"),
    save_failed: |e| format!("Error al guardar: {e}"),
    open: "Abrir",
    paste: "Pegar",
    save: "Guardar",
    save_as: "Guardar como…",
    close_tab: "Cerrar (Ctrl+W)",
    drop_here: "Arrastra una imagen aquí",
    language: "Idioma",
    size: "Tamaño:",
    auto: "Auto",
    auto_hint: |max| format!("Como se recortó, pero no más de {max}"),
    fixed: "Fijo",
    fixed_hint: "Ancho múltiplo de 9, alto múltiplo de 14",
    angle: "Ángulo:",
    angle_hint: "Girar la imagen en sentido horario",
    ccw: "Sentido antihorario",
    cw: "Sentido horario",
    crop: |w, h, ow, oh| format!("Recorte {w}×{h} → {ow}×{oh}"),
    gaps_transparent: "El marco sobrepasa el borde de la imagen girada: las esquinas quedarán transparentes",
    gaps_black: "El marco sobrepasa el borde de la imagen girada: las esquinas quedarán negras",
    upscaled: "El recorte es menor que el tamaño de salida: se ampliará",
    below_min: |min| format!("Menor que {min}"),
    images: "Imágenes",
    clipboard: "portapapeles",
    clipboard_unavailable: |e| format!("Portapapeles no disponible: {e}"),
    clipboard_bad_image: "Imagen no válida en el portapapeles",
    clipboard_empty: "No hay ninguna imagen en el portapapeles",
};

static FIL: Strings = Strings {
    welcome: "Magbukas ng larawan: Ctrl+O, i-drag ang file sa window, o Ctrl+V",
    help: "Wheel — laki ng frame · Ctrl+wheel — zoom · Shift+wheel, [ ] — iikot · \
        I-drag gamit ang right/middle click — igalaw · F — ipagkasya · arrow keys — ±1px",
    open_failed: |p, e| format!("Hindi mabuksan ang {p}: {e}"),
    too_small: |w, h| format!("Masyadong maliit ang larawang {w}×{h} para sa 9:14"),
    tab_closed: "Nakasara na ang tab",
    saved: |w, h, p| format!("Nai-save ang {w}×{h} → {p}"),
    save_failed: |e| format!("Hindi na-save: {e}"),
    open: "Buksan",
    paste: "I-paste",
    save: "I-save",
    save_as: "I-save bilang…",
    close_tab: "Isara (Ctrl+W)",
    drop_here: "I-drag ang larawan dito",
    language: "Wika",
    size: "Laki:",
    auto: "Awto",
    auto_hint: |max| format!("Kung paano na-crop, pero hindi lalampas sa {max}"),
    fixed: "Nakapirmi",
    fixed_hint: "Ang lapad ay multiple ng 9, ang taas ay multiple ng 14",
    angle: "Anggulo:",
    angle_hint: "Iikot ang larawan pakanan (clockwise)",
    ccw: "Pakaliwa (counterclockwise)",
    cw: "Pakanan (clockwise)",
    crop: |w, h, ow, oh| format!("Crop {w}×{h} → {ow}×{oh}"),
    gaps_transparent: "Lumalampas ang frame sa gilid ng inikot na larawan — magiging transparent ang mga sulok",
    gaps_black: "Lumalampas ang frame sa gilid ng inikot na larawan — magiging itim ang mga sulok",
    upscaled: "Mas maliit ang crop kaysa sa output size — palalakihin ito",
    below_min: |min| format!("Mas maliit sa {min}"),
    images: "Mga larawan",
    clipboard: "clipboard",
    clipboard_unavailable: |e| format!("Hindi magamit ang clipboard: {e}"),
    clipboard_bad_image: "Sirang larawan sa clipboard",
    clipboard_empty: "Walang larawan sa clipboard",
};

static MS: Strings = Strings {
    welcome: "Buka imej: Ctrl+O, seret fail ke tetingkap, atau Ctrl+V",
    help: "Roda — saiz bingkai · Ctrl+roda — zum · Shift+roda, [ ] — putar · \
        Seret klik kanan/tengah — anjak · F — muat · anak panah — ±1px",
    open_failed: |p, e| format!("Tidak dapat membuka {p}: {e}"),
    too_small: |w, h| format!("Imej {w}×{h} terlalu kecil untuk 9:14"),
    tab_closed: "Tab sudah ditutup",
    saved: |w, h, p| format!("Disimpan {w}×{h} → {p}"),
    save_failed: |e| format!("Ralat semasa menyimpan: {e}"),
    open: "Buka",
    paste: "Tampal",
    save: "Simpan",
    save_as: "Simpan sebagai…",
    close_tab: "Tutup (Ctrl+W)",
    drop_here: "Seret imej ke sini",
    language: "Bahasa",
    size: "Saiz:",
    auto: "Auto",
    auto_hint: |max| format!("Seperti dipangkas, tetapi tidak melebihi {max}"),
    fixed: "Tetap",
    fixed_hint: "Lebar gandaan 9, tinggi gandaan 14",
    angle: "Sudut:",
    angle_hint: "Putar imej mengikut arah jam",
    ccw: "Lawan arah jam",
    cw: "Ikut arah jam",
    crop: |w, h, ow, oh| format!("Pangkas {w}×{h} → {ow}×{oh}"),
    gaps_transparent: "Bingkai melepasi tepi imej yang diputar — sudutnya akan lut sinar",
    gaps_black: "Bingkai melepasi tepi imej yang diputar — sudutnya akan hitam",
    upscaled: "Pangkasan lebih kecil daripada saiz output — akan dibesarkan",
    below_min: |min| format!("Lebih kecil daripada {min}"),
    images: "Imej",
    clipboard: "papan keratan",
    clipboard_unavailable: |e| format!("Papan keratan tidak tersedia: {e}"),
    clipboard_bad_image: "Imej tidak sah dalam papan keratan",
    clipboard_empty: "Tiada imej dalam papan keratan",
};

static UK: Strings = Strings {
    welcome: "Відкрийте зображення: Ctrl+O, перетягніть файл у вікно або Ctrl+V",
    help: "Колесо — розмір рамки · Ctrl+колесо — зум · Shift+колесо, [ ] — поворот · \
        ПКМ/СКМ — зсув · F — вписати · стрілки — ±1px",
    open_failed: |p, e| format!("Не вдалося відкрити {p}: {e}"),
    too_small: |w, h| format!("Зображення {w}×{h} замале для 9:14"),
    tab_closed: "Вкладку вже закрито",
    saved: |w, h, p| format!("Збережено {w}×{h} → {p}"),
    save_failed: |e| format!("Помилка збереження: {e}"),
    open: "Відкрити",
    paste: "Вставити",
    save: "Зберегти",
    save_as: "Зберегти як…",
    close_tab: "Закрити (Ctrl+W)",
    drop_here: "Перетягніть зображення сюди",
    language: "Мова",
    size: "Розмір:",
    auto: "Авто",
    auto_hint: |max| format!("Як вирізано, але не більше {max}"),
    fixed: "Фіксований",
    fixed_hint: "Ширина кратна 9, висота кратна 14",
    angle: "Кут:",
    angle_hint: "Поворот зображення за годинниковою стрілкою",
    ccw: "Проти годинникової",
    cw: "За годинниковою",
    crop: |w, h, ow, oh| format!("Кроп {w}×{h} → {ow}×{oh}"),
    gaps_transparent: "Рамка заходить за край повернутого зображення — кути будуть прозорими",
    gaps_black: "Рамка заходить за край повернутого зображення — кути будуть чорними",
    upscaled: "Кроп менший за вихідний розмір — буде збільшено",
    below_min: |min| format!("Менше за {min}"),
    images: "Зображення",
    clipboard: "буфер обміну",
    clipboard_unavailable: |e| format!("Буфер обміну недоступний: {e}"),
    clipboard_bad_image: "Некоректне зображення в буфері обміну",
    clipboard_empty: "У буфері обміну немає зображення",
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_locales() {
        assert_eq!(Lang::from_locale("uk_UA.UTF-8"), Some(Lang::Uk));
        assert_eq!(Lang::from_locale("pt-BR"), Some(Lang::Pt));
        assert_eq!(Lang::from_locale("de_DE@euro"), Some(Lang::De));
        assert_eq!(Lang::from_locale("tl_PH"), Some(Lang::Fil));
        assert_eq!(Lang::from_locale("EN"), Some(Lang::En));
        assert_eq!(Lang::from_locale("C.UTF-8"), None);
        assert_eq!(Lang::from_locale("ja_JP"), None);
    }
}
