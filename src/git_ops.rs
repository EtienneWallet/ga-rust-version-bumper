use git2::{IndexAddOption, PushOptions, RemoteCallbacks, Repository, Signature};
use std::path::Path;

pub fn is_file_tracked(repo_path: &Path, file_path: &Path) -> Result<bool, String> {
    let repo =
        Repository::open(repo_path).map_err(|e| format!("Failed to open git repository: {}", e))?;

    let head = match repo.head() {
        Ok(h) => h,
        Err(_) => return Ok(false),
    };

    let tree = head
        .peel_to_tree()
        .map_err(|e| format!("Failed to peel HEAD to tree: {}", e))?;

    let file_str = file_path
        .to_str()
        .ok_or_else(|| "Invalid file path".to_string())?;

    Ok(tree.get_path(Path::new(file_str)).is_ok())
}

pub fn get_head_commit_message(repo_path: &Path) -> Result<String, String> {
    let repo = Repository::open(repo_path).map_err(|e| {
        format!(
            "Failed to open git repository at {}: {}",
            repo_path.display(),
            e
        )
    })?;

    let head = repo
        .head()
        .map_err(|e| format!("Failed to get HEAD: {}", e))?;

    let commit = head
        .peel_to_commit()
        .map_err(|e| format!("Failed to peel HEAD to commit: {}", e))?;

    let message = commit.message().unwrap_or("").trim().to_string();

    Ok(message)
}

pub fn is_skip_commit(message: &str) -> bool {
    message.starts_with("chore: bump version to ")
        || message.starts_with("chore: release stable ")
        || message.starts_with("chore: start next development cycle ")
}

pub fn configure_git_identity(repo: &Repository) -> Result<(), String> {
    let mut config = repo
        .config()
        .map_err(|e| format!("Failed to open repo config: {}", e))?;

    config
        .set_str("user.name", "github-actions[bot]")
        .map_err(|e| format!("Failed to set user.name: {}", e))?;

    config
        .set_str("user.email", "github-actions[bot]@users.noreply.github.com")
        .map_err(|e| format!("Failed to set user.email: {}", e))?;

    Ok(())
}

/// Stage Cargo.toml, create a commit, and create a lightweight tag.
/// Used for version bumps that should be tagged (dev/minor/major bumps on non-main, stable releases on main).
pub fn commit_and_tag(repo_path: &Path, message: &str, tag: &str) -> Result<(), String> {
    commit_with_optional_tag(repo_path, message, Some(tag))
}

/// Stage Cargo.toml and create a commit without a tag.
/// Used for dev branch advancement commits, which should not have tags.
pub fn commit_only(repo_path: &Path, message: &str) -> Result<(), String> {
    commit_with_optional_tag(repo_path, message, None)
}

fn commit_with_optional_tag(
    repo_path: &Path,
    message: &str,
    tag: Option<&str>,
) -> Result<(), String> {
    let repo =
        Repository::open(repo_path).map_err(|e| format!("Failed to open git repository: {}", e))?;

    configure_git_identity(&repo)?;

    let sig = Signature::now(
        "github-actions[bot]",
        "github-actions[bot]@users.noreply.github.com",
    )
    .map_err(|e| format!("Failed to create signature: {}", e))?;

    // Stage Cargo.toml
    let mut index = repo
        .index()
        .map_err(|e| format!("Failed to get index: {}", e))?;

    index
        .add_all(
            ["Cargo.toml", "Cargo.lock"].iter(),
            IndexAddOption::DEFAULT,
            None,
        )
        .map_err(|e| format!("Failed to stage Cargo.toml: {}", e))?;

    index
        .write()
        .map_err(|e| format!("Failed to write index: {}", e))?;

    let tree_oid = index
        .write_tree()
        .map_err(|e| format!("Failed to write tree: {}", e))?;

    let tree = repo
        .find_tree(tree_oid)
        .map_err(|e| format!("Failed to find tree: {}", e))?;

    // Get parent commit (HEAD)
    let parent_commit = match repo.head() {
        Ok(head) => Some(
            head.peel_to_commit()
                .map_err(|e| format!("Failed to peel HEAD to commit: {}", e))?,
        ),
        Err(_) => None,
    };

    let parents: Vec<&git2::Commit> = parent_commit.iter().collect();

    let commit_oid = repo
        .commit(Some("HEAD"), &sig, &sig, message, &tree, &parents)
        .map_err(|e| format!("Failed to create commit: {}", e))?;

    if let Some(tag_name) = tag {
        let commit_obj = repo
            .find_object(commit_oid, Some(git2::ObjectType::Commit))
            .map_err(|e| format!("Failed to find commit object: {}", e))?;

        repo.tag_lightweight(tag_name, &commit_obj, false)
            .map_err(|e| format!("Failed to create tag '{}': {}", tag_name, e))?;
    }

    Ok(())
}

/// Push branch (and optionally a specific tag) to the remote origin.
/// If `tag` is Some, only that tag is pushed; if None, no tags are pushed.
/// This avoids unnecessary network traffic and prevents pushing unrelated tags.
/// Requires GITHUB_TOKEN and GITHUB_REPOSITORY environment variables.
pub fn push_to_remote(repo_path: &Path, branch: &str, tag: Option<&str>) -> Result<(), String> {
    let token = std::env::var("GITHUB_TOKEN")
        .map_err(|_| "GITHUB_TOKEN environment variable not set".to_string())?;

    let repo_name = std::env::var("GITHUB_REPOSITORY")
        .map_err(|_| "GITHUB_REPOSITORY environment variable not set".to_string())?;

    let remote_url = format!(
        "https://x-access-token:{}@github.com/{}.git",
        token, repo_name
    );

    let repo =
        Repository::open(repo_path).map_err(|e| format!("Failed to open git repository: {}", e))?;

    let mut remote = repo
        .remote_anonymous(&remote_url)
        .map_err(|e| format!("Failed to create remote: {}", e))?;

    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(|_url, _username, _allowed| {
        git2::Cred::userpass_plaintext("x-access-token", &token)
    });

    let mut push_opts = PushOptions::new();
    push_opts.remote_callbacks(callbacks);

    let branch_refspec = format!("refs/heads/{}:refs/heads/{}", branch, branch);
    let mut refspecs = vec![branch_refspec];

    if let Some(tag_name) = tag {
        refspecs.push(format!("refs/tags/{}:refs/tags/{}", tag_name, tag_name));
    }

    println!("Pushing to origin/{}", branch);

    remote
        .push(&refspecs, Some(&mut push_opts))
        .map_err(|e| format!("Failed to push to origin/{}: {}", branch, e))?;

    Ok(())
}

pub fn checkout_branch(repo_path: &Path, branch: &str) -> Result<(), String> {
    let token = std::env::var("GITHUB_TOKEN")
        .map_err(|_| "GITHUB_TOKEN environment variable not set".to_string())?;

    let repo_name = std::env::var("GITHUB_REPOSITORY")
        .map_err(|_| "GITHUB_REPOSITORY environment variable not set".to_string())?;

    let remote_url = format!(
        "https://x-access-token:{}@github.com/{}.git",
        token, repo_name
    );

    let repo =
        Repository::open(repo_path).map_err(|e| format!("Failed to open git repository: {}", e))?;

    // Fetch the branch from remote
    let mut remote = repo
        .remote_anonymous(&remote_url)
        .map_err(|e| format!("Failed to create remote for fetch: {}", e))?;

    let mut fetch_opts = git2::FetchOptions::new();
    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(|_url, _username, _allowed| {
        git2::Cred::userpass_plaintext("x-access-token", &token)
    });
    fetch_opts.remote_callbacks(callbacks);

    let refspec = format!("refs/heads/{}:refs/remotes/origin/{}", branch, branch);
    remote
        .fetch(&[&refspec], Some(&mut fetch_opts), None)
        .map_err(|e| format!("Failed to fetch branch '{}': {}", branch, e))?;

    // Find the remote tracking ref
    let remote_ref = format!("refs/remotes/origin/{}", branch);
    let remote_commit = repo
        .find_reference(&remote_ref)
        .map_err(|_| format!("Branch '{}' not found on remote", branch))?
        .peel_to_commit()
        .map_err(|e| format!("Failed to peel remote ref to commit: {}", e))?;

    // Create or reset local branch
    let local_ref = format!("refs/heads/{}", branch);
    if repo.find_reference(&local_ref).is_ok() {
        repo.find_reference(&local_ref)
            .unwrap()
            .set_target(remote_commit.id(), &format!("reset to origin/{}", branch))
            .map_err(|e| format!("Failed to reset local branch: {}", e))?;
    } else {
        repo.branch(branch, &remote_commit, false)
            .map_err(|e| format!("Failed to create local branch '{}': {}", branch, e))?;
    }

    // Checkout the branch
    let obj = repo
        .revparse_single(&format!("refs/heads/{}", branch))
        .map_err(|e| format!("Failed to find branch ref: {}", e))?;

    repo.checkout_tree(&obj, None)
        .map_err(|e| format!("Failed to checkout tree for branch '{}': {}", branch, e))?;

    repo.set_head(&local_ref)
        .map_err(|e| format!("Failed to set HEAD to '{}': {}", branch, e))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::{Repository, Signature};
    use std::fs;
    use tempfile::TempDir;

    fn setup_repo_with_commit(message: &str) -> (TempDir, Repository) {
        let dir = TempDir::new().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        // Need at least one file to commit
        let file_path = dir.path().join("README.md");
        fs::write(&file_path, "test").unwrap();

        let sig = Signature::now("Test", "test@test.com").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new("README.md")).unwrap();
        index.write().unwrap();
        let tree_oid = index.write_tree().unwrap();
        {
            let tree = repo.find_tree(tree_oid).unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[])
                .unwrap();
        }

        (dir, repo)
    }

    #[test]
    fn test_is_file_tracked_true() {
        let (dir, _repo) = setup_repo_with_commit("init");
        // README.md was committed in setup
        assert!(is_file_tracked(dir.path(), Path::new("README.md")).unwrap());
    }

    #[test]
    fn test_is_file_tracked_false() {
        let (dir, _repo) = setup_repo_with_commit("init");
        assert!(!is_file_tracked(dir.path(), Path::new("Cargo.lock")).unwrap());
    }

    #[test]
    fn test_get_head_commit_message() {
        let (dir, _repo) = setup_repo_with_commit("fix: test commit");
        let msg = get_head_commit_message(dir.path()).unwrap();
        assert_eq!(msg, "fix: test commit");
    }

    #[test]
    fn test_is_skip_commit_bump() {
        assert!(is_skip_commit("chore: bump version to 1.2.3-dev5"));
    }

    #[test]
    fn test_is_skip_commit_release() {
        assert!(is_skip_commit("chore: release stable 1.3.0"));
    }

    #[test]
    fn test_is_skip_commit_dev_cycle() {
        assert!(is_skip_commit(
            "chore: start next development cycle 1.3.1-dev0"
        ));
    }

    #[test]
    fn test_is_skip_commit_non_matching() {
        assert!(!is_skip_commit("feat: add new feature"));
        assert!(!is_skip_commit("fix: bug fix"));
    }

    #[test]
    fn test_configure_git_identity() {
        let (dir, repo) = setup_repo_with_commit("init");
        configure_git_identity(&repo).unwrap();
        let config = repo.config().unwrap();
        assert_eq!(
            config.get_string("user.name").unwrap(),
            "github-actions[bot]"
        );
        assert_eq!(
            config.get_string("user.email").unwrap(),
            "github-actions[bot]@users.noreply.github.com"
        );
        drop(dir);
    }

    #[test]
    fn test_commit_and_tag() {
        let (dir, _repo) = setup_repo_with_commit("init: initial commit");
        let cargo_path = dir.path().join("Cargo.toml");
        fs::write(&cargo_path, "[package]\nversion = \"1.0.0\"\n").unwrap();

        commit_and_tag(
            dir.path(),
            "chore: bump version to 1.0.1-dev0",
            "v1.0.1-dev0",
        )
        .unwrap();

        let repo = Repository::open(dir.path()).unwrap();
        // Check tag exists
        let tag_ref = repo.find_reference("refs/tags/v1.0.1-dev0");
        assert!(tag_ref.is_ok(), "Tag should exist");

        // Check commit message
        let msg = get_head_commit_message(dir.path()).unwrap();
        assert_eq!(msg, "chore: bump version to 1.0.1-dev0");
    }

    #[test]
    fn test_push_to_remote_missing_token() {
        // Remove GITHUB_TOKEN from env if present, ensure error is returned
        let original = std::env::var("GITHUB_TOKEN").ok();
        std::env::remove_var("GITHUB_TOKEN");
        let dir = TempDir::new().unwrap();
        let result = push_to_remote(dir.path(), "main", None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("GITHUB_TOKEN"));
        // restore
        if let Some(val) = original {
            std::env::set_var("GITHUB_TOKEN", val);
        }
    }
}
