use crate::checker::ToolStatus;
use colored::Colorize;

/// Format tool statuses as a table and print to stdout.
pub fn print_table(statuses: &[ToolStatus]) {
    if statuses.is_empty() {
        println!("{}", "No CLI tools detected in scanned files.".dimmed());
        return;
    }

    // Calculate column widths
    let name_width = statuses
        .iter()
        .map(|s| s.name.len())
        .max()
        .unwrap_or(4)
        .max(4);

    let sources_width = statuses
        .iter()
        .map(|s| format_sources(&s.sources).len())
        .max()
        .unwrap_or(8)
        .max(8);

    let installed_width = 9; // "installed" length
    let version_width = 7; // "version" length minimum

    // Header
    println!(
        "  {:<name_width$}  {:<sources_width$}  {:<installed_width$}  {}",
        "Tool".bold(),
        "Found in".bold(),
        "Installed".bold(),
        "Version".bold(),
    );
    println!(
        "  {:<name_width$}  {:<sources_width$}  {:<installed_width$}  {:<version_width$}",
        "─".repeat(name_width),
        "─".repeat(sources_width),
        "─".repeat(installed_width),
        "─".repeat(version_width),
    );

    // Rows
    for status in statuses {
        let installed_str = if status.installed {
            "yes".green().to_string()
        } else {
            "NO".red().bold().to_string()
        };

        let version_str = match &status.version {
            Some(v) => v.clone(),
            None if status.installed => "unknown".yellow().to_string(),
            None => "—".dimmed().to_string(),
        };

        let sources_str = format_sources(&status.sources);

        println!(
            "  {:<name_width$}  {:<sources_width$}  {:<installed_width$}  {}",
            status.name, sources_str, installed_str, version_str,
        );
    }

    // Summary
    let total = statuses.len();
    let installed = statuses.iter().filter(|s| s.installed).count();
    let missing = total - installed;

    println!();
    if missing == 0 {
        println!(
            "  {} All {} tools installed.",
            "✓".green().bold(),
            total
        );
    } else {
        println!(
            "  {} {}/{} tools installed, {} missing.",
            "✗".red().bold(),
            installed,
            total,
            missing.to_string().red().bold()
        );
    }
}

/// Format source file set as a comma-separated string.
fn format_sources(sources: &std::collections::BTreeSet<String>) -> String {
    let items: Vec<&str> = sources.iter().map(|s| s.as_str()).collect();
    items.join(", ")
}

/// Determine the exit code based on tool statuses.
///
/// Returns 0 if all found, 1 if any missing.
pub fn exit_code(statuses: &[ToolStatus]) -> i32 {
    if statuses.iter().all(|s| s.installed) {
        0
    } else {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn make_status(name: &str, installed: bool, version: Option<&str>) -> ToolStatus {
        ToolStatus {
            name: name.to_string(),
            sources: BTreeSet::from(["Makefile".to_string()]),
            installed,
            version: version.map(|v| v.to_string()),
        }
    }

    #[test]
    fn test_exit_code_all_installed() {
        let statuses = vec![
            make_status("git", true, Some("2.43.0")),
            make_status("curl", true, Some("8.4.0")),
        ];
        assert_eq!(exit_code(&statuses), 0);
    }

    #[test]
    fn test_exit_code_some_missing() {
        let statuses = vec![
            make_status("git", true, Some("2.43.0")),
            make_status("kubectl", false, None),
        ];
        assert_eq!(exit_code(&statuses), 1);
    }

    #[test]
    fn test_exit_code_empty() {
        let statuses: Vec<ToolStatus> = vec![];
        assert_eq!(exit_code(&statuses), 0);
    }

    #[test]
    fn test_format_sources() {
        let sources = BTreeSet::from(["Makefile".to_string(), "deploy.sh".to_string()]);
        assert_eq!(format_sources(&sources), "Makefile, deploy.sh");
    }

    #[test]
    fn test_format_sources_single() {
        let sources = BTreeSet::from(["ci.yml".to_string()]);
        assert_eq!(format_sources(&sources), "ci.yml");
    }
}
