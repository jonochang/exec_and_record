# Changelog

## 0.2.1

- Default to quiet output; `--verbose` restores tool output and summary.
- Add `--overwrite` to replace existing outputs.
- Echo command as the first line in the recording and keep an interactive shell open by default.
- Add `--exit-after` to end the session after the command finishes.

## 0.2.0

- Add Nix flake and package definitions for `nix profile add` installs.
- Introduce `record`/`check` subcommands and `--quiet` option.
- Add verbose dependency/version output for `check --verbose`.
- Improve output handling: optional `.cast`, unified paths, geometry args, and ffmpeg scaling fix.
- Improve error handling and dependency checks.

## 0.1.0

- Initial release.
