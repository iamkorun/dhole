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
)]
struct Cli {
    /// Directory to scan (defaults to current directory)
    #[arg(short, long, default_value = ".")]
    dir: PathBuf,

    /// Quiet mode: only set exit code, no output (useful for CI)
    #[arg(short, long)]
    quiet: bool,

    /// Show which files are being scanned
    #[arg(short, long)]
    verbose: bool,
}

fn main() {
    let cli = Cli::parse();

    let dir = match cli.dir.canonicalize() {
        Ok(d) => d,
        Err(e) => {
            if !cli.quiet {
                eprintln!("Error: cannot access directory '{}': {}", cli.dir.display(), e);
            }
            process::exit(2);
        }
    };

    if !dir.is_dir() {
        if !cli.quiet {
            eprintln!("Error: '{}' is not a directory", dir.display());
        }
        process::exit(2);
    }

    let verbose = cli.verbose && !cli.quiet;

    if verbose {
        use colored::Colorize;
        eprintln!("{} scanning {}", "verbose:".dimmed(), dir.display());
    }

    let scan_result = scanner::scan_directory(&dir);

    if verbose {
        use colored::Colorize;
        eprintln!("{} found {} tool(s) across scanned files", "verbose:".dimmed(), scan_result.len());
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
