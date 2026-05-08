use std::path::{Path, PathBuf};

use anyhow::Result;
use walkdir::WalkDir;

const VIDEO_EXTS: &[&str] = &[
    "mkv", "mp4", "avi", "webm", "mov", "ts", "m4v", "wmv", "flv",
];

const SUBTITLE_EXTS: &[&str] = &["srt"];

pub fn find_videos(root: &Path) -> Result<Vec<PathBuf>> {
    find_with_exts(root, VIDEO_EXTS)
}

pub fn find_srts(root: &Path) -> Result<Vec<PathBuf>> {
    find_with_exts(root, SUBTITLE_EXTS)
}

fn find_with_exts(root: &Path, exts: &[&str]) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    for entry in WalkDir::new(root).follow_links(false) {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        if !entry.file_type().is_file() {
            continue;
        }
        let Some(ext) = entry.path().extension().and_then(|e| e.to_str()) else {
            continue;
        };
        let ext_lc = ext.to_ascii_lowercase();
        if exts.iter().any(|e| *e == ext_lc) {
            out.push(entry.path().to_path_buf());
        }
    }
    out.sort();
    Ok(out)
}
