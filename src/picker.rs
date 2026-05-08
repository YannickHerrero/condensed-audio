use std::io::Write;
use std::process::{Command, Stdio};

use anyhow::{Context, Result, anyhow, bail};

pub struct PickOpts<'a> {
    pub prompt: &'a str,
}

/// Pipe `lines` to fzf and return the selected line.
/// Returns Ok(None) when the user cancels (Esc / Ctrl-C).
pub fn pick<'a, I, S>(lines: I, opts: PickOpts<'_>) -> Result<Option<String>>
where
    I: IntoIterator<Item = &'a S>,
    S: AsRef<str> + ?Sized + 'a,
{
    let mut child = Command::new("fzf")
        .arg(format!("--prompt={}> ", opts.prompt))
        .arg("--no-multi")
        .arg("--height=40%")
        .arg("--reverse")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .context("failed to spawn fzf — is it installed and on PATH?")?;

    {
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow!("failed to open fzf stdin"))?;
        for line in lines {
            stdin.write_all(line.as_ref().as_bytes())?;
            stdin.write_all(b"\n")?;
        }
    }

    let output = child.wait_with_output().context("waiting for fzf")?;
    match output.status.code() {
        Some(0) => {
            let s = String::from_utf8(output.stdout).context("fzf stdout not utf-8")?;
            Ok(Some(s.trim_end_matches('\n').to_string()))
        }
        Some(1) | Some(130) => Ok(None), // 1 = no match, 130 = ctrl-c / esc
        Some(code) => bail!("fzf exited with code {code}"),
        None => bail!("fzf terminated by signal"),
    }
}
