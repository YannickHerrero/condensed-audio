use anyhow::Result;

use crate::picker::{self, PickOpts};

#[derive(Debug, Clone, Copy)]
pub enum Source {
    Embedded,
    External,
}

pub fn prompt_source() -> Result<Option<Source>> {
    let lines = [
        "Embedded subtitle track",
        "External SRT file",
    ];
    let picked = picker::pick(
        lines.iter().copied(),
        PickOpts { prompt: "subtitle source" },
    )?;
    Ok(picked.map(|s| match s.as_str() {
        "Embedded subtitle track" => Source::Embedded,
        _ => Source::External,
    }))
}
