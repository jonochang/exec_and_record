# exec_and_record design

## Overview
`exec_and_record` shells out to established terminal capture and rendering tools to produce reliable artifacts quickly. It prioritizes correctness and portability over a custom capture stack.

## Pipeline
1. Resolve output directory and base name from `--output`, else `--out-dir`/`--name`, else defaults.
2. Ensure required tools exist based on selected output formats.
3. Run `asciinema rec` at fixed terminal size.
4. Optionally run:
   - `asciinema convert -f txt` for transcript
   - `agg` for GIF render
   - `ffmpeg` for MP4 conversion
   - `script` for a raw log when `raw` format is selected

## Output Resolution
Precedence order:
1. `--output` (derives directory + base name from the path)
2. `--out-dir` + `--name`
3. Defaults: `./recordings` + `session_YYYYMMDD_HHMMSS`

## Error Handling
- If a required tool is missing, the command fails early with a clear error.
- Each subprocess is executed with status checking and a labeled error on failure.

## File Layout
- `src/main.rs`: CLI parsing, output resolution, orchestration of external tools.
- `docs/specs/`: product brief and design notes.

## Future Enhancements
- `--output` to mean single file only with inferred format.
- Optional retention of intermediate GIF when producing MP4.
- Embed terminal theme or font configuration.
