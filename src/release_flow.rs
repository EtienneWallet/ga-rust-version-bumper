use std::path::Path;

use crate::bump_logic::{is_stable, next_dev_version, parse_version, strip_dev_suffix, NextDevTarget};
use crate::git_ops;
use crate::toml_ops;

pub struct MainReleaseOutputs {
    pub new_version: String,
    pub previous_version: String,
    pub bumped: bool,
    pub next_dev_version: String,
    pub dev_advance_failed: bool,
}

pub fn run_main_branch_release(
    dev_branch: &str,
    next_dev_target: &str,
    repo_path: &Path,
    dry_run: bool,
) -> Result<MainReleaseOutputs, String> {
    // 1. Check skip
    let commit_msg = git_ops::get_head_commit_message(repo_path)?;
    if git_ops::is_skip_commit(&commit_msg) {
        println!("Skipping: detected automated version commit (message: {:?})", commit_msg);
        return Ok(MainReleaseOutputs {
            new_version: String::new(),
            previous_version: String::new(),
            bumped: false,
            next_dev_version: String::new(),
            dev_advance_failed: false,
        });
    }

    // 2. Read current version
    let (version_str, location) = toml_ops::read_version(repo_path.join("Cargo.toml").as_path())?;
    let current = parse_version(&version_str)?;
    let previous_version = version_str.clone();

    // 3. Already stable -> no-op
    if is_stable(&current) {
        println!("Version {} is already stable; nothing to do.", version_str);
        return Ok(MainReleaseOutputs {
            new_version: String::new(),
            previous_version,
            bumped: false,
            next_dev_version: String::new(),
            dev_advance_failed: false,
        });
    }

    // 4. Compute stable version
    let stable = strip_dev_suffix(&current);
    let stable_str = stable.to_string();

    // 5. Compute next dev version
    let target = NextDevTarget::from_str(next_dev_target)?;
    let next_dev = next_dev_version(&stable, target);
    let next_dev_str = next_dev.to_string();

    if dry_run {
        println!("[DRY-RUN] Would release stable {} on main", stable_str);
        println!(
            "[DRY-RUN] Would advance {} to {}",
            dev_branch, next_dev_str
        );
        return Ok(MainReleaseOutputs {
            new_version: stable_str,
            previous_version,
            bumped: true,
            next_dev_version: next_dev_str,
            dev_advance_failed: false,
        });
    }

    // 6. Write stable version to Cargo.toml
    toml_ops::write_version(
        repo_path.join("Cargo.toml").as_path(),
        location,
        &stable_str,
    )?;

    // 7. Commit and tag on main
    let tag = format!("v{}", stable_str);
    let msg = format!("chore: release stable {}", stable_str);
    git_ops::commit_and_tag(repo_path, &msg, &tag)?;

    // 8. Push main — failure is hard error
    git_ops::push_to_remote(repo_path, "main", Some(&tag))?;

    // 9. Advance dev branch
    let mut dev_advance_failed = false;

    match advance_dev_branch(repo_path, dev_branch, &next_dev_str) {
        Ok(()) => {
            println!("Dev branch {} advanced to {}", dev_branch, next_dev_str);
        }
        Err(e) => {
            eprintln!(
                "Warning: Dev branch advancement failed: {}. \
                Manual remediation: set version to {} on {} branch.",
                e, next_dev_str, dev_branch
            );
            dev_advance_failed = true;
        }
    }

    Ok(MainReleaseOutputs {
        new_version: stable_str,
        previous_version,
        bumped: true,
        next_dev_version: next_dev_str,
        dev_advance_failed,
    })
}

/// Advance the dev branch to the next development version after a stable release on main.
/// Steps:
/// 1. Checkout the dev branch
/// 2. Re-read the version location from the dev branch's Cargo.toml (may use [package] vs [workspace.package] differently than main)
/// 3. Write the next dev version
/// 4. Commit the change (no tag)
/// 5. Push to the dev branch (best-effort; failures are logged but not fatal)
fn advance_dev_branch(
    repo_path: &Path,
    dev_branch: &str,
    next_dev_str: &str,
) -> Result<(), String> {
    // Checkout dev branch
    git_ops::checkout_branch(repo_path, dev_branch)?;

    // Re-read location from dev branch's Cargo.toml — may differ from main's structure
    // (e.g., main uses [workspace.package], dev uses [package], or vice versa)
    let (_, dev_location) = toml_ops::read_version(repo_path.join("Cargo.toml").as_path())?;

    // Write next dev version
    toml_ops::write_version(
        repo_path.join("Cargo.toml").as_path(),
        dev_location,
        next_dev_str,
    )?;

    // Commit (no tag for dev advancement)
    let msg = format!("chore: start next development cycle {}", next_dev_str);
    git_ops::commit_only(repo_path, &msg)?;

    // Push dev branch (no tag for dev advancement)
    git_ops::push_to_remote(repo_path, dev_branch, None)?;

    Ok(())
}
