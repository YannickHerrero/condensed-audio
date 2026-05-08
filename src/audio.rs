use anyhow::Result;

use crate::ffprobe::Stream;
use crate::picker::{self, PickOpts};

/// Returns the audio-sub-index (the N in ffmpeg's `0:a:N`).
/// If there is exactly one audio stream, returns Some(0) without prompting.
pub fn pick_audio_track(audio_streams: &[&Stream]) -> Result<Option<u32>> {
    match audio_streams.len() {
        0 => anyhow::bail!("video has no audio streams"),
        1 => Ok(Some(0)),
        _ => prompt_audio(audio_streams),
    }
}

fn prompt_audio(audio_streams: &[&Stream]) -> Result<Option<u32>> {
    let labels: Vec<String> = audio_streams
        .iter()
        .enumerate()
        .map(|(i, s)| format_audio_label(i, s))
        .collect();

    let picked = picker::pick(
        labels.iter().map(String::as_str),
        PickOpts { prompt: "audio track" },
    )?;
    let Some(picked) = picked else { return Ok(None) };

    let idx = labels.iter().position(|l| *l == picked).unwrap();
    Ok(Some(idx as u32))
}

fn format_audio_label(audio_idx: usize, s: &Stream) -> String {
    let codec = s.codec_name.as_deref().unwrap_or("?");
    let lang = s.language().unwrap_or("und");
    let ch = s.channels.map(|c| c.to_string()).unwrap_or_else(|| "?".into());
    let title = s.title().unwrap_or("");
    if title.is_empty() {
        format!(
            "#{audio_idx} (stream={})  lang={}  codec={}  ch={}",
            s.index, lang, codec, ch
        )
    } else {
        format!(
            "#{audio_idx} (stream={})  lang={}  codec={}  ch={}  title={}",
            s.index, lang, codec, ch, title
        )
    }
}
