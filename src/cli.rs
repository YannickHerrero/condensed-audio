use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about = "Generate condensed audio from a video + subtitles")]
pub struct Args {
    #[arg(long, default_value_t = 500)]
    pub pad_ms: u32,

    #[arg(long, default_value_t = 200)]
    pub gap_ms: u32,
}
