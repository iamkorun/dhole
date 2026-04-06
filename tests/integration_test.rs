use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::tempdir;

fn dhole_cmd() -> Command {
    Command::cargo_bin("dhole").unwrap()
}

#[test]
fn test_help_flag() {
    dhole_cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("sniff out every hidden CLI"));
}

#[test]
fn test_version_flag() {
    dhole_cmd()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("dhole"));
}

#[test]
fn test_empty_directory() {
    let dir = tempdir().unwrap();
    dhole_cmd()
        .args(["-d", dir.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("No CLI tools detected"));
}

#[test]
fn test_scan_makefile_finds_tools() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("Makefile"),
        "build:\n\tdocker build -t app .\n\tcurl -s http://health\n",
    )
    .unwrap();

    dhole_cmd()
        .args(["-d", dir.path().to_str().unwrap()])
        .assert()
        .stdout(predicate::str::contains("docker"))
        .stdout(predicate::str::contains("curl"))
        .stdout(predicate::str::contains("Makefile"));
}

#[test]
fn test_quiet_mode_no_output() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("Makefile"),
        "deploy:\n\tgit push origin main\n",
    )
    .unwrap();

    let output = dhole_cmd()
        .args(["-d", dir.path().to_str().unwrap(), "-q"])
        .output()
        .unwrap();

    // Quiet mode should produce no stdout
    assert!(output.stdout.is_empty(), "quiet mode should have no stdout");
}

#[test]
fn test_exit_code_1_when_missing_tool() {
    let dir = tempdir().unwrap();
    // Reference a tool that definitely doesn't exist
    fs::write(
        dir.path().join("Makefile"),
        "check:\n\thelm version\n",
    )
    .unwrap();

    // Only assert exit code if helm is NOT installed
    let which = std::process::Command::new("which")
        .arg("helm")
        .output()
        .unwrap();

    if !which.status.success() {
        dhole_cmd()
            .args(["-d", dir.path().to_str().unwrap()])
            .assert()
            .code(1);
    }
}

#[test]
fn test_exit_code_0_when_all_installed() {
    let dir = tempdir().unwrap();
    // git should be installed everywhere
    fs::write(
        dir.path().join("Makefile"),
        "check:\n\tgit status\n",
    )
    .unwrap();

    dhole_cmd()
        .args(["-d", dir.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("git"));
}

#[test]
fn test_exit_code_2_on_invalid_dir() {
    dhole_cmd()
        .args(["-d", "/nonexistent/path/xyz123"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("error"));
}

#[test]
fn test_scan_shell_script() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("deploy.sh"),
        "#!/bin/bash\naws s3 sync . s3://bucket/\njq '.version' package.json\n",
    )
    .unwrap();

    dhole_cmd()
        .args(["-d", dir.path().to_str().unwrap()])
        .assert()
        .stdout(predicate::str::contains("aws"))
        .stdout(predicate::str::contains("jq"))
        .stdout(predicate::str::contains("deploy.sh"));
}

#[test]
fn test_scan_github_workflow() {
    let dir = tempdir().unwrap();
    let wf_dir = dir.path().join(".github").join("workflows");
    fs::create_dir_all(&wf_dir).unwrap();
    fs::write(
        wf_dir.join("ci.yml"),
        "jobs:\n  test:\n    steps:\n      - run: cargo test\n      - run: npm run lint\n",
    )
    .unwrap();

    dhole_cmd()
        .args(["-d", dir.path().to_str().unwrap()])
        .assert()
        .stdout(predicate::str::contains("cargo"))
        .stdout(predicate::str::contains("npm"));
}

#[test]
fn test_scan_dockerfile() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("Dockerfile"),
        "FROM python:3.11\nRUN pip install flask\nRUN curl -sL https://example.com\n",
    )
    .unwrap();

    dhole_cmd()
        .args(["-d", dir.path().to_str().unwrap()])
        .assert()
        .stdout(predicate::str::contains("pip"))
        .stdout(predicate::str::contains("curl"))
        .stdout(predicate::str::contains("Dockerfile"));
}

#[test]
fn test_scan_justfile() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("Justfile"),
        "build:\n    cargo build --release\n",
    )
    .unwrap();

    dhole_cmd()
        .args(["-d", dir.path().to_str().unwrap()])
        .assert()
        .stdout(predicate::str::contains("cargo"))
        .stdout(predicate::str::contains("Justfile"));
}

#[test]
fn test_nested_shell_script() {
    let dir = tempdir().unwrap();
    let scripts = dir.path().join("scripts");
    fs::create_dir(&scripts).unwrap();
    fs::write(scripts.join("deploy.sh"), "#!/bin/bash\nrsync -avz . server:/app\n").unwrap();

    dhole_cmd()
        .args(["-d", dir.path().to_str().unwrap()])
        .assert()
        .stdout(predicate::str::contains("rsync"));
}

#[test]
fn test_dedup_across_files() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("Makefile"), "test:\n\tgit log\n").unwrap();
    fs::write(dir.path().join("deploy.sh"), "#!/bin/bash\ngit push\n").unwrap();

    let output = dhole_cmd()
        .args(["-d", dir.path().to_str().unwrap()])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    // "git" should appear only once as a tool row, but sources should list both files
    let git_lines: Vec<&str> = stdout.lines().filter(|l| l.trim_start().starts_with("git")).collect();
    assert_eq!(git_lines.len(), 1, "git should appear as one row, got: {:?}", git_lines);
    // Both sources should be listed
    assert!(
        stdout.contains("Makefile") && stdout.contains("deploy.sh"),
        "Both sources should be listed"
    );
}

#[test]
fn test_quiet_and_verbose_are_mutually_exclusive() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("Makefile"), "test:\n\tgit status\n").unwrap();

    dhole_cmd()
        .args(["-d", dir.path().to_str().unwrap(), "--quiet", "--verbose"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}

#[test]
fn test_invalid_dir_error_visible_even_with_quiet() {
    // The error message must always reach stderr, even in --quiet mode,
    // otherwise the user has no idea why dhole bailed.
    dhole_cmd()
        .args(["-d", "/nonexistent/path/abc999", "--quiet"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("error"));
}

#[test]
fn test_dir_pointing_at_a_file_errors() {
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("regular-file.txt");
    fs::write(&file, "hello").unwrap();

    dhole_cmd()
        .args(["-d", file.to_str().unwrap()])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("not a directory"));
}

#[test]
fn test_verbose_writes_to_stderr() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("Makefile"), "test:\n\tgit status\n").unwrap();

    dhole_cmd()
        .args(["-d", dir.path().to_str().unwrap(), "--verbose"])
        .assert()
        .stderr(predicate::str::contains("scanning"))
        .stderr(predicate::str::contains("found"));
}

#[test]
fn test_compose_yaml_is_scanned() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("compose.yaml"),
        "services:\n  app:\n    image: alpine\n    command: rsync -avz . /backup\n",
    )
    .unwrap();

    dhole_cmd()
        .args(["-d", dir.path().to_str().unwrap()])
        .assert()
        .stdout(predicate::str::contains("rsync"))
        .stdout(predicate::str::contains("compose.yaml"));
}

#[test]
fn test_lowercase_justfile_is_scanned() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("justfile"),
        "build:\n    cargo build --release\n",
    )
    .unwrap();

    dhole_cmd()
        .args(["-d", dir.path().to_str().unwrap()])
        .assert()
        .stdout(predicate::str::contains("cargo"))
        .stdout(predicate::str::contains("justfile"));
}
