# exec_and_record

Record an interactive terminal command at a fixed size, then export to mp4/gif/txt/cast/raw.

## Dependencies

This tool shells out to:
- `asciinema`
- `agg` (for gif)
- `ffmpeg` (for mp4)
- `script` (for raw logs)

## Usage

```bash
exec_and_record record -- codex
exec_and_record record --output ./recordings/session_20260101_120000.mp4 --format mp4,txt -- claude
exec_and_record record --cols 120 --rows 60 --format mp4 -- gemini
exec_and_record check --format mp4,txt
exec_and_record check --format mp4,txt --verbose
exec_and_record --version
```

Outputs go to `./recordings/session_YYYYMMDD_HHMMSS.{mp4|cast|...}` by default, or you can
use `--output` to set a single output path (directory + base name derived from the path).
`--output` conflicts with `--out-dir` and `--name`.

Note: `.cast` is always produced by asciinema but is removed unless `--format cast` is requested.

## Commands

- `record`: run and record a command (default workflow)
- `check`: verify tool availability for requested formats
- `--version`: print version (built-in clap flag)

## Options (record)

- `--output <path>`: single output path; directory + base name derived from path
- `--out-dir <path>`: output directory (default `./recordings`)
- `--name <base>`: base filename (default `session_YYYYMMDD_HHMMSS`)
- `--format <list>`: comma-separated list (default `mp4`)
- `--cols <n>`: terminal columns (default `120`)
- `--rows <n>`: terminal rows (default `60`)
- `--verbose`: show tool output and summary for debugging (default is quiet)
- `-- <cmd> [args...]`: command to exec and record

## Options (check)

- `--format <list>`: comma-separated list (default `mp4`)
- `--verbose`: print dependency versions and default settings

## Platform

Unix-only. The pipeline depends on `asciinema`, `agg`, `ffmpeg`, and optionally `script`.

## Output Formats

- `mp4` (default)
- `gif`
- `txt` (plain text transcript)
- `cast` (asciinema recording)
- `raw` (raw terminal log via `script`)

## Build

```bash
cargo build --release
```

## Nix

```bash
nix develop
cargo generate-lockfile
cargo build --release
```
