use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

use anyhow::{bail, Context, Result};
use clap::{Args, Parser, Subcommand, ValueEnum};
use chrono::Local;

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Cli {
    #[command(subcommand)]
    command: CliCommand,
}

#[derive(Subcommand, Debug)]
enum CliCommand {
    /// Record an interactive command
    Record(RecordArgs),
    /// Check dependencies for requested output formats
    Check(CheckArgs),
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
    #[arg(long, default_value_t = DEFAULT_COLS)]
    cols: u16,

    /// Terminal rows
    #[arg(long, default_value_t = DEFAULT_ROWS)]
    rows: u16,

    /// Output formats (comma-separated). Default: mp4
    ///
    /// Supported values: cast, txt, raw, gif, mp4.
    #[arg(long, value_delimiter = ',', default_value = DEFAULT_FORMATS)]
    format: Vec<OutputFormat>,

    /// Suppress summary output
    #[arg(long)]
    quiet: bool,

    /// Command to exec and record. Use `--` before the command.
    #[arg(last = true, required = true)]
    cmd: Vec<String>,
}

#[derive(Args, Debug)]
struct CheckArgs {
    /// Output formats to validate (comma-separated). Default: mp4
    ///
    /// Supported values: cast, txt, raw, gif, mp4.
    #[arg(long, value_delimiter = ',', default_value = DEFAULT_FORMATS)]
    format: Vec<OutputFormat>,

    /// Print dependency versions and default settings
    #[arg(long)]
    verbose: bool,
}

#[derive(Copy, Clone, Debug, ValueEnum, PartialEq, Eq, Hash)]
enum OutputFormat {
    Cast,
    Txt,
    Raw,
    Gif,
    Mp4,
}

const DEFAULT_COLS: u16 = 120;
const DEFAULT_ROWS: u16 = 60;
const DEFAULT_FORMATS: &str = "mp4";
const DEFAULT_OUT_DIR: &str = "recordings";

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        CliCommand::Record(args) => record(args),
        CliCommand::Check(args) => check(args),
    }
}

fn record(args: RecordArgs) -> Result<()> {
    let ts = Local::now().format("%Y%m%d_%H%M%S").to_string();
    let (out_dir, base_name) = resolve_output(&args, &ts);
    std::fs::create_dir_all(&out_dir)
        .with_context(|| format!("create out dir {}", out_dir.display()))?;

    let formats = args.format.clone();
    require_tools_for_formats(&formats)?;

    let cast_file = out_dir.join(format!("{}.{}", base_name, OutputFormat::Cast.extension()));
    let outputs = OutputPaths::new(&out_dir, &base_name);

    let cmd_str = shell_join(&args.cmd);
    let rec_cmd = if formats.contains(&OutputFormat::Raw) {
        let raw_path = shell_escape_path(outputs.path(OutputFormat::Raw).as_path());
        let cmd_arg = shell_escape_str(&cmd_str);
        format!("script -q -f -c {} {}", cmd_arg, raw_path)
    } else {
        cmd_str
    };

    run_status(
        Command::new("asciinema")
            .arg("rec")
            .args(geometry_args(args.cols, args.rows))
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
                .arg(outputs.path(OutputFormat::Txt)),
            "asciinema convert txt",
        )?;
    }

    if formats.contains(&OutputFormat::Gif) || formats.contains(&OutputFormat::Mp4) {
        run_status(
            Command::new("agg")
                .args(geometry_args(args.cols, args.rows))
                .arg(&cast_file)
                .arg(outputs.path(OutputFormat::Gif)),
            "agg",
        )?;
    }

    if formats.contains(&OutputFormat::Mp4) {
        run_status(
            Command::new("ffmpeg")
                .arg("-y")
                .arg("-i")
                .arg(outputs.path(OutputFormat::Gif))
                .arg("-vf")
                .arg("scale=trunc(iw/2)*2:trunc(ih/2)*2")
                .arg("-movflags")
                .arg("faststart")
                .arg("-pix_fmt")
                .arg("yuv420p")
                .arg(outputs.path(OutputFormat::Mp4)),
            "ffmpeg",
        )?;
    }

    if !formats.contains(&OutputFormat::Gif) && formats.contains(&OutputFormat::Mp4) {
        let _ = std::fs::remove_file(outputs.path(OutputFormat::Gif));
    }
    if !formats.contains(&OutputFormat::Cast) {
        let _ = std::fs::remove_file(&cast_file);
    }

    if !args.quiet {
        eprintln!("Done:");
        let mut printed = std::collections::HashSet::new();
        for format in &formats {
            if !printed.insert(*format) {
                continue;
            }
            if *format == OutputFormat::Cast {
                eprintln!("- cast:      {}", cast_file.display());
                continue;
            }
            eprintln!(
                "- {}: {}",
                format.label(),
                outputs.path(*format).display()
            );
        }
    }

    Ok(())
}

fn check(args: CheckArgs) -> Result<()> {
    let formats = args.format.clone();
    require_tools_for_formats(&formats)?;
    if args.verbose {
        print_versions(&formats)?;
        print_defaults();
    }
    println!("OK");
    Ok(())
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
        .unwrap_or_else(|| PathBuf::from(DEFAULT_OUT_DIR));
    let base_name = cli
        .name
        .clone()
        .unwrap_or_else(|| format!("session_{}", ts));
    (out_dir, base_name)
}

fn shell_join(parts: &[String]) -> String {
    parts
        .iter()
        .map(|s| shell_escape_str(s.as_str()))
        .collect::<Vec<_>>()
        .join(" ")
}

fn shell_escape_str(s: &str) -> String {
    shell_escape::escape(Cow::Borrowed(s)).to_string()
}

fn shell_escape_path(path: &Path) -> String {
    shell_escape::escape(path.to_string_lossy()).to_string()
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
    if formats.contains(&OutputFormat::Txt) {
        require_asciinema_convert()?;
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

fn require_asciinema_convert() -> Result<()> {
    let status = Command::new("asciinema")
        .arg("convert")
        .arg("--help")
        .status()
        .with_context(|| "failed to execute asciinema convert --help")?;
    if !status.success() {
        bail!("asciinema convert not available in this version");
    }
    Ok(())
}

fn print_versions(formats: &[OutputFormat]) -> Result<()> {
    println!("Dependencies:");
    println!("- asciinema: {}", tool_version("asciinema", &["--version"])?);
    if formats.contains(&OutputFormat::Gif) || formats.contains(&OutputFormat::Mp4) {
        println!("- agg: {}", tool_version("agg", &["--version"])?);
    }
    if formats.contains(&OutputFormat::Mp4) {
        println!("- ffmpeg: {}", tool_version("ffmpeg", &["-version"])?);
    }
    if formats.contains(&OutputFormat::Raw) {
        println!("- script: {}", tool_version("script", &["--version"])?);
    }
    Ok(())
}

fn tool_version(tool: &str, args: &[&str]) -> Result<String> {
    let output = Command::new(tool)
        .args(args)
        .output()
        .with_context(|| format!("failed to execute {tool}"))?;
    if !output.status.success() {
        return Ok("unknown".to_string());
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout.lines().next().unwrap_or("").trim();
    if line.is_empty() {
        Ok("unknown".to_string())
    } else {
        Ok(line.to_string())
    }
}

fn print_defaults() {
    println!("Defaults:");
    println!("- cols: {}", DEFAULT_COLS);
    println!("- rows: {}", DEFAULT_ROWS);
    println!("- formats: {}", DEFAULT_FORMATS);
    println!("- out_dir: {}", DEFAULT_OUT_DIR);
    println!("- name: session_YYYYMMDD_HHMMSS");
}

fn geometry_args(cols: u16, rows: u16) -> [String; 4] {
    [
        "--cols".to_string(),
        cols.to_string(),
        "--rows".to_string(),
        rows.to_string(),
    ]
}

struct OutputPaths {
    out_dir: PathBuf,
    base_name: String,
}

impl OutputPaths {
    fn new(out_dir: &Path, base_name: &str) -> Self {
        Self {
            out_dir: out_dir.to_path_buf(),
            base_name: base_name.to_string(),
        }
    }

    fn path(&self, format: OutputFormat) -> PathBuf {
        self.out_dir
            .join(format!("{}.{}", self.base_name, format.extension()))
    }
}

impl OutputFormat {
    fn extension(self) -> &'static str {
        match self {
            OutputFormat::Cast => "cast",
            OutputFormat::Txt => "txt",
            OutputFormat::Raw => "raw.log",
            OutputFormat::Gif => "gif",
            OutputFormat::Mp4 => "mp4",
        }
    }

    fn label(self) -> &'static str {
        match self {
            OutputFormat::Cast => "cast",
            OutputFormat::Txt => "text",
            OutputFormat::Raw => "raw log",
            OutputFormat::Gif => "gif",
            OutputFormat::Mp4 => "mp4",
        }
    }
}

#[cfg(not(unix))]
compile_error!("exec_and_record is only supported on Unix-like systems.");

#[cfg(test)]
mod tests {
    use super::*;

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
