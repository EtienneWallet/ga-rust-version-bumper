use std::fs;
use std::path::Path;
use toml_edit::DocumentMut;

#[derive(Debug, Clone, PartialEq)]
pub enum VersionLocation {
    Workspace,
    Package,
}

pub fn read_version(path: &Path) -> Result<(String, VersionLocation), String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read Cargo.toml at {}: {}", path.display(), e))?;

    let doc: DocumentMut = content
        .parse()
        .map_err(|e| format!("Failed to parse Cargo.toml: {}", e))?;

    // Check [workspace.package] first
    if let Some(workspace) = doc.get("workspace") {
        if let Some(pkg) = workspace.get("package") {
            if let Some(ver) = pkg.get("version") {
                if let Some(s) = ver.as_str() {
                    return Ok((s.to_string(), VersionLocation::Workspace));
                }
            }
        }
    }

    // Fallback to [package]
    if let Some(pkg) = doc.get("package") {
        if let Some(ver) = pkg.get("version") {
            if let Some(s) = ver.as_str() {
                return Ok((s.to_string(), VersionLocation::Package));
            }
        }
    }

    Err("Version field not found in Cargo.toml".to_string())
}

pub fn write_version(
    path: &Path,
    location: VersionLocation,
    new_version: &str,
) -> Result<(), String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read Cargo.toml at {}: {}", path.display(), e))?;

    let mut doc: DocumentMut = content
        .parse()
        .map_err(|e| format!("Failed to parse Cargo.toml: {}", e))?;

    match location {
        VersionLocation::Workspace => {
            doc["workspace"]["package"]["version"] = toml_edit::value(new_version);
        }
        VersionLocation::Package => {
            doc["package"]["version"] = toml_edit::value(new_version);
        }
    }

    fs::write(path, doc.to_string()).map_err(|e| format!("Failed to write Cargo.toml: {}", e))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write_toml(dir: &TempDir, content: &str) -> std::path::PathBuf {
        let path = dir.path().join("Cargo.toml");
        fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn test_read_workspace_package() {
        let dir = TempDir::new().unwrap();
        let path = write_toml(
            &dir,
            r#"
[workspace.package]
version = "1.2.3"
"#,
        );
        let (ver, loc) = read_version(&path).unwrap();
        assert_eq!(ver, "1.2.3");
        assert_eq!(loc, VersionLocation::Workspace);
    }

    #[test]
    fn test_read_package_fallback() {
        let dir = TempDir::new().unwrap();
        let path = write_toml(
            &dir,
            r#"
[package]
name = "foo"
version = "0.1.0"
"#,
        );
        let (ver, loc) = read_version(&path).unwrap();
        assert_eq!(ver, "0.1.0");
        assert_eq!(loc, VersionLocation::Package);
    }

    #[test]
    fn test_workspace_takes_priority() {
        let dir = TempDir::new().unwrap();
        let path = write_toml(
            &dir,
            r#"
[workspace.package]
version = "2.0.0"

[package]
version = "1.0.0"
"#,
        );
        let (ver, loc) = read_version(&path).unwrap();
        assert_eq!(ver, "2.0.0");
        assert_eq!(loc, VersionLocation::Workspace);
    }

    #[test]
    fn test_malformed_toml_returns_err() {
        let dir = TempDir::new().unwrap();
        let path = write_toml(&dir, "not valid toml ][");
        assert!(read_version(&path).is_err());
    }

    #[test]
    fn test_missing_version_field_returns_err() {
        let dir = TempDir::new().unwrap();
        let path = write_toml(
            &dir,
            r#"
[package]
name = "foo"
"#,
        );
        let err = read_version(&path).unwrap_err();
        assert!(err.contains("Version field not found"));
    }

    #[test]
    fn test_write_preserves_formatting() {
        let dir = TempDir::new().unwrap();
        let original = r#"# top comment
[package]
name = "foo"
# version comment
version = "1.0.0"
edition = "2021"
"#;
        let path = write_toml(&dir, original);
        write_version(&path, VersionLocation::Package, "1.1.0").unwrap();
        let result = fs::read_to_string(&path).unwrap();
        assert!(result.contains("version = \"1.1.0\""));
        assert!(result.contains("# top comment"));
        assert!(result.contains("# version comment"));
        assert!(result.contains("edition = \"2021\""));
    }
}
