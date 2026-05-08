# condensed-audio

A small Rust CLI that turns a video into a "condensed audio" file — only the
parts of the video where a subtitle line is present. Useful for language
learning (Refold / Anki immersion).

## Requirements

- `fzf`, `ffmpeg`, `ffprobe` on `PATH`.

## Usage

Run inside a directory that contains your video files:

```
condensed-audio
```

You'll be prompted (via `fzf`) to:

1. Pick a video file.
2. Choose between an embedded subtitle track or an external `.srt` file.
3. Pick the subtitle track / SRT.
4. Pick an audio track (skipped when only one).

The result is written to the current directory as
`<video_stem>.condensed.mp3`.

### Flags

- `--pad-ms <ms>` — padding around each subtitle line (default `500`).
- `--gap-ms <ms>` — merge two segments if separated by less than this many ms
  (default `200`).
