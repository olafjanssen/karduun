use predicates::prelude::*;
use std::process::Command;
use tempfile::TempDir;
use std::fs;
use std::path::PathBuf;

fn run_bin(pkg: &str, args: &[&str], _cwd: &PathBuf) -> assert_cmd::assert::Assert {
    // Run cargo from the workspace root (parent of this crate directory)
    let ws_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf();
    let mut cmd = Command::new("cargo");
    cmd.arg("run").arg("-q").arg("-p").arg(pkg).arg("--");
    for a in args { cmd.arg(a); }
    cmd.current_dir(ws_root);
    assert_cmd::Command::from_std(cmd).assert()
}

#[test]
fn scribe_and_scout_happy_path() {
    let tmp = TempDir::new().expect("tempdir");
    let repo = PathBuf::from(tmp.path());

    // scribe init
    run_bin("scribe", &["init", "--repo", repo.to_str().unwrap()], &repo)
        .success()
        .stdout(predicate::str::contains("Initialized cardstack repository"));

    // scribe new card A
    run_bin(
        "scribe",
        &["new", "First Card", "--slug", "first-card", "--tag", "example", "--repo", repo.to_str().unwrap()],
        &repo,
    )
    .success()
    .stdout(predicate::str::contains("Created card:"));

    // scribe new card B
    run_bin(
        "scribe",
        &["new", "Second Card", "--slug", "second-card", "--repo", repo.to_str().unwrap()],
        &repo,
    )
    .success();

    // discover created files (rough check)
    let cards_dir = repo.join("cards");
    assert!(cards_dir.exists());

    // scout list should report 2 cards (human-readable output)
    // Count yaml files under cards/ to assert expectation
    let mut yaml_count = 0;
    for entry in walkdir::WalkDir::new(&cards_dir) {
        let entry = entry.unwrap();
        if entry.path().is_file() && entry.path().extension().and_then(|s| s.to_str()) == Some("yaml") {
            yaml_count += 1;
        }
    }
    assert!(yaml_count >= 2, "expected at least 2 cards, found {}", yaml_count);

    run_bin(
        "scout",
        &["list", "--repo", repo.to_str().unwrap()],
        &repo,
    )
    .success()
    .stdout(predicate::str::contains("Found ").and(predicate::str::contains(" card(s)")));

    // scribe link A -> B (typed)
    // Need identifiers; do a simple grep in files to find slugs
    let mut a_uid = None;
    let mut b_uid = None;
    for entry in walkdir::WalkDir::new(&cards_dir) {
        let entry = entry.unwrap();
        if entry.path().is_file() && entry.path().extension().and_then(|s| s.to_str()) == Some("yaml") {
            let name = entry.path().file_stem().unwrap().to_string_lossy().to_string();
            if name.contains("--first-card") { a_uid = Some(name.split("--").next().unwrap().to_string()); }
            if name.contains("--second-card") { b_uid = Some(name.split("--").next().unwrap().to_string()); }
        }
    }
    let a_uid = a_uid.expect("first-card uid");
    let b_uid = b_uid.expect("second-card uid");

    run_bin(
        "scribe",
        &["link", &a_uid, "--to", &b_uid, "--type", "parent-of", "--repo", repo.to_str().unwrap()],
        &repo,
    )
    .success()
    .stdout(predicate::str::contains("Linked"));

    // scout backlinks should show A when querying B
    run_bin(
        "scout",
        &["backlinks", &b_uid, "--repo", repo.to_str().unwrap()],
        &repo,
    )
    .success()
    .stdout(predicate::str::contains(&a_uid));

    // scribe edit: add a field
    run_bin(
        "scribe",
        &["edit", &a_uid, "--field", "priority=high", "--repo", repo.to_str().unwrap()],
        &repo,
    )
    .success();

    // scribe show: ensure prints title
    run_bin(
        "scribe",
        &["show", &a_uid, "--repo", repo.to_str().unwrap()],
        &repo,
    )
    .success()
    .stdout(predicate::str::contains("Title:"));

    // scout tree should not error
    run_bin(
        "scout",
        &["tree", &a_uid, "--repo", repo.to_str().unwrap()],
        &repo,
    )
    .success();
}

#[test]
fn smoke_help_other_tools() {
    let tmp = TempDir::new().expect("tempdir");
    let repo = PathBuf::from(tmp.path());
    fs::create_dir_all(repo.join(".cardstack")).unwrap();

    // Each of these should at least show --help and exit successfully
    for (pkg, subargs) in [
        ("catalog", vec!["--help"]),
        ("gauge", vec!["--help"]),
        ("curator", vec!["--help"]),
        ("stencil", vec!["--help"]),
        ("porter", vec!["--help"]),
        ("notary", vec!["--help"]),
    ] {
        run_bin(pkg, &subargs.iter().map(|s| *s).collect::<Vec<_>>(), &repo)
            .success()
            .stdout(predicate::str::contains("Usage"));
    }
}

