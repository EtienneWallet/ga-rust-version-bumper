use rust_version_bumper::bump_logic::{compute_bump, parse_version, NextDevTarget, next_dev_version};
use rust_version_bumper::commit_parser::{parse_commit, CommitImpact};
use rust_version_bumper::git_ops::{commit_and_tag, commit_only, get_head_commit_message, is_skip_commit};
use rust_version_bumper::release_flow::run_main_branch_release;
use rust_version_bumper::toml_ops::{read_version, write_version, VersionLocation};

use git2::{Repository, Signature};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Create a temp git repo, write a Cargo.toml with the given version, commit with the given message.
fn setup_test_repo(initial_version: &str, commit_message: &str) -> (TempDir, PathBuf) {
    let dir = TempDir::new().unwrap();
    let repo = Repository::init(dir.path()).unwrap();

    let cargo_content = format!(
        "[package]\nname = \"test\"\nversion = \"{}\"\nedition = \"2021\"\n",
        initial_version
    );
    let cargo_path = dir.path().join("Cargo.toml");
    fs::write(&cargo_path, &cargo_content).unwrap();

    let sig = Signature::now("Test", "test@test.com").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("Cargo.toml")).unwrap();
    index.write().unwrap();
    let tree_oid = index.write_tree().unwrap();
    {
        let tree = repo.find_tree(tree_oid).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, commit_message, &tree, &[]).unwrap();
    }

    let path = dir.path().to_path_buf();
    (dir, path)
}

// --- Bump logic integration: covers the spec scenario table ---

#[test]
fn test_dev_bump_no_suffix() {
    let v = parse_version("0.1.0").unwrap();
    let result = compute_bump(&v, CommitImpact::Dev);
    assert_eq!(result.to_string(), "0.1.0-dev0");
}

#[test]
fn test_dev_bump_with_suffix() {
    let v = parse_version("1.2.3-dev5").unwrap();
    let result = compute_bump(&v, CommitImpact::Dev);
    assert_eq!(result.to_string(), "1.2.3-dev6");
}

#[test]
fn test_feat_major_zero_patch_gt0() {
    // major==0 doesn't block feat; patch > 0 → minor bump
    let v = parse_version("0.1.3-dev2").unwrap();
    let result = compute_bump(&v, CommitImpact::Feature);
    assert_eq!(result.to_string(), "0.2.0-dev0");
}

#[test]
fn test_feat_patch_zero() {
    let v = parse_version("1.2.0-dev3").unwrap();
    let result = compute_bump(&v, CommitImpact::Feature);
    assert_eq!(result.to_string(), "1.2.0-dev4");
}

#[test]
fn test_feat_minor_bump() {
    let v = parse_version("1.2.5-dev3").unwrap();
    let result = compute_bump(&v, CommitImpact::Feature);
    assert_eq!(result.to_string(), "1.3.0-dev0");
}

#[test]
fn test_breaking_major_bump() {
    let v = parse_version("1.5.2-dev1").unwrap();
    let result = compute_bump(&v, CommitImpact::Breaking);
    assert_eq!(result.to_string(), "2.0.0-dev0");
}

#[test]
fn test_plain_refactor_major_bump() {
    let v = parse_version("1.5.2-dev1").unwrap();
    let result = compute_bump(&v, CommitImpact::Refactor);
    assert_eq!(result.to_string(), "2.0.0-dev0");
}

#[test]
fn test_breaking_major_zero() {
    // major==0 downgrades major to minor; patch > 0 → minor bump
    let v = parse_version("0.5.2-dev3").unwrap();
    let result = compute_bump(&v, CommitImpact::Refactor);
    assert_eq!(result.to_string(), "0.6.0-dev0");
}

#[test]
fn test_breaking_at_x00() {
    let v = parse_version("1.0.0-dev5").unwrap();
    let result = compute_bump(&v, CommitImpact::Refactor);
    assert_eq!(result.to_string(), "1.0.0-dev6");
}

#[test]
fn test_next_dev_patch_target() {
    let v = parse_version("1.3.0").unwrap();
    let result = next_dev_version(&v, NextDevTarget::Patch);
    assert_eq!(result.to_string(), "1.3.1-dev0");
}

#[test]
fn test_next_dev_minor_target() {
    let v = parse_version("1.3.0").unwrap();
    let result = next_dev_version(&v, NextDevTarget::Minor);
    assert_eq!(result.to_string(), "1.4.0-dev0");
}

// --- Skip commit detection ---

#[test]
fn test_skip_bump_commit() {
    assert!(is_skip_commit("chore: bump version to 1.2.3-dev6"));
}

#[test]
fn test_skip_release_commit() {
    assert!(is_skip_commit("chore: release stable 1.3.0"));
}

#[test]
fn test_skip_dev_cycle_commit() {
    assert!(is_skip_commit("chore: start next development cycle 1.3.1-dev0"));
}

// --- TOML read/write ---

#[test]
fn test_malformed_toml() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("Cargo.toml");
    fs::write(&path, "not valid toml ][").unwrap();
    assert!(read_version(&path).is_err());
}

#[test]
fn test_no_version_field() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("Cargo.toml");
    fs::write(&path, "[package]\nname = \"foo\"\n").unwrap();
    let err = read_version(&path).unwrap_err();
    assert!(err.contains("Version field not found"));
}

// --- Commit parser ---

#[test]
fn test_non_conventional_commit() {
    let result = parse_commit("random text");
    assert_eq!(result, CommitImpact::Dev);
}

// --- Git repo: commit-and-tag flow ---

#[test]
fn test_commit_creates_tag_and_advances_head() {
    let (_dir, repo_path) = setup_test_repo("1.2.3-dev5", "fix: handle null");

    // Simulate writing a new version
    let cargo_path = repo_path.join("Cargo.toml");
    write_version(&cargo_path, VersionLocation::Package, "1.2.3-dev6").unwrap();

    commit_and_tag(&repo_path, "chore: bump version to 1.2.3-dev6", "v1.2.3-dev6").unwrap();

    let msg = get_head_commit_message(&repo_path).unwrap();
    assert_eq!(msg, "chore: bump version to 1.2.3-dev6");

    // Verify version in file
    let (ver, _) = read_version(&cargo_path).unwrap();
    assert_eq!(ver, "1.2.3-dev6");

    // Verify tag exists
    let repo = Repository::open(&repo_path).unwrap();
    assert!(repo.find_reference("refs/tags/v1.2.3-dev6").is_ok());
}

#[test]
fn test_dry_run_produces_no_commit() {
    let (_dir, repo_path) = setup_test_repo("1.2.3-dev5", "fix: handle null");
    let repo = Repository::open(&repo_path).unwrap();

    // Record initial HEAD
    let initial_head = repo.head().unwrap().peel_to_commit().unwrap().id();

    // In dry-run mode, we only compute and log, do NOT call commit_and_tag
    let v = parse_version("1.2.3-dev5").unwrap();
    let new_v = compute_bump(&v, CommitImpact::Dev);
    assert_eq!(new_v.to_string(), "1.2.3-dev6");

    // HEAD should be unchanged
    let current_head = repo.head().unwrap().peel_to_commit().unwrap().id();
    assert_eq!(initial_head, current_head, "dry-run must not create commits");
}

#[test]
fn test_commit_only_creates_no_tag() {
    let (_dir, repo_path) = setup_test_repo("1.3.1-dev0", "chore: release stable 1.3.0");
    let cargo_path = repo_path.join("Cargo.toml");
    write_version(&cargo_path, VersionLocation::Package, "1.3.1-dev0").unwrap();

    commit_only(&repo_path, "chore: start next development cycle 1.3.1-dev0").unwrap();

    let msg = get_head_commit_message(&repo_path).unwrap();
    assert_eq!(msg, "chore: start next development cycle 1.3.1-dev0");

    // Verify NO tag was created
    let repo = Repository::open(&repo_path).unwrap();
    let tags: Vec<_> = repo.tag_names(None).unwrap().iter().flatten().map(String::from).collect();
    assert!(tags.is_empty(), "commit_only must not create any tags");
}

// --- release_flow integration tests (dry-run to avoid network) ---

#[test]
fn test_main_release_dry_run_strips_dev_suffix() {
    let (_dir, repo_path) = setup_test_repo("1.3.0-dev10", "feat: final feature");

    let outputs = run_main_branch_release("develop", "patch", &repo_path, true).unwrap();

    assert_eq!(outputs.new_version, "1.3.0");
    assert_eq!(outputs.previous_version, "1.3.0-dev10");
    assert!(outputs.bumped);
    assert_eq!(outputs.next_dev_version, "1.3.1-dev0");
    assert!(!outputs.dev_advance_failed);
}

#[test]
fn test_main_release_dry_run_minor_target() {
    let (_dir, repo_path) = setup_test_repo("1.3.0-dev10", "feat: final feature");

    let outputs = run_main_branch_release("develop", "minor", &repo_path, true).unwrap();

    assert_eq!(outputs.new_version, "1.3.0");
    assert_eq!(outputs.next_dev_version, "1.4.0-dev0");
    assert!(outputs.bumped);
}

#[test]
fn test_main_release_already_stable_is_noop() {
    let (_dir, repo_path) = setup_test_repo("1.3.0", "feat: some feature");

    let outputs = run_main_branch_release("develop", "patch", &repo_path, true).unwrap();

    assert_eq!(outputs.new_version, "");
    assert!(!outputs.bumped);
    assert_eq!(outputs.next_dev_version, "");
    assert!(!outputs.dev_advance_failed);
}

#[test]
fn test_main_release_skip_on_skip_commit() {
    let (_dir, repo_path) =
        setup_test_repo("1.3.0-dev10", "chore: release stable 1.3.0");

    let outputs = run_main_branch_release("develop", "patch", &repo_path, true).unwrap();

    assert_eq!(outputs.new_version, "");
    assert!(!outputs.bumped);
}

#[test]
fn test_main_release_dry_run_no_commit_created() {
    let (_dir, repo_path) = setup_test_repo("1.3.0-dev10", "feat: done");
    let repo = Repository::open(&repo_path).unwrap();
    let initial_head = repo.head().unwrap().peel_to_commit().unwrap().id();

    run_main_branch_release("develop", "patch", &repo_path, true).unwrap();

    let current_head = repo.head().unwrap().peel_to_commit().unwrap().id();
    assert_eq!(initial_head, current_head, "dry-run must not create commits on main");
}
