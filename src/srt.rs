use std::fs;
use std::path::Path;
use std::sync::OnceLock;

use anyhow::{Context, Result, bail};
use regex::Regex;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cue {
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: String,
}

pub fn parse_file(path: &Path) -> Result<Vec<Cue>> {
    let raw = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    // SRT files are commonly UTF-8 (often with BOM); fall back to lossy if not.
    let text = match std::str::from_utf8(&raw) {
        Ok(s) => s.to_string(),
        Err(_) => String::from_utf8_lossy(&raw).into_owned(),
    };
    parse_str(text.trim_start_matches('\u{feff}'))
}

pub fn parse_str(input: &str) -> Result<Vec<Cue>> {
    let mut cues = Vec::new();
    let normalized = input.replace("\r\n", "\n").replace('\r', "\n");

    for block in normalized.split("\n\n") {
        let block = block.trim_matches('\n');
        if block.is_empty() {
            continue;
        }
        let mut lines = block.lines();
        let first = lines.next().unwrap_or("");
        // The first line is usually a numeric index — but some SRTs omit it.
        // If it doesn't contain "-->", treat it as the index and read the next line.
        let timing_line = if first.contains("-->") {
            first.to_string()
        } else {
            match lines.next() {
                Some(l) => l.to_string(),
                None => continue,
            }
        };

        let (start, end) = parse_timing(&timing_line)
            .with_context(|| format!("invalid timing line: {timing_line:?}"))?;

        let text = lines.collect::<Vec<_>>().join("\n");
        cues.push(Cue {
            start_ms: start,
            end_ms: end,
            text,
        });
    }

    Ok(cues)
}

fn parse_timing(line: &str) -> Result<(u64, u64)> {
    let mut parts = line.split("-->");
    let left = parts.next().unwrap_or("").trim();
    let right = parts.next().unwrap_or("").trim();
    if left.is_empty() || right.is_empty() {
        bail!("missing --> separator");
    }
    // strip any trailing styling info (X1:... etc.)
    let right = right.split_whitespace().next().unwrap_or("");
    Ok((parse_timestamp(left)?, parse_timestamp(right)?))
}

fn parse_timestamp(s: &str) -> Result<u64> {
    // HH:MM:SS,mmm  or  HH:MM:SS.mmm
    let s = s.trim();
    let (hms, ms) = match s.rsplit_once([',', '.']) {
        Some((a, b)) => (a, b),
        None => bail!("missing fractional separator in {s:?}"),
    };
    let mut hms_parts = hms.split(':');
    let h: u64 = hms_parts.next().unwrap_or("0").parse()?;
    let m: u64 = hms_parts.next().unwrap_or("0").parse()?;
    let sec: u64 = hms_parts.next().unwrap_or("0").parse()?;
    let ms: u64 = ms.parse()?;
    Ok(((h * 3600 + m * 60 + sec) * 1000) + ms)
}

/// Strip styling/SDH noise from a cue. Returns the cleaned text, possibly empty
/// if every line was just noise.
pub fn clean_text(input: &str) -> String {
    let html = html_re().replace_all(input, "");
    let ass = ass_re().replace_all(&html, "");
    let no_music = music_re().replace_all(&ass, "");

    let mut out_lines: Vec<String> = Vec::new();
    for line in no_music.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if is_only_bracketed(trimmed) {
            continue;
        }
        out_lines.push(trimmed.to_string());
    }
    out_lines.join("\n")
}

/// Drop cues whose cleaned text is empty (pure SDH/music/etc).
/// Surviving cues are returned with their **original** text untouched, so
/// the new SRT preserves italics and original spacing.
pub fn filter_noise(cues: Vec<Cue>) -> Vec<Cue> {
    cues.into_iter()
        .filter(|c| !clean_text(&c.text).is_empty())
        .collect()
}

fn html_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"<[^>]+>").unwrap())
}

fn ass_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"\{[^}]*\}").unwrap())
}

fn music_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"[\u{266A}\u{266B}\u{266C}\u{266D}\u{266E}\u{266F}]").unwrap())
}

/// True when the line is entirely a bracketed SDH descriptor like
/// "[door slams]" or "(in Spanish)".
fn is_only_bracketed(line: &str) -> bool {
    let s = line.trim();
    (s.starts_with('[') && s.ends_with(']')) || (s.starts_with('(') && s.ends_with(')'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_srt() {
        let input = "1\n00:00:01,000 --> 00:00:02,500\nHello\n\n2\n00:00:03,000 --> 00:00:04,000\nWorld\nLine two";
        let cues = parse_str(input).unwrap();
        assert_eq!(cues.len(), 2);
        assert_eq!(cues[0].start_ms, 1000);
        assert_eq!(cues[0].end_ms, 2500);
        assert_eq!(cues[0].text, "Hello");
        assert_eq!(cues[1].text, "World\nLine two");
    }

    #[test]
    fn parses_dot_separator() {
        let input = "00:00:01.250 --> 00:00:02.750\nText";
        let cues = parse_str(input).unwrap();
        assert_eq!(cues[0].start_ms, 1250);
        assert_eq!(cues[0].end_ms, 2750);
    }

    #[test]
    fn handles_crlf() {
        let input = "1\r\n00:00:01,000 --> 00:00:02,000\r\nHi\r\n";
        let cues = parse_str(input).unwrap();
        assert_eq!(cues.len(), 1);
        assert_eq!(cues[0].text, "Hi");
    }

    #[test]
    fn cleans_html_and_ass_tags() {
        assert_eq!(clean_text("<i>hello</i> {\\an8}world"), "hello world");
    }

    #[test]
    fn drops_pure_sdh_lines() {
        assert_eq!(clean_text("[door slams]"), "");
        assert_eq!(clean_text("(in Spanish)"), "");
        assert_eq!(clean_text("Hello\n[noise]\nworld"), "Hello\nworld");
    }

    #[test]
    fn strips_music_notes() {
        assert_eq!(clean_text("\u{266A} la la la \u{266A}"), "la la la");
    }

    #[test]
    fn filter_noise_drops_empty_cues() {
        let cues = vec![
            Cue {
                start_ms: 0,
                end_ms: 1000,
                text: "[music]".into(),
            },
            Cue {
                start_ms: 1000,
                end_ms: 2000,
                text: "Real line".into(),
            },
            Cue {
                start_ms: 2000,
                end_ms: 3000,
                text: "<i>♪</i>".into(),
            },
        ];
        let kept = filter_noise(cues);
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].text, "Real line");
    }

    #[test]
    fn filter_noise_preserves_original_formatting() {
        let cues = vec![Cue {
            start_ms: 0,
            end_ms: 1000,
            text: "<i>Bonjour</i>".into(),
        }];
        let kept = filter_noise(cues);
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].text, "<i>Bonjour</i>");
    }
}
