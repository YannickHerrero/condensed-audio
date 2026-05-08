mod cli;
mod discovery;
mod picker;

use std::env;

use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    let args = cli::Args::parse();
    let cwd = env::current_dir()?;
    let videos = discovery::find_videos(&cwd)?;
    println!(
        "pad_ms={} gap_ms={} videos_found={}",
        args.pad_ms,
        args.gap_ms,
        videos.len()
    );
    Ok(())
}
