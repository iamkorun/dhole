use std::collections::BTreeSet;
use std::process::Command;
use std::time::Duration;

/// Result of checking a single tool's availability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolStatus {
    pub name: String,
    pub sources: BTreeSet<String>,
    pub installed: bool,
    pub version: Option<String>,
}

/// Check whether a tool is installed and retrieve its version.
pub fn check_tool(name: &str, sources: BTreeSet<String>) -> ToolStatus {
    let installed = is_installed(name);

    let version = if installed {
        get_version(name)
    } else {
        None
    };

    ToolStatus {
        name: name.to_string(),
        sources,
        installed,
        version,
    }
}

/// Check all discovered tools in parallel and return their statuses.
///
/// Each tool spawns 1-3 subprocesses (which + version commands), so doing
/// them sequentially turns into hundreds of milliseconds for a typical
/// project. We fan out one thread per tool with `thread::scope` so the
/// caller still gets a single owned `Vec<ToolStatus>`.
pub fn check_all(tools: &crate::scanner::ScanResult) -> Vec<ToolStatus> {
    let entries: Vec<(String, BTreeSet<String>)> = tools
        .iter()
        .map(|(name, sources)| (name.clone(), sources.clone()))
        .collect();

    std::thread::scope(|scope| {
        let handles: Vec<_> = entries
            .into_iter()
            .map(|(name, sources)| {
                scope.spawn(move || check_tool(&name, sources))
            })
            .collect();

        let mut results: Vec<ToolStatus> = handles
            .into_iter()
            .map(|h| h.join().expect("checker thread panicked"))
            .collect();
        results.sort_by(|a, b| a.name.cmp(&b.name));
        results
    })
}

/// Check if a tool is installed by running `which`.
fn is_installed(name: &str) -> bool {
    Command::new("which")
        .arg(name)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Try to get a version string from a tool.
///
/// Tries `--version`, then `-v`, then `version` subcommand.
/// Uses a timeout to avoid hanging on tools that block.
fn get_version(name: &str) -> Option<String> {
    let strategies = ["--version", "-v", "version"];

    for flag in &strategies {
        if let Some(version) = try_version_command(name, flag) {
            return Some(version);
        }
    }

    None
}

/// Run a version command with a timeout and extract a version string.
fn try_version_command(name: &str, flag: &str) -> Option<String> {
    let mut child = Command::new(name)
        .arg(flag)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .ok()?;

    // Wait with a timeout of 5 seconds
    let timeout = Duration::from_secs(5);
    let start = std::time::Instant::now();

    loop {
        match child.try_wait() {
            Ok(Some(_status)) => break,
            Ok(None) => {
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    return None;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(_) => return None,
        }
    }

    let output = child.wait_with_output().ok()?;
    let raw = if output.stdout.is_empty() {
        String::from_utf8_lossy(&output.stderr).to_string()
    } else {
        String::from_utf8_lossy(&output.stdout).to_string()
    };

    extract_version_string(&raw)
}

/// Extract a version-like pattern from raw output.
///
/// Looks for patterns like "1.2.3", "v1.2.3", "1.2", etc.
fn extract_version_string(raw: &str) -> Option<String> {
    let first_line = raw.lines().next().unwrap_or("").trim();
    if first_line.is_empty() {
        return None;
    }

    // Try to find a version pattern: optional 'v' followed by digits.digits[.digits[.digits...]]
    let mut best: Option<&str> = None;
    let bytes = first_line.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        let start = i;
        // skip optional 'v' or 'V'
        if i < len && (bytes[i] == b'v' || bytes[i] == b'V') {
            i += 1;
        }
        // need at least one digit
        if i >= len || !bytes[i].is_ascii_digit() {
            i = start + 1;
            continue;
        }
        // consume digits
        while i < len && bytes[i].is_ascii_digit() {
            i += 1;
        }
        // need a dot
        if i >= len || bytes[i] != b'.' {
            i = start + 1;
            continue;
        }
        i += 1;
        // need at least one digit after dot
        if i >= len || !bytes[i].is_ascii_digit() {
            i = start + 1;
            continue;
        }
        // consume rest of version: digits, dots, hyphens, plus, alphanumeric (for pre-release tags)
        while i < len
            && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'.' || bytes[i] == b'-' || bytes[i] == b'+')
        {
            i += 1;
        }
        let candidate = &first_line[start..i];
        // Pick the longest match
        if best.is_none_or(|b| candidate.len() > b.len()) {
            best = Some(candidate);
        }
    }

    best.map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_version_simple() {
        assert_eq!(
            extract_version_string("git version 2.43.0"),
            Some("2.43.0".to_string())
        );
    }

    #[test]
    fn test_extract_version_with_v_prefix() {
        assert_eq!(
            extract_version_string("v1.28.0"),
            Some("v1.28.0".to_string())
        );
    }

    #[test]
    fn test_extract_version_complex() {
        assert_eq!(
            extract_version_string("Docker version 24.0.7, build afdd53b"),
            Some("24.0.7".to_string())
        );
    }

    #[test]
    fn test_extract_version_prerelease() {
        assert_eq!(
            extract_version_string("rustc 1.75.0-nightly (abc1234 2024-01-01)"),
            Some("1.75.0-nightly".to_string())
        );
    }

    #[test]
    fn test_extract_version_none() {
        assert_eq!(extract_version_string(""), None);
        assert_eq!(extract_version_string("no version here"), None);
    }

    #[test]
    fn test_extract_version_multiline() {
        // Only first line is checked
        assert_eq!(
            extract_version_string("curl 8.4.0\nRelease-Date: 2023-10-11"),
            Some("8.4.0".to_string())
        );
    }

    #[test]
    fn test_check_tool_git() {
        // git should be installed on most systems
        let status = check_tool("git", BTreeSet::from(["Makefile".to_string()]));
        assert!(status.installed);
        assert!(status.version.is_some());
        assert_eq!(status.name, "git");
    }

    #[test]
    fn test_extract_version_python_style() {
        assert_eq!(
            extract_version_string("Python 3.14.0"),
            Some("3.14.0".to_string())
        );
    }

    #[test]
    fn test_extract_version_npm_style() {
        assert_eq!(
            extract_version_string("10.2.4"),
            Some("10.2.4".to_string())
        );
    }

    #[test]
    fn test_extract_version_go_style() {
        assert_eq!(
            extract_version_string("go version go1.22.0 linux/amd64"),
            Some("1.22.0".to_string())
        );
    }

    #[test]
    fn test_check_tool_nonexistent() {
        let status = check_tool(
            "definitely_not_a_real_tool_xyz123",
            BTreeSet::from(["test.sh".to_string()]),
        );
        assert!(!status.installed);
        assert!(status.version.is_none());
    }
}
