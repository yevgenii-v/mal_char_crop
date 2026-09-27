//! Native file dialogs, run on a thread so the window keeps repainting.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, TryRecvError, channel};

use eframe::egui;

use crate::imaging::{OPEN_EXTENSIONS, SaveFormat};

#[derive(Clone, Copy)]
pub enum DialogKind {
    Open,
    /// Save the tab with this id.
    Save(u64),
}

pub struct PendingDialog {
    pub kind: DialogKind,
    rx: Receiver<Vec<PathBuf>>,
}

impl PendingDialog {
    pub fn open(ctx: &egui::Context, dir: Option<&Path>, filter_name: &str) -> Self {
        let dlg = with_dir(rfd::FileDialog::new(), dir).add_filter(filter_name, OPEN_EXTENSIONS);
        Self::spawn(ctx, DialogKind::Open, move || dlg.pick_files())
    }

    pub fn save(
        ctx: &egui::Context,
        tab_id: u64,
        dir: Option<&Path>,
        file_name: String,
        format: SaveFormat,
    ) -> Self {
        // The tab's own format is listed first so it is the default filter.
        let mut filters = [("PNG", &["png"][..]), ("JPEG", &["jpg", "jpeg"][..])];
        if format == SaveFormat::Jpeg {
            filters.reverse();
        }
        let dlg = filters
            .into_iter()
            .fold(with_dir(rfd::FileDialog::new(), dir), |d, (label, exts)| d.add_filter(label, exts))
            .set_file_name(file_name);
        Self::spawn(ctx, DialogKind::Save(tab_id), move || dlg.save_file().map(|p| vec![p]))
    }

    fn spawn(
        ctx: &egui::Context,
        kind: DialogKind,
        run: impl FnOnce() -> Option<Vec<PathBuf>> + Send + 'static,
    ) -> Self {
        let (tx, rx) = channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(run().unwrap_or_default());
            ctx.request_repaint();
        });
        Self { kind, rx }
    }

    /// The chosen paths once the dialog is closed (empty if cancelled).
    pub fn result(&self) -> Option<Vec<PathBuf>> {
        match self.rx.try_recv() {
            Ok(paths) => Some(paths),
            Err(TryRecvError::Empty) => None,
            // The dialog thread died; treat it as cancelled rather than blocking new dialogs.
            Err(TryRecvError::Disconnected) => Some(Vec::new()),
        }
    }
}

fn with_dir(dlg: rfd::FileDialog, dir: Option<&Path>) -> rfd::FileDialog {
    match dir {
        Some(dir) => dlg.set_directory(dir),
        None => dlg,
    }
}
