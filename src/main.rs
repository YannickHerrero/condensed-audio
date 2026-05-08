mod cli;

use clap::Parser;

fn main() {
    let args = cli::Args::parse();
    println!("pad_ms={} gap_ms={}", args.pad_ms, args.gap_ms);
}
