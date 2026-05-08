use std::path::{Path, PathBuf};

use anyhow::{Result, bail};

use crate::discovery;
use crate::picker::{self, PickOpts};

#[derive(Debug, Clone, Copy)]
pub enum Source {
    Embedded,
    External,
}

pub fn prompt_source() -> Result<Option<Source>> {
    let lines = [
        "Embedded subtitle track",
        "External SRT file",
    ];
    let picked = picker::pick(
        lines.iter().copied(),
        PickOpts { prompt: "subtitle source" },
    )?;
    Ok(picked.map(|s| match s.as_str() {
        "Embedded subtitle track" => Source::Embedded,
        _ => Source::External,
    }))
}

pub fn pick_external_srt(root: &Path) -> Result<Option<PathBuf>> {
    let srts = discovery::find_srts(root)?;
    if srts.is_empty() {
        bail!("no .srt files found under {}", root.display());
    }
    let labels: Vec<String> = srts
        .iter()
        .map(|p| p.display().to_string())
        .collect();
    let picked = picker::pick(
        labels.iter().map(String::as_str),
        PickOpts { prompt: "external srt" },
    )?;
    Ok(picked.map(PathBuf::from))
}
