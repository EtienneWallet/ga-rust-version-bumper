use rust_version_bumper::bump_logic;
use rust_version_bumper::commit_parser;
use rust_version_bumper::git_ops;
use rust_version_bumper::release_flow;
use rust_version_bumper::toml_ops;

use clap::Parser;
use std::path::Path;

#[derive(Parser)]
#[command(name = "rust-version-bumper")]
struct Args {
    #[arg(long)]
    branch: String,
    #[arg(long, default_value = "false")]
    dry_run: String,
    #[arg(long, default_value = "develop")]
    dev_branch: String,
    #[arg(long, default_value = "patch")]
    next_dev_target: String,
}

fn write_output(key: &str, value: &str) {
    if let Ok(path) = std::env::var("GITHUB_OUTPUT") {
        let line = format!("{}={}\n", key, value);
        std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .and_then(|mut f| {
                use std::io::Write;
                f.write_all(line.as_bytes())
            })
            .unwrap_or_else(|e| eprintln!("Warning: failed to write to GITHUB_OUTPUT: {}", e));
    } else {
        println!("OUTPUT: {}={}", key, value);
    }
}

fn main() {
    let args = Args::parse();

    // Validate inputs
    if args.dry_run != "true" && args.dry_run != "false" {
        eprintln!("Error: --dry-run must be 'true' or 'false', got '{}'", args.dry_run);
        std::process::exit(1);
    }
    if args.next_dev_target != "patch" && args.next_dev_target != "minor" {
        eprintln!(
            "Error: --next-dev-target must be 'patch' or 'minor', got '{}'",
            args.next_dev_target
        );
        std::process::exit(1);
    }

    let dry_run = args.dry_run == "true";
    let repo_path = Path::new(".");

    if args.branch == "main" {
        match release_flow::run_main_branch_release(&args.dev_branch, &args.next_dev_target, repo_path, dry_run) {
            Ok(outputs) => {
                write_output("new-version", &outputs.new_version);
                write_output("previous-version", &outputs.previous_version);
                write_output("bumped", if outputs.bumped { "true" } else { "false" });
                write_output("next-dev-version", &outputs.next_dev_version);
                write_output(
                    "dev-advance-failed",
                    if outputs.dev_advance_failed { "true" } else { "false" },
                );
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        match run_non_main(&args, repo_path, dry_run) {
            Ok(()) => {}
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
    }
}

fn run_non_main(args: &Args, repo_path: &Path, dry_run: bool) -> Result<(), String> {
    // 1. Read HEAD commit message
    let commit_msg = git_ops::get_head_commit_message(repo_path)?;

    // 2. Skip check
    if git_ops::is_skip_commit(&commit_msg) {
        println!("Skipping: detected automated version commit (message: {:?})", commit_msg);
        write_output("new-version", "");
        write_output("previous-version", "");
        write_output("bumped", "false");
        write_output("next-dev-version", "");
        write_output("dev-advance-failed", "false");
        return Ok(());
    }

    // 3. Read current version
    let (version_str, location) = toml_ops::read_version(repo_path.join("Cargo.toml").as_path())?;
    let current = bump_logic::parse_version(&version_str)?;

    // 4. Set previous-version
    let previous_version = version_str.clone();

    // 5. Parse commit impact
    let impact = commit_parser::parse_commit(&commit_msg);

    // 6. Compute new version
    let new_version = bump_logic::compute_bump(&current, impact);
    let new_version_str = new_version.to_string();

    if dry_run {
        println!("[DRY-RUN] Would bump to {}", new_version_str);
        write_output("new-version", &new_version_str);
        write_output("previous-version", &previous_version);
        write_output("bumped", "true");
        write_output("next-dev-version", "");
        write_output("dev-advance-failed", "false");
        return Ok(());
    }

    // 7. Write new version
    toml_ops::write_version(repo_path.join("Cargo.toml").as_path(), location, &new_version_str)?;

    // 8. Commit and tag
    let tag = format!("v{}", new_version_str);
    let msg = format!("chore: bump version to {}", new_version_str);
    git_ops::commit_and_tag(repo_path, &msg, &tag)?;

    // 9. Push
    git_ops::push_to_remote(repo_path, &args.branch, Some(&tag))?;

    // 10. Set outputs
    write_output("new-version", &new_version_str);
    write_output("previous-version", &previous_version);
    write_output("bumped", "true");
    write_output("next-dev-version", "");
    write_output("dev-advance-failed", "false");

    Ok(())
}
