use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// Build `<cwd>/<video_stem>.condensed.mp3`.
pub fn resolve_path(cwd: &Path, video: &Path) -> PathBuf {
    let stem = video
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "output".into());
    cwd.join(format!("{stem}.condensed.mp3"))
}

/// If the path exists, prompt the user on stdin to confirm overwrite.
/// Returns true if the caller may proceed.
pub fn confirm_overwrite(path: &Path) -> Result<bool> {
    if !path.exists() {
        return Ok(true);
    }
    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();
    write!(
        stdout,
        "{} already exists. Overwrite? [y/N] ",
        path.display()
    )?;
    stdout.flush()?;

    let stdin = std::io::stdin();
    let mut line = String::new();
    stdin
        .lock()
        .read_line(&mut line)
        .context("reading confirmation")?;
    let answer = line.trim().to_ascii_lowercase();
    Ok(answer == "y" || answer == "yes")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn resolves_alongside_cwd() {
        let cwd = PathBuf::from("/tmp");
        let video = PathBuf::from("/some/where/Show.S01E02.mkv");
        assert_eq!(
            resolve_path(&cwd, &video),
            PathBuf::from("/tmp/Show.S01E02.condensed.mp3")
        );
    }

    #[test]
    fn handles_no_extension() {
        let cwd = PathBuf::from("/tmp");
        let video = PathBuf::from("/some/where/raw");
        assert_eq!(
            resolve_path(&cwd, &video),
            PathBuf::from("/tmp/raw.condensed.mp3")
        );
    }
}
