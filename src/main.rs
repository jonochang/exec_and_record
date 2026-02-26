use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

use anyhow::{bail, Context, Result};
use clap::{Args, Parser, Subcommand, ValueEnum};
use chrono::Local;
use shell_escape::escape;

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Record an interactive command
    Record(RecordArgs),
    /// Check dependencies for requested output formats
    Check(CheckArgs),
    /// Print version information
    Version,
}

#[derive(Args, Debug)]
struct RecordArgs {
    /// Output directory (default: ./recordings)
    ///
    /// Conflicts with `--output`.
    #[arg(long, conflicts_with = "output")]
    out_dir: Option<PathBuf>,

    /// Single output path. Sets directory and base name from the path.
    /// If an extension is present, it's ignored for naming other formats.
    ///
    /// Conflicts with `--out-dir` and `--name`.
    #[arg(long, conflicts_with_all = ["out_dir", "name"])]
    output: Option<PathBuf>,

    /// Base name for output files
    ///
    /// Conflicts with `--output`.
    #[arg(long, conflicts_with = "output")]
    name: Option<String>,

    /// Terminal columns
    #[arg(long, default_value_t = 120)]
    cols: u16,

    /// Terminal rows
    #[arg(long, default_value_t = 60)]
    rows: u16,

    /// Output formats (comma-separated). Default: mp4
    ///
    /// Supported values: cast, txt, raw, gif, mp4.
    #[arg(long, value_delimiter = ',', default_value = "mp4")]
    format: Vec<OutputFormat>,

    /// Command to exec and record. Use `--` before the command.
    #[arg(last = true, required = true)]
    cmd: Vec<String>,
}

#[derive(Args, Debug)]
struct CheckArgs {
    /// Output formats to validate (comma-separated). Default: mp4
    ///
    /// Supported values: cast, txt, raw, gif, mp4.
    #[arg(long, value_delimiter = ',', default_value = "mp4")]
    format: Vec<OutputFormat>,
}

#[derive(Clone, Debug, ValueEnum, PartialEq, Eq)]
enum OutputFormat {
    Cast,
    Txt,
    Raw,
    Gif,
    Mp4,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Record(args) => record(args),
        Command::Check(args) => check(args),
        Command::Version => {
            println!("{}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
    }
}

fn record(args: RecordArgs) -> Result<()> {
    let ts = Local::now().format("%Y%m%d_%H%M%S").to_string();
    let (out_dir, base_name) = resolve_output(&args, &ts);
    std::fs::create_dir_all(&out_dir)
        .with_context(|| format!("create out dir {}", out_dir.display()))?;

    let formats = normalize_formats(&args.format);
    require_tools_for_formats(&formats)?;

    let cast_file = out_dir.join(format!("{}.cast", base_name));
    let txt_file = out_dir.join(format!("{}.txt", base_name));
    let raw_file = out_dir.join(format!("{}.raw.log", base_name));
    let gif_file = out_dir.join(format!("{}.gif", base_name));
    let mp4_file = out_dir.join(format!("{}.mp4", base_name));

    let cmd_str = shell_join(&args.cmd);
    let rec_cmd = if formats.contains(&OutputFormat::Raw) {
        let raw_path = shell_escape_path(&raw_file);
        format!("script -q -f -c {} {}", cmd_str, raw_path)
    } else {
        cmd_str
    };

    run_status(
        Command::new("asciinema")
            .arg("rec")
            .arg("--cols")
            .arg(args.cols.to_string())
            .arg("--rows")
            .arg(args.rows.to_string())
            .arg("-c")
            .arg(rec_cmd)
            .arg(&cast_file),
        "asciinema rec",
    )?;

    if formats.contains(&OutputFormat::Txt) {
        run_status(
            Command::new("asciinema")
                .arg("convert")
                .arg("-f")
                .arg("txt")
                .arg(&cast_file)
                .arg(&txt_file),
            "asciinema convert txt",
        )?;
    }

    if formats.contains(&OutputFormat::Gif) || formats.contains(&OutputFormat::Mp4) {
        run_status(
            Command::new("agg")
                .arg("--cols")
                .arg(args.cols.to_string())
                .arg(&cast_file)
                .arg(&gif_file),
            "agg",
        )?;
    }

    if formats.contains(&OutputFormat::Mp4) {
        run_status(
            Command::new("ffmpeg")
                .arg("-y")
                .arg("-i")
                .arg(&gif_file)
                .arg("-movflags")
                .arg("faststart")
                .arg("-pix_fmt")
                .arg("yuv420p")
                .arg(&mp4_file),
            "ffmpeg",
        )?;
    }

    if !formats.contains(&OutputFormat::Gif) && formats.contains(&OutputFormat::Mp4) {
        let _ = std::fs::remove_file(&gif_file);
    }

    eprintln!("Done:");
    eprintln!("- asciinema: {}", cast_file.display());
    if formats.contains(&OutputFormat::Txt) {
        eprintln!("- text log:  {}", txt_file.display());
    }
    if formats.contains(&OutputFormat::Raw) {
        eprintln!("- raw log:   {}", raw_file.display());
    }
    if formats.contains(&OutputFormat::Gif) {
        eprintln!("- gif:       {}", gif_file.display());
    }
    if formats.contains(&OutputFormat::Mp4) {
        eprintln!("- mp4:       {}", mp4_file.display());
    }

    Ok(())
}

fn check(args: CheckArgs) -> Result<()> {
    let formats = normalize_formats(&args.format);
    require_tools_for_formats(&formats)?;
    println!("OK");
    Ok(())
}

fn normalize_formats(formats: &[OutputFormat]) -> Vec<OutputFormat> {
    let mut out = Vec::new();
    for f in formats {
        if !out.contains(f) {
            out.push(f.clone());
        }
    }
    out
}

fn resolve_output(cli: &RecordArgs, ts: &str) -> (PathBuf, String) {
    if let Some(output) = &cli.output {
        let dir = output.parent().filter(|p| !p.as_os_str().is_empty());
        let out_dir = dir.map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
        let base_name = output
            .file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("session_{}", ts));
        return (out_dir, base_name);
    }

    let out_dir = cli
        .out_dir
        .clone()
        .unwrap_or_else(|| PathBuf::from("recordings"));
    let base_name = cli
        .name
        .clone()
        .unwrap_or_else(|| format!("session_{}", ts));
    (out_dir, base_name)
}

fn shell_join(parts: &[String]) -> String {
    parts
        .iter()
        .map(|s| escape(OsStr::new(s)))
        .map(|s| s.to_string())
        .collect::<Vec<_>>()
        .join(" ")
}

fn shell_escape_path(path: &Path) -> String {
    escape(path.as_os_str()).to_string()
}

fn require_tools_for_formats(formats: &[OutputFormat]) -> Result<()> {
    require_tool("asciinema")?;
    if formats.contains(&OutputFormat::Raw) {
        require_tool("script")?;
    }
    if formats.contains(&OutputFormat::Gif) || formats.contains(&OutputFormat::Mp4) {
        require_tool("agg")?;
    }
    if formats.contains(&OutputFormat::Mp4) {
        require_tool("ffmpeg")?;
    }
    Ok(())
}

fn require_tool(name: &str) -> Result<()> {
    if find_in_path(name).is_none() {
        bail!("{name} not found in PATH");
    }
    Ok(())
}

fn find_in_path(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path).find_map(|dir| {
        let full = dir.join(name);
        if is_executable(&full) {
            Some(full)
        } else {
            None
        }
    })
}

fn run_status(cmd: &mut Command, label: &str) -> Result<()> {
    let status: ExitStatus = cmd
        .status()
        .with_context(|| format!("failed to run {label}"))?;
    if !status.success() {
        bail!("{label} failed with status {status}");
    }
    Ok(())
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path)
        .map(|m| m.is_file() && (m.permissions().mode() & 0o111 != 0))
        .unwrap_or(false)
}

#[cfg(windows)]
fn is_executable(path: &Path) -> bool {
    std::fs::metadata(path).map(|m| m.is_file()).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_formats_dedupes_preserves_order() {
        let formats = vec![
            OutputFormat::Mp4,
            OutputFormat::Txt,
            OutputFormat::Mp4,
            OutputFormat::Gif,
            OutputFormat::Txt,
        ];
        let out = normalize_formats(&formats);
        assert_eq!(
            out,
            vec![OutputFormat::Mp4, OutputFormat::Txt, OutputFormat::Gif]
        );
    }

    #[test]
    fn resolve_output_uses_output_path() {
        let args = RecordArgs {
            out_dir: Some(PathBuf::from("recordings")),
            output: Some(PathBuf::from("./out/demo.mp4")),
            name: Some("ignored".to_string()),
            cols: 120,
            rows: 60,
            format: vec![OutputFormat::Mp4],
            cmd: vec!["echo".to_string(), "hi".to_string()],
        };
        let (dir, base) = resolve_output(&args, "20250101_000000");
        assert_eq!(dir, PathBuf::from("./out"));
        assert_eq!(base, "demo");
    }

    #[test]
    fn resolve_output_uses_out_dir_and_name() {
        let args = RecordArgs {
            out_dir: Some(PathBuf::from("artifacts")),
            output: None,
            name: Some("run_42".to_string()),
            cols: 120,
            rows: 60,
            format: vec![OutputFormat::Mp4],
            cmd: vec!["echo".to_string(), "hi".to_string()],
        };
        let (dir, base) = resolve_output(&args, "20250101_000000");
        assert_eq!(dir, PathBuf::from("artifacts"));
        assert_eq!(base, "run_42");
    }

    #[test]
    fn resolve_output_defaults() {
        let args = RecordArgs {
            out_dir: None,
            output: None,
            name: None,
            cols: 120,
            rows: 60,
            format: vec![OutputFormat::Mp4],
            cmd: vec!["echo".to_string(), "hi".to_string()],
        };
        let (dir, base) = resolve_output(&args, "20250101_000000");
        assert_eq!(dir, PathBuf::from("recordings"));
        assert_eq!(base, "session_20250101_000000");
    }
}
