mod audio;
mod cli;
mod discovery;
mod extract;
mod ffprobe;
mod output;
mod picker;
mod segments;
mod srt;
mod subtitle;

use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result, bail};
use clap::Parser;

use crate::picker::{PickOpts, pick};
use crate::subtitle::Source;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err:#}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<()> {
    let args = cli::Args::parse();
    let cwd = env::current_dir().context("getting current directory")?;

    // 1. Pick a video.
    let video = match pick_video(&cwd)? {
        Some(v) => v,
        None => {
            println!("cancelled.");
            return Ok(());
        }
    };
    println!("video: {}", video.display());

    // 2. Probe streams.
    let probe = ffprobe::probe(&video).context("probing video")?;

    // 3. Choose subtitle source.
    let source = match subtitle::prompt_source()? {
        Some(s) => s,
        None => {
            println!("cancelled.");
            return Ok(());
        }
    };

    // 4. Resolve subtitle file path. _tmp keeps the embedded extraction alive.
    let (srt_path, _tmp): (PathBuf, Option<tempfile::NamedTempFile>) = match source {
        Source::Embedded => {
            let subs = probe.subtitle_streams();
            let stream_idx = match subtitle::pick_embedded_track(&subs)? {
                Some(i) => i,
                None => {
                    println!("cancelled.");
                    return Ok(());
                }
            };
            let tmp = subtitle::extract_embedded_to_srt(&video, stream_idx)?;
            (tmp.path().to_path_buf(), Some(tmp))
        }
        Source::External => {
            let p = match subtitle::pick_external_srt(&cwd)? {
                Some(p) => p,
                None => {
                    println!("cancelled.");
                    return Ok(());
                }
            };
            (p, None)
        }
    };
    println!("subtitles: {}", srt_path.display());

    // 5. Pick audio track.
    let audio_streams = probe.audio_streams();
    let audio_idx = match audio::pick_audio_track(&audio_streams)? {
        Some(i) => i,
        None => {
            println!("cancelled.");
            return Ok(());
        }
    };
    println!("audio track: 0:a:{audio_idx}");

    // 6. Parse SRT, filter noise, build segments.
    let cues = srt::parse_file(&srt_path)?;
    let cues = srt::filter_noise(cues);
    if cues.is_empty() {
        bail!("no spoken segments after filtering — every cue was empty/noise");
    }
    let segments = segments::build(&cues, args.pad_ms, args.gap_ms, probe.duration_ms);
    if segments.is_empty() {
        bail!("no segments to encode");
    }
    println!(
        "segments: {} (total ~{:.1}s)",
        segments.len(),
        segments::total_duration_ms(&segments) as f64 / 1000.0
    );

    // 7. Resolve output paths + confirm overwrite.
    let out = output::resolve(&cwd, &video);
    if !output::ensure_dir_and_confirm(&out)? {
        println!("aborted by user.");
        return Ok(());
    }

    // 8. Remap cues onto the condensed timeline (for the new SRT).
    let remapped = segments::remap_cues(&cues, &segments);

    // 9. Encode audio, then write the matching SRT.
    println!("encoding to {} …", out.mp3.display());
    extract::encode(&video, audio_idx, &segments, &out.mp3)?;
    srt::write(&out.srt, &remapped)?;

    let condensed_ms = segments::total_duration_ms(&segments);
    println!("done: {}", out.dir.display());
    match probe.duration_ms {
        Some(orig) if orig > 0 => println!(
            "original: {} → condensed: {} ({:.1}%)",
            fmt_duration(orig),
            fmt_duration(condensed_ms),
            100.0 * condensed_ms as f64 / orig as f64,
        ),
        _ => println!("condensed: {}", fmt_duration(condensed_ms)),
    }

    Ok(())
}

fn fmt_duration(ms: u64) -> String {
    let total_secs = ms / 1000;
    let h = total_secs / 3600;
    let m = (total_secs % 3600) / 60;
    let s = total_secs % 60;
    if h > 0 {
        format!("{h}h{m:02}m{s:02}s")
    } else {
        format!("{m}m{s:02}s")
    }
}

fn pick_video(cwd: &std::path::Path) -> Result<Option<PathBuf>> {
    let videos = discovery::find_videos(cwd)?;
    if videos.is_empty() {
        bail!("no video files found under {}", cwd.display());
    }
    let labels: Vec<String> = videos.iter().map(|p| p.display().to_string()).collect();
    let picked = pick(
        labels.iter().map(String::as_str),
        PickOpts { prompt: "video" },
    )?;
    Ok(picked.map(PathBuf::from))
}
