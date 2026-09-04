//! Background directory scan for the file browser.
//! Files are reported as the walk finds them and analyzed on the rayon pool, so the list fills in early.

use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;

use walkdir::WalkDir;

use crate::ls::{LsFlags, LsRow, analyze_tile_row, is_tile_extension};

pub(crate) enum ScanEvent {
    /// A tile file was found.
    Found(PathBuf),
    /// Analysis of the file found at this position finished.
    Analyzed(usize, Box<LsRow>),
    /// The directory walk is complete (analyses may still be in flight).
    Done,
}

pub(crate) fn start_scan(dir: PathBuf, flags: LsFlags) -> mpsc::Receiver<ScanEvent> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let entries = WalkDir::new(&dir)
            .sort_by_file_name()
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_file() && is_tile_extension(e.path()));
        for (i, entry) in entries.enumerate() {
            let path = entry.into_path();
            if tx.send(ScanEvent::Found(path.clone())).is_err() {
                return;
            }
            let tx = tx.clone();
            let base = dir.clone();
            rayon::spawn(move || {
                let row = Box::new(analyze_tile_row(&path, &base, flags));
                let _ = tx.send(ScanEvent::Analyzed(i, row));
            });
        }
        let _ = tx.send(ScanEvent::Done);
    });
    rx
}
