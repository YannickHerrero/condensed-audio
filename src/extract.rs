use std::io::Write;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};

use crate::segments::Segment;

/// Build the audio filter graph that selects only the given segments and
/// resets timestamps so the result is contiguous.
fn build_filter(audio_idx: u32, segments: &[Segment]) -> String {
    let between_clauses: Vec<String> = segments
        .iter()
        .map(|s| {
            format!(
                "between(t,{:.3},{:.3})",
                s.start_ms as f64 / 1000.0,
                s.end_ms as f64 / 1000.0
            )
        })
        .collect();

    format!(
        "[0:a:{audio_idx}]aselect='{expr}',asetpts=N/SR/TB[out]",
        expr = between_clauses.join("+")
    )
}

pub fn encode(video: &Path, audio_idx: u32, segments: &[Segment], output: &Path) -> Result<()> {
    if segments.is_empty() {
        bail!("no segments to encode — nothing to do");
    }

    let filter = build_filter(audio_idx, segments);
    let mut filter_file = tempfile::Builder::new()
        .prefix("condensed-audio-filter-")
        .suffix(".txt")
        .tempfile()
        .context("creating filter tempfile")?;
    filter_file
        .write_all(filter.as_bytes())
        .context("writing filter tempfile")?;

    let status = Command::new("ffmpeg")
        .arg("-y")
        .arg("-v")
        .arg("error")
        .arg("-stats")
        .arg("-i")
        .arg(video)
        .arg("-filter_complex_script")
        .arg(filter_file.path())
        .arg("-map")
        .arg("[out]")
        .arg("-c:a")
        .arg("libmp3lame")
        .arg("-b:a")
        .arg("128k")
        .arg(output)
        .status()
        .context("failed to spawn ffmpeg")?;

    if !status.success() {
        bail!("ffmpeg failed encoding output");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_includes_each_segment() {
        let segs = vec![
            Segment {
                start_ms: 1000,
                end_ms: 2500,
            },
            Segment {
                start_ms: 4000,
                end_ms: 5500,
            },
        ];
        let filter = build_filter(2, &segs);
        assert!(filter.starts_with("[0:a:2]aselect='"));
        assert!(filter.contains("between(t,1.000,2.500)"));
        assert!(filter.contains("between(t,4.000,5.500)"));
        assert!(filter.contains("+"));
        assert!(filter.ends_with(",asetpts=N/SR/TB[out]"));
    }
}
