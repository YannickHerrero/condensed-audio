use std::io::Write;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};

use crate::segments::Segment;

/// Build a filter graph that:
///   1. splits the chosen audio stream into N taps (one per segment),
///   2. trims each tap to its segment and resets PTS to 0,
///   3. concatenates the trimmed clips into a single output.
///
/// Why this shape: the obvious approach (one big `aselect='between+between+...'`)
/// hits ffmpeg's expression-parser recursion limit around ~100 terms and fails
/// with "Cannot allocate memory". The atrim+concat graph stays flat regardless
/// of segment count.
fn build_filter(audio_idx: u32, segments: &[Segment]) -> String {
    let n = segments.len();
    let mut graph = String::new();

    // Tap labels: [a0], [a1], ...
    let split_outputs: String = (0..n).map(|i| format!("[a{i}]")).collect();
    graph.push_str(&format!("[0:a:{audio_idx}]asplit={n}{split_outputs};"));

    for (i, seg) in segments.iter().enumerate() {
        graph.push_str(&format!(
            "[a{i}]atrim=start={start:.3}:end={end:.3},asetpts=PTS-STARTPTS[s{i}];",
            start = seg.start_ms as f64 / 1000.0,
            end = seg.end_ms as f64 / 1000.0,
        ));
    }

    let concat_inputs: String = (0..n).map(|i| format!("[s{i}]")).collect();
    graph.push_str(&format!("{concat_inputs}concat=n={n}:v=0:a=1[out]"));

    graph
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
    fn filter_uses_atrim_concat_graph() {
        let segs = vec![
            Segment { start_ms: 1000, end_ms: 2500 },
            Segment { start_ms: 4000, end_ms: 5500 },
        ];
        let filter = build_filter(2, &segs);
        assert!(filter.contains("[0:a:2]asplit=2[a0][a1];"));
        assert!(filter.contains("[a0]atrim=start=1.000:end=2.500,asetpts=PTS-STARTPTS[s0];"));
        assert!(filter.contains("[a1]atrim=start=4.000:end=5.500,asetpts=PTS-STARTPTS[s1];"));
        assert!(filter.ends_with("[s0][s1]concat=n=2:v=0:a=1[out]"));
    }
}
