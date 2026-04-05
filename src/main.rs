use std::path::PathBuf;
use std::process;

use clap::Parser;

use dhole::checker;
use dhole::display;
use dhole::scanner;

/// Scan a project for external CLI tool dependencies and check if they're installed.
#[derive(Parser, Debug)]
#[command(name = "dhole", version, about)]
struct Cli {
    /// Directory to scan (defaults to current directory)
    #[arg(short, long, default_value = ".")]
    dir: PathBuf,

    /// Quiet mode: only set exit code, no output (useful for CI)
    #[arg(short, long)]
    quiet: bool,
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

    let scan_result = scanner::scan_directory(&dir);

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
