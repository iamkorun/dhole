use std::path::PathBuf;
use std::process;

use clap::Parser;

use dhole::checker;
use dhole::display;
use dhole::scanner;

/// dhole — sniff out every hidden CLI dependency in your project
///
/// Scans Makefiles, shell scripts, CI configs, Dockerfiles, and docker-compose
/// files to find all external CLI tools your project depends on, then checks
/// which ones are installed.
#[derive(Parser, Debug)]
#[command(
    name = "dhole",
    version,
    about = "Sniff out every hidden CLI dependency in your project",
    after_long_help = "EXAMPLES:
    dhole                       Scan the current directory
    dhole --dir ../api          Scan a specific directory
    dhole --quiet               Exit code only — useful in CI pipelines
    dhole --verbose             Print which files were scanned

EXIT CODES:
    0   All detected tools are installed
    1   One or more tools are missing
    2   Bad input — directory does not exist, permission denied, etc.

NO_COLOR:
    Set the NO_COLOR environment variable to disable colored output."
)]
struct Cli {
    /// Directory to scan (defaults to current directory)
    #[arg(short, long, default_value = ".", value_name = "PATH")]
    dir: PathBuf,

    /// Quiet mode — no output, exit code only (useful for CI)
    #[arg(short, long, conflicts_with = "verbose")]
    quiet: bool,

    /// Show which files are being scanned
    #[arg(short, long)]
    verbose: bool,
}

fn main() {
    let cli = Cli::parse();

    // Errors always go to stderr — even in --quiet mode the user needs to
    // know why dhole bailed.
    let dir = match cli.dir.canonicalize() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("error: cannot access '{}': {}", cli.dir.display(), e);
            process::exit(2);
        }
    };

    if !dir.is_dir() {
        eprintln!("error: '{}' is not a directory", dir.display());
        process::exit(2);
    }

    if cli.verbose {
        use colored::Colorize;
        eprintln!("{} scanning {}", "verbose:".dimmed(), dir.display());
    }

    let scan_result = scanner::scan_directory(&dir);

    if cli.verbose {
        use colored::Colorize;
        eprintln!(
            "{} found {} tool reference(s) across scanned files",
            "verbose:".dimmed(),
            scan_result.len()
        );
    }

    if scan_result.is_empty() {
        if !cli.quiet {
            println!("No CLI tools detected in scanned files.");
        }
        process::exit(0);
    }

    let statuses = checker::check_all(&scan_result);
    let code = display::exit_code(&statuses);

    if !cli.quiet {
        display::print_table(&statuses);
    }

    process::exit(code);
}
