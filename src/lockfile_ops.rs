use std::path::Path;

use crate::git_ops;

/// Update version fields in Cargo.lock for local (non-registry) packages.
/// Returns Ok(true) if updates were made, Ok(false) if Cargo.lock is not tracked.
pub fn update_lockfile_version(
    repo_path: &Path,
    old_version: &str,
    new_version: &str,
) -> Result<bool, String> {
    let lockfile_path = repo_path.join("Cargo.lock");

    if !git_ops::is_file_tracked(repo_path, Path::new("Cargo.lock"))? {
        return Ok(false);
    }

    let content = std::fs::read_to_string(&lockfile_path)
        .map_err(|e| format!("Failed to read Cargo.lock: {}", e))?;

    let mut doc = content
        .parse::<toml_edit::DocumentMut>()
        .map_err(|e| format!("Failed to parse Cargo.lock: {}", e))?;

    let mut updated = false;

    if let Some(packages) = doc
        .get_mut("package")
        .and_then(|p| p.as_array_of_tables_mut())
    {
        for pkg in packages.iter_mut() {
            // Local packages have no `source` field
            if pkg.get("source").is_some() {
                continue;
            }
            if let Some(version) = pkg.get("version").and_then(|v| v.as_str()) {
                if version == old_version {
                    pkg["version"] = toml_edit::value(new_version);
                    updated = true;
                }
            }
        }
    }

    if updated {
        std::fs::write(&lockfile_path, doc.to_string())
            .map_err(|e| format!("Failed to write Cargo.lock: {}", e))?;
        println!(
            "Updated Cargo.lock versions from {} to {}",
            old_version, new_version
        );
    }

    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::{Repository, Signature};
    use std::fs;
    use tempfile::TempDir;

    fn setup_repo_with_lockfile(lockfile_content: &str) -> TempDir {
        let dir = TempDir::new().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        // Create Cargo.toml
        fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"test\"\nversion = \"1.0.0\"\n",
        )
        .unwrap();

        // Create Cargo.lock
        fs::write(dir.path().join("Cargo.lock"), lockfile_content).unwrap();

        // Commit both files so Cargo.lock is tracked
        let sig = Signature::now("Test", "test@test.com").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("Cargo.toml")).unwrap();
        index.add_path(Path::new("Cargo.lock")).unwrap();
        index.write().unwrap();
        let tree_oid = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_oid).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "init", &tree, &[])
            .unwrap();

        dir
    }

    #[test]
    fn test_not_tracked_returns_false() {
        let dir = TempDir::new().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        // Create initial commit without Cargo.lock
        fs::write(dir.path().join("README.md"), "test").unwrap();
        let sig = Signature::now("Test", "test@test.com").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("README.md")).unwrap();
        index.write().unwrap();
        let tree_oid = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_oid).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "init", &tree, &[])
            .unwrap();

        let result = update_lockfile_version(dir.path(), "1.0.0", "1.1.0").unwrap();
        assert!(!result);
    }

    #[test]
    fn test_simple_project_update() {
        let lockfile = r#"version = 4

[[package]]
name = "my-crate"
version = "1.0.0"

[[package]]
name = "serde"
version = "1.0.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
"#;
        let dir = setup_repo_with_lockfile(lockfile);

        let result = update_lockfile_version(dir.path(), "1.0.0", "1.1.0").unwrap();
        assert!(result);

        let content = fs::read_to_string(dir.path().join("Cargo.lock")).unwrap();
        assert!(content.contains("version = \"1.1.0\""));
        // serde should remain 1.0.0 (has source field)
        assert!(content.contains("name = \"serde\""));
    }

    #[test]
    fn test_workspace_multiple_local_packages() {
        let lockfile = r#"version = 4

[[package]]
name = "workspace-root"
version = "2.0.0"

[[package]]
name = "workspace-member"
version = "2.0.0"

[[package]]
name = "external-dep"
version = "2.0.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
"#;
        let dir = setup_repo_with_lockfile(lockfile);

        let result = update_lockfile_version(dir.path(), "2.0.0", "2.1.0").unwrap();
        assert!(result);

        let content = fs::read_to_string(dir.path().join("Cargo.lock")).unwrap();
        let doc: toml_edit::DocumentMut = content.parse().unwrap();
        let packages = doc["package"].as_array_of_tables().unwrap();

        for pkg in packages.iter() {
            let name = pkg["name"].as_str().unwrap();
            let version = pkg["version"].as_str().unwrap();
            match name {
                "workspace-root" | "workspace-member" => assert_eq!(version, "2.1.0"),
                "external-dep" => assert_eq!(version, "2.0.0"),
                _ => panic!("Unexpected package: {}", name),
            }
        }
    }

    #[test]
    fn test_no_match_leaves_file_unchanged() {
        let lockfile = r#"version = 4

[[package]]
name = "my-crate"
version = "1.0.0"
"#;
        let dir = setup_repo_with_lockfile(lockfile);

        let result = update_lockfile_version(dir.path(), "9.9.9", "10.0.0").unwrap();
        assert!(result); // tracked, but no updates made

        let content = fs::read_to_string(dir.path().join("Cargo.lock")).unwrap();
        assert!(content.contains("version = \"1.0.0\""));
        assert!(!content.contains("10.0.0"));
    }
}
