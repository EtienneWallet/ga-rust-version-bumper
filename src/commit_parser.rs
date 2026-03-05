#[derive(Debug, PartialEq, Clone)]
pub enum CommitImpact {
    Breaking,
    Refactor,
    Feature,
    Dev,
}

/// Parse a conventional commit message and return the highest-impact level.
pub fn parse_commit(message: &str) -> CommitImpact {
    if message.is_empty() {
        eprintln!("Warning: empty commit message -- defaulting to dev bump");
        return CommitImpact::Dev;
    }

    // Match: type[(scope)][!]: description
    // type is one or more lowercase (or uppercase) letters
    let first_line = message.lines().next().unwrap_or("");
    let (commit_type, has_bang) = match parse_type_and_bang(first_line) {
        Some(v) => v,
        None => {
            eprintln!("Warning: Non-conventional commit -- defaulting to dev bump");
            return CommitImpact::Dev;
        }
    };

    let has_breaking_footer = message.contains("BREAKING CHANGE:");

    // Breaking: ! on any type (except refactor -- covered below), or BREAKING CHANGE footer
    // Refactor: always treated as refactor (major) regardless of !
    // Priority: refactor overrides the bang check for breaking vs refactor distinction,
    // but both produce the same major-bump behavior. We use Refactor for refactor types.
    let commit_type_lower = commit_type.to_lowercase();

    if commit_type_lower == "refactor" {
        return CommitImpact::Refactor;
    }

    if has_bang || has_breaking_footer {
        return CommitImpact::Breaking;
    }

    if commit_type_lower == "feat" {
        return CommitImpact::Feature;
    }

    CommitImpact::Dev
}

/// Returns (type_str, has_bang) if the line matches conventional commit format, else None.
fn parse_type_and_bang(line: &str) -> Option<(&str, bool)> {
    // Find first occurrence of ':', '!', or '('
    let colon_pos = line.find(':')?;

    // Everything before the colon is "type[(scope)][!]"
    let before_colon = &line[..colon_pos];

    // Must not be empty and must not contain spaces (basic sanity)
    if before_colon.is_empty() || before_colon.contains(' ') {
        return None;
    }

    let has_bang = before_colon.ends_with('!');
    let without_bang = if has_bang {
        &before_colon[..before_colon.len() - 1]
    } else {
        before_colon
    };

    // Extract type: up to '(' (scope) or end
    let commit_type = if let Some(paren_pos) = without_bang.find('(') {
        let closing = without_bang.find(')')?;
        if closing < paren_pos {
            return None;
        }
        &without_bang[..paren_pos]
    } else {
        without_bang
    };

    // Type must be non-empty and only letters
    if commit_type.is_empty() || !commit_type.chars().all(|c| c.is_ascii_alphabetic()) {
        return None;
    }

    Some((commit_type, has_bang))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feat() {
        assert_eq!(parse_commit("feat: add OAuth"), CommitImpact::Feature);
    }

    #[test]
    fn test_feat_bang() {
        assert_eq!(parse_commit("feat!: add OAuth"), CommitImpact::Breaking);
    }

    #[test]
    fn test_fix() {
        assert_eq!(parse_commit("fix: handle null"), CommitImpact::Dev);
    }

    #[test]
    fn test_refactor() {
        assert_eq!(parse_commit("refactor: rework db"), CommitImpact::Refactor);
    }

    #[test]
    fn test_refactor_bang() {
        assert_eq!(parse_commit("refactor!: rework db"), CommitImpact::Refactor);
    }

    #[test]
    fn test_chore() {
        assert_eq!(parse_commit("chore: bump version to 1.2.3"), CommitImpact::Dev);
    }

    #[test]
    fn test_feat_with_scope() {
        assert_eq!(parse_commit("feat(auth): add OAuth"), CommitImpact::Feature);
    }

    #[test]
    fn test_case_insensitive() {
        assert_eq!(parse_commit("FEAT: add OAuth"), CommitImpact::Feature);
    }

    #[test]
    fn test_breaking_change_footer() {
        let msg = "fix: update API\n\nBREAKING CHANGE: old endpoint removed";
        assert_eq!(parse_commit(msg), CommitImpact::Breaking);
    }

    #[test]
    fn test_empty_message() {
        assert_eq!(parse_commit(""), CommitImpact::Dev);
    }

    #[test]
    fn test_non_conventional() {
        assert_eq!(parse_commit("random commit message"), CommitImpact::Dev);
    }

    #[test]
    fn test_feat_bang_and_breaking_footer() {
        let msg = "feat!: big change\n\nBREAKING CHANGE: removed everything";
        assert_eq!(parse_commit(msg), CommitImpact::Breaking);
    }
}
