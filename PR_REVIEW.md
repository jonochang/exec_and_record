# PR Review: exec_and_record

## Correctness

### C1. `Command` enum shadows `std::process::Command` (blocking)

`src/main.rs:3` imports `std::process::Command`, and `src/main.rs:18` defines `enum Command`. The local enum shadows the import. Every call to `Command::new(...)` (lines 112, 125, 137, 148) resolves to the enum, not `std::process::Command`. The enum has no `new` associated function, so **this code does not compile**.

Fix: rename the enum (e.g., `enum Cmd` or `enum SubCommand`) or fully qualify `std::process::Command::new(...)` at each call site.

### C2. `escape(OsStr::new(s))` type mismatch (blocking)

`src/main.rs:227` — `shell_escape::escape` expects `Cow<str>`, but `OsStr::new(s)` produces `&OsStr`, which does not implement `Into<Cow<str>>`. This is a compile-time type error.

Fix: since `s` is already `&String`, pass `Cow::Borrowed(s.as_str())` directly.

### C3. `agg` invoked without `--rows`

`src/main.rs:139-140` — `agg` receives `--cols` but not `--rows`. The recording is captured at a specific `cols x rows` geometry, but the GIF render only constrains columns. Depending on `agg` version, this may cause the rendered GIF to have a different row count than the recording, cutting off content or adding whitespace.

### C4. Intermediate `.cast` file not cleaned up

`src/main.rs:162-164` correctly removes the intermediate `.gif` when only mp4 was requested. However, the `.cast` file is always produced (it's asciinema's native format) and is never removed when the user didn't request `cast`. Users asking for only `mp4` will always find a stray `.cast` file alongside it.

### C5. Nested shell escaping is fragile

`src/main.rs:104-109` — When `raw` format is selected, the user command is shell-escaped once via `shell_join`, then embedded into a `script -q -f -c {cmd} {path}` string, which is then passed to `asciinema rec -c`. This is two levels of shell interpretation with only one level of escaping. Commands containing quotes, spaces, or metacharacters may break or be interpreted incorrectly.

---

## Simplicity

### S1. Redundant `Version` subcommand

`src/main.rs:24` defines a `Version` subcommand, but `#[command(version)]` on the `Cli` struct (line 11) already gives clap's built-in `--version` flag. Having both `exec_and_record version` and `exec_and_record --version` is confusing. Remove the subcommand and let clap handle it.

### S2. `normalize_formats` reimplements ordered dedup

`src/main.rs:191-199` is a manual ordered dedup loop. This could be a one-liner with itertools or, more practically, duplicate formats are harmless in the downstream `contains` checks since each conversion step is idempotent. Consider whether dedup is needed at all.

---

## Maintainability

### M1. Format-to-extension mapping is implicit and scattered

The mapping from `OutputFormat` to file extension is spread across lines 97-101 (`.cast`, `.txt`, `.raw.log`, `.gif`, `.mp4`). Adding a new format requires updating path construction, tool checking, conversion logic, cleanup logic, and summary output in five separate places. A method like `OutputFormat::extension(&self) -> &str` would centralize this.

### M2. `formats.contains(...)` repeated throughout

The same `contains` pattern appears in:
- Tool requirement checking (`require_tools_for_formats`, line 237)
- Recording command construction (line 104)
- Conversion dispatch (lines 124, 136, 147)
- Cleanup (line 162)
- Summary output (lines 168-179)

Each new format requires adding a branch in all of these. Consider iterating the format list once and dispatching per format, or giving each `OutputFormat` variant methods that describe its pipeline step.

### M3. All output paths eagerly computed

`src/main.rs:97-101` computes all five output paths regardless of which formats are requested. Not a bug, but it means adding formats requires adding path bindings even for unused ones. Computing paths on demand (or from a method on `OutputFormat`) would be cleaner.

---

## Redundancy

### R1. `--cols` is passed to both `asciinema rec` and `agg` separately

Lines 113-114 and 139-140 both manually thread `args.cols`. If `--rows` is added to the `agg` call (per C3), this duplication grows. A helper struct or function for terminal geometry args would reduce repetition.

### R2. Summary output repeats the same `if contains / eprintln` pattern

Lines 166-179 have five nearly identical blocks. A loop over the formats list printing each file that was produced would be shorter and wouldn't need updating when formats are added.

---

## Design

### D1. `cast` is conflated as both an intermediate artifact and a user-selectable format

The `.cast` file is always produced because asciinema requires it. Listing `cast` as a selectable `--format` option implies the user can opt in/out, but they can't opt out — the file is always created. Consider either: (a) always clean up `.cast` unless explicitly requested, or (b) document that `.cast` is always produced as a side effect.

### D2. No `--quiet` or `--verbose` flag

The "Done:" summary (lines 166-179) is always printed to stderr. The spec mentions CI use (scenarios.md, scenario 5). A `--quiet` flag would help CI pipelines that only care about exit codes.

### D3. Windows `is_executable` gives false cross-platform impression

`src/main.rs:289-291` provides a Windows implementation of `is_executable`, but the entire pipeline (`asciinema`, `agg`, `script`, `ffmpeg`) is Unix-only. Shipping a Windows code path for tool detection when the tools themselves don't work on Windows is misleading. Consider either removing the Windows path or gating the crate to Unix with `#[cfg(unix)]` at the crate level.

### D4. `check` subcommand doesn't verify tool versions

`check` (line 184) confirms tools exist in `PATH` but doesn't verify minimum versions. For instance, `asciinema convert` is only available in asciinema v3+. A stale install would pass `check` but fail during `record`.

---

## Summary

| Severity | Count | Items |
|----------|-------|-------|
| Blocking (won't compile) | 2 | C1, C2 |
| Bug / likely incorrect | 3 | C3, C4, C5 |
| Simplification | 2 | S1, S2 |
| Maintainability | 3 | M1, M2, M3 |
| Redundancy | 2 | R1, R2 |
| Design | 4 | D1, D2, D3, D4 |

**The two blocking issues (C1, C2) must be fixed before this code can compile.** After those, C3-C5 should be addressed for correctness. The remaining items are improvements that would make the codebase cleaner and more maintainable.
