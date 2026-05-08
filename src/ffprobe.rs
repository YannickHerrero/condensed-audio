use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Stream {
    pub index: u32,
    pub codec_type: String,
    #[serde(default)]
    pub codec_name: Option<String>,
    #[serde(default)]
    pub channels: Option<u32>,
    #[serde(default)]
    pub tags: HashMap<String, String>,
}

impl Stream {
    pub fn language(&self) -> Option<&str> {
        self.tags.get("language").map(|s| s.as_str())
    }

    pub fn title(&self) -> Option<&str> {
        self.tags.get("title").map(|s| s.as_str())
    }
}

#[derive(Debug, Deserialize)]
pub struct Format {
    #[serde(default)]
    pub duration: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ProbeOutput {
    streams: Vec<Stream>,
    format: Format,
}

#[derive(Debug)]
pub struct ProbeResult {
    pub streams: Vec<Stream>,
    pub duration_ms: Option<u64>,
}

impl ProbeResult {
    pub fn audio_streams(&self) -> Vec<&Stream> {
        self.streams
            .iter()
            .filter(|s| s.codec_type == "audio")
            .collect()
    }

    pub fn subtitle_streams(&self) -> Vec<&Stream> {
        self.streams
            .iter()
            .filter(|s| s.codec_type == "subtitle")
            .collect()
    }
}

pub fn probe(video: &Path) -> Result<ProbeResult> {
    let output = Command::new("ffprobe")
        .args([
            "-v", "error",
            "-print_format", "json",
            "-show_streams",
            "-show_format",
        ])
        .arg(video)
        .output()
        .context("failed to spawn ffprobe — is it installed and on PATH?")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("ffprobe failed: {stderr}");
    }

    let parsed: ProbeOutput =
        serde_json::from_slice(&output.stdout).context("parsing ffprobe json")?;

    let duration_ms = parsed.format.duration.as_deref().and_then(|s| {
        s.parse::<f64>().ok().map(|d| (d * 1000.0).round() as u64)
    });

    Ok(ProbeResult {
        streams: parsed.streams,
        duration_ms,
    })
}
