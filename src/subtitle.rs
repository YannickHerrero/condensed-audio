use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use tempfile::NamedTempFile;

use crate::discovery;
use crate::ffprobe::Stream;
use crate::picker::{self, PickOpts};

const TEXT_SUB_CODECS: &[&str] = &["subrip", "ass", "ssa", "mov_text", "webvtt"];

#[derive(Debug, Clone, Copy)]
pub enum Source {
    Embedded,
    External,
}

pub fn prompt_source() -> Result<Option<Source>> {
    let lines = ["Embedded subtitle track", "External SRT file"];
    let picked = picker::pick(
        lines.iter().copied(),
        PickOpts {
            prompt: "subtitle source",
        },
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
    let labels: Vec<String> = srts.iter().map(|p| p.display().to_string()).collect();
    let picked = picker::pick(
        labels.iter().map(String::as_str),
        PickOpts {
            prompt: "external srt",
        },
    )?;
    Ok(picked.map(PathBuf::from))
}

pub fn pick_embedded_track(streams: &[&Stream]) -> Result<Option<u32>> {
    let candidates: Vec<&&Stream> = streams
        .iter()
        .filter(|s| {
            s.codec_name
                .as_deref()
                .map(|c| TEXT_SUB_CODECS.contains(&c))
                .unwrap_or(false)
        })
        .collect();

    if candidates.is_empty() {
        bail!("no text-based embedded subtitle tracks found (PGS/VobSub aren't supported)");
    }

    let labels: Vec<String> = candidates.iter().map(|s| format_stream_label(s)).collect();

    let picked = picker::pick(
        labels.iter().map(String::as_str),
        PickOpts {
            prompt: "subtitle track",
        },
    )?;
    let Some(picked) = picked else {
        return Ok(None);
    };

    let idx = labels.iter().position(|l| *l == picked).unwrap();
    Ok(Some(candidates[idx].index))
}

/// Extract an embedded subtitle stream to a temporary .srt file.
/// `stream_index` is the global ffprobe index.
pub fn extract_embedded_to_srt(video: &Path, stream_index: u32) -> Result<NamedTempFile> {
    let tmp = tempfile::Builder::new()
        .prefix("condensed-audio-")
        .suffix(".srt")
        .tempfile()
        .context("creating temp srt file")?;

    let status = Command::new("ffmpeg")
        .arg("-y")
        .arg("-v")
        .arg("error")
        .arg("-i")
        .arg(video)
        .arg("-map")
        .arg(format!("0:{stream_index}"))
        .arg("-c:s")
        .arg("srt")
        .arg(tmp.path())
        .status()
        .context("failed to spawn ffmpeg")?;

    if !status.success() {
        bail!("ffmpeg failed extracting subtitle stream {stream_index}");
    }

    Ok(tmp)
}

fn format_stream_label(s: &Stream) -> String {
    let codec = s.codec_name.as_deref().unwrap_or("?");
    let lang = s.language().unwrap_or("und");
    let title = s.title().unwrap_or("");
    if title.is_empty() {
        format!("#{}  lang={}  codec={}", s.index, lang, codec)
    } else {
        format!(
            "#{}  lang={}  codec={}  title={}",
            s.index, lang, codec, title
        )
    }
}
