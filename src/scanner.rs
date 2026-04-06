use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

/// Known CLI tools to detect in project files.
///
/// Keep alphabetised. Tools that are also common English words
/// (`find`, `convert`, `make`) are intentionally included — the
/// word-boundary matcher prevents most false positives, and the
/// cost of a false negative ("forgot to install make") is higher
/// than the cost of a false positive in a comment.
const KNOWN_TOOLS: &[&str] = &[
    "ansible",
    "awk",
    "aws",
    "az",
    "bash",
    "bun",
    "bundle",
    "cargo",
    "cmake",
    "composer",
    "convert",
    "curl",
    "deno",
    "docker",
    "docker-compose",
    "ffmpeg",
    "find",
    "g++",
    "gcc",
    "gcloud",
    "gem",
    "gh",
    "git",
    "go",
    "gradle",
    "grep",
    "helm",
    "java",
    "jq",
    "kubectl",
    "make",
    "mongosh",
    "mvn",
    "mysql",
    "nginx",
    "node",
    "npm",
    "npx",
    "openssl",
    "pg_dump",
    "php",
    "pip",
    "pnpm",
    "podman",
    "poetry",
    "psql",
    "python",
    "python3",
    "redis-cli",
    "rsync",
    "ruby",
    "rustc",
    "scp",
    "sed",
    "ssh",
    "tar",
    "terraform",
    "unzip",
    "uv",
    "wget",
    "yarn",
    "yq",
    "zip",
    "zsh",
];

/// Scan result: map of tool name -> set of source files.
pub type ScanResult = BTreeMap<String, BTreeSet<String>>;

/// Scan a project directory for CLI tool references.
///
/// Returns a map of tool_name -> set of source file paths (relative to `dir`).
pub fn scan_directory(dir: &Path) -> ScanResult {
    let mut result: ScanResult = BTreeMap::new();

    let files = collect_scannable_files(dir);
    for file_path in &files {
        let relative = file_path
            .strip_prefix(dir)
            .unwrap_or(file_path)
            .to_string_lossy()
            .to_string();

        let content = match fs::read_to_string(file_path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let tools = extract_tools(&content);
        for tool in tools {
            result
                .entry(tool)
                .or_default()
                .insert(relative.clone());
        }
    }

    result
}

/// Collect files worth scanning from a project directory.
fn collect_scannable_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();

    // Specific files to check at root
    let root_files = [
        "Makefile",
        "makefile",
        "GNUmakefile",
        "docker-compose.yml",
        "docker-compose.yaml",
        "compose.yml",
        "compose.yaml",
        ".gitlab-ci.yml",
        "Dockerfile",
        "Containerfile",
        "Justfile",
        "justfile",
        "Taskfile.yml",
        "Taskfile.yaml",
    ];

    for name in &root_files {
        let path = dir.join(name);
        if path.is_file() {
            files.push(path);
        }
    }

    // GitHub Actions workflows
    let workflows_dir = dir.join(".github").join("workflows");
    if workflows_dir.is_dir()
        && let Ok(entries) = fs::read_dir(&workflows_dir)
    {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                if ext == "yml" || ext == "yaml" {
                    files.push(path);
                }
            }
        }
    }

    // Shell scripts (recursively, but skip hidden dirs and common noise)
    collect_shell_scripts(dir, &mut files, 0);

    files
}

/// Recursively collect shell scripts, skipping hidden directories,
/// node_modules, target, etc. Catches common shell extensions.
fn collect_shell_scripts(dir: &Path, files: &mut Vec<PathBuf>, depth: u32) {
    if depth > 5 {
        return;
    }

    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        // Skip hidden dirs, build artifacts, vendored deps.
        if name.starts_with('.')
            || matches!(
                name.as_str(),
                "node_modules" | "target" | "vendor" | "__pycache__" | "dist" | "build"
            )
        {
            continue;
        }

        if path.is_dir() {
            collect_shell_scripts(&path, files, depth + 1);
        } else if path.is_file() && is_shell_script(&name) {
            files.push(path);
        }
    }
}

/// True if the filename looks like a shell script.
fn is_shell_script(name: &str) -> bool {
    name.ends_with(".sh") || name.ends_with(".bash") || name.ends_with(".zsh")
}

/// Extract known CLI tool names from file content using word boundary matching.
fn extract_tools(content: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();

    for tool in KNOWN_TOOLS {
        if content_contains_tool(content, tool) {
            found.insert(tool.to_string());
        }
    }

    found
}

/// Check if content contains a tool name at a word boundary.
///
/// A tool is considered referenced if it appears as a standalone word,
/// not as a substring of another identifier. We check that the character
/// before and after the match is not alphanumeric or underscore.
fn content_contains_tool(content: &str, tool: &str) -> bool {
    let tool_bytes = tool.as_bytes();
    let content_bytes = content.as_bytes();
    let tool_len = tool_bytes.len();

    if content_bytes.len() < tool_len {
        return false;
    }

    let mut i = 0;
    while i <= content_bytes.len() - tool_len {
        if &content_bytes[i..i + tool_len] == tool_bytes {
            let before_ok = if i == 0 {
                true
            } else {
                !is_word_char(content_bytes[i - 1])
            };

            let after_ok = if i + tool_len >= content_bytes.len() {
                true
            } else {
                !is_word_char(content_bytes[i + tool_len])
            };

            if before_ok && after_ok {
                return true;
            }
        }
        i += 1;
    }

    false
}

/// Check if a byte is a word character (alphanumeric or underscore).
/// Hyphens are NOT word chars so `docker-compose` is matched correctly.
fn is_word_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_extract_tools_from_makefile_content() {
        let content = "build:\n\tdocker build -t myapp .\n\tkubectl apply -f deploy.yml\n";
        let tools = extract_tools(content);
        assert!(tools.contains("docker"));
        assert!(tools.contains("kubectl"));
    }

    #[test]
    fn test_word_boundary_matching() {
        // "node" should not match "node_modules" as a standalone tool
        // but "node" alone should match
        let content = "node_modules/foo";
        let tools = extract_tools(content);
        assert!(!tools.contains("node"), "node should not match inside node_modules");

        let content2 = "node --version";
        let tools2 = extract_tools(content2);
        assert!(tools2.contains("node"));
    }

    #[test]
    fn test_hyphenated_tools() {
        let content = "docker-compose up -d";
        let tools = extract_tools(content);
        assert!(tools.contains("docker-compose"));
        // "docker" should also match since "docker-" has a hyphen after it,
        // but hyphen is not a word char, so "docker" is at a boundary
        assert!(tools.contains("docker"));
    }

    #[test]
    fn test_no_false_positives() {
        let content = "This is a normal sentence with no commands.";
        let tools = extract_tools(content);
        assert!(tools.is_empty());
    }

    #[test]
    fn test_scan_directory_with_makefile() {
        let dir = tempfile::tempdir().unwrap();
        let makefile = dir.path().join("Makefile");
        fs::write(&makefile, "deploy:\n\tkubectl apply -f k8s/\n\tcurl -X POST http://webhook\n").unwrap();

        let result = scan_directory(dir.path());
        assert!(result.contains_key("kubectl"));
        assert!(result.contains_key("curl"));
        assert!(result["kubectl"].contains("Makefile"));
    }

    #[test]
    fn test_scan_shell_scripts() {
        let dir = tempfile::tempdir().unwrap();
        let script = dir.path().join("deploy.sh");
        fs::write(&script, "#!/bin/bash\naws s3 sync . s3://bucket\njq '.version' package.json\n").unwrap();

        let result = scan_directory(dir.path());
        assert!(result.contains_key("aws"));
        assert!(result.contains_key("jq"));
    }

    #[test]
    fn test_scan_github_workflows() {
        let dir = tempfile::tempdir().unwrap();
        let wf_dir = dir.path().join(".github").join("workflows");
        fs::create_dir_all(&wf_dir).unwrap();
        fs::write(wf_dir.join("ci.yml"), "steps:\n  - run: cargo test\n  - run: npm run build\n").unwrap();

        let result = scan_directory(dir.path());
        assert!(result.contains_key("cargo"));
        assert!(result.contains_key("npm"));
    }

    #[test]
    fn test_scan_docker_compose() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("docker-compose.yml"),
            "services:\n  app:\n    build: .\n  redis:\n    image: redis-cli\n",
        )
        .unwrap();

        let result = scan_directory(dir.path());
        // docker-compose.yml itself references docker-compose implicitly
        // but we scan content, so this checks for tools mentioned in content
        assert!(result.contains_key("redis-cli"));
    }

    #[test]
    fn test_deduplication_across_files() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("Makefile"), "test:\n\tcurl http://api\n").unwrap();

        let script = dir.path().join("setup.sh");
        fs::write(&script, "#!/bin/bash\ncurl -s http://health\n").unwrap();

        let result = scan_directory(dir.path());
        assert!(result.contains_key("curl"));
        assert!(result["curl"].len() >= 2, "curl should be found in both files");
    }

    #[test]
    fn test_empty_directory() {
        let dir = tempfile::tempdir().unwrap();
        let result = scan_directory(dir.path());
        assert!(result.is_empty());
    }

    #[test]
    fn test_scan_dockerfile() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("Dockerfile"),
            "FROM node:20\nRUN npm install\nRUN curl -sL https://example.com | tar xz\n",
        )
        .unwrap();

        let result = scan_directory(dir.path());
        assert!(result.contains_key("npm"));
        assert!(result.contains_key("curl"));
        assert!(result.contains_key("tar"));
    }

    #[test]
    fn test_scan_justfile() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("Justfile"),
            "deploy:\n    kubectl apply -f k8s/\n    helm upgrade app chart/\n",
        )
        .unwrap();

        let result = scan_directory(dir.path());
        assert!(result.contains_key("kubectl"));
        assert!(result.contains_key("helm"));
    }

    #[test]
    fn test_scan_nested_shell_scripts() {
        let dir = tempfile::tempdir().unwrap();
        let scripts_dir = dir.path().join("scripts");
        fs::create_dir(&scripts_dir).unwrap();
        fs::write(scripts_dir.join("setup.sh"), "#!/bin/bash\npip install -r requirements.txt\n").unwrap();

        let result = scan_directory(dir.path());
        assert!(result.contains_key("pip"));
        assert!(result["pip"].iter().any(|s| s.contains("scripts/setup.sh")));
    }

    #[test]
    fn test_scan_skips_node_modules() {
        let dir = tempfile::tempdir().unwrap();
        let nm_dir = dir.path().join("node_modules").join("foo");
        fs::create_dir_all(&nm_dir).unwrap();
        fs::write(nm_dir.join("build.sh"), "#!/bin/bash\ncargo build\n").unwrap();

        let result = scan_directory(dir.path());
        assert!(!result.contains_key("cargo"), "should skip node_modules");
    }

    #[test]
    fn test_content_contains_tool_edge_cases() {
        assert!(content_contains_tool("git push", "git"));
        assert!(content_contains_tool("run git push", "git"));
        assert!(!content_contains_tool("digit", "git"));
        assert!(content_contains_tool("git", "git"));
        assert!(!content_contains_tool("", "git"));
    }
}
