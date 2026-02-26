# exec_and_record brief

## Goal
Provide a small CLI that records an interactive terminal command at a fixed size and emits video and log artifacts for LLM review.

## Success Criteria
- Runs any command via `-- <cmd> [args...]` and records at 120x60 by default.
- Produces an MP4 by default, with optional additional output formats.
- Simple overrides for output path, output directory, and base name.
- Clear failure messages when required external tools are missing.

## Non-Goals
- Pure-Rust terminal capture, rendering, or video encoding pipeline.
- Live streaming or remote upload of recordings.
- Post-processing features beyond basic format conversion.

## Constraints
- Uses external tools: `asciinema`, `agg`, `ffmpeg`, and optionally `script`.
- Output defaults under `./recordings`.

## CLI Summary
- `--format mp4,txt,gif,raw,cast` (default: `mp4`)
- `--output <path>` sets output directory and base name
- `--out-dir <path>` sets output directory
- `--name <base>` sets base name
- `--cols`, `--rows` for terminal size

## Output Templates
- Default base name: `session_YYYYMMDD_HHMMSS`
- Default directory: `./recordings`
- Output files: `<base>.mp4`, `<base>.gif`, `<base>.txt`, `<base>.raw.log`, `<base>.cast`
