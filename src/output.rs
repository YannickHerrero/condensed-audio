use std::fs;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub struct OutputPaths {
    pub dir: PathBuf,
    pub mp3: PathBuf,
    pub srt: PathBuf,
}

/// Build `<cwd>/<stem>.condensed/` with `<stem>.condensed.{mp3,srt}` inside.
pub fn resolve(cwd: &Path, video: &Path) -> OutputPaths {
    let stem = video
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "output".into());
    let dir = cwd.join(format!("{stem}.condensed"));
    let mp3 = dir.join(format!("{stem}.condensed.mp3"));
    let srt = dir.join(format!("{stem}.condensed.srt"));
    OutputPaths { dir, mp3, srt }
}

/// Create the output dir if needed; if either file we own already exists,
/// prompt the user. Other files inside the directory are left alone.
/// Returns true if the caller may proceed.
pub fn ensure_dir_and_confirm(paths: &OutputPaths) -> Result<bool> {
    if !paths.dir.exists() {
        fs::create_dir_all(&paths.dir)
            .with_context(|| format!("creating {}", paths.dir.display()))?;
        return Ok(true);
    }

    let mp3_exists = paths.mp3.exists();
    let srt_exists = paths.srt.exists();
    if !mp3_exists && !srt_exists {
        return Ok(true);
    }

    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();
    let which = match (mp3_exists, srt_exists) {
        (true, true) => "audio + srt",
        (true, false) => "audio",
        (false, true) => "srt",
        _ => unreachable!(),
    };
    write!(
        stdout,
        "{}/ already contains a previous {} file. Overwrite? [y/N] ",
        paths.dir.display(),
        which
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

    #[test]
    fn resolves_folder_and_filenames() {
        let cwd = PathBuf::from("/tmp");
        let video = PathBuf::from("/some/where/Show.S01E02.mkv");
        let out = resolve(&cwd, &video);
        assert_eq!(out.dir, PathBuf::from("/tmp/Show.S01E02.condensed"));
        assert_eq!(
            out.mp3,
            PathBuf::from("/tmp/Show.S01E02.condensed/Show.S01E02.condensed.mp3")
        );
        assert_eq!(
            out.srt,
            PathBuf::from("/tmp/Show.S01E02.condensed/Show.S01E02.condensed.srt")
        );
    }

    #[test]
    fn handles_no_extension() {
        let cwd = PathBuf::from("/tmp");
        let video = PathBuf::from("/some/where/raw");
        let out = resolve(&cwd, &video);
        assert_eq!(out.dir, PathBuf::from("/tmp/raw.condensed"));
        assert_eq!(
            out.mp3,
            PathBuf::from("/tmp/raw.condensed/raw.condensed.mp3")
        );
    }
}
