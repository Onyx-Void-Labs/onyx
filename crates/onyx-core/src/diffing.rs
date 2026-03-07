// ─── Onyx Core — Diffing Engine (Version Control) ───────────────────

use similar::{ChangeTag, TextDiff};

/// Compute a line-level unified diff between `old` and `new` text.
/// Returns a human-readable diff string with +/- prefixes.
pub fn compute_diff(old: &str, new: &str) -> String {
    let diff = TextDiff::from_lines(old, new);
    let mut output = String::new();

    for change in diff.iter_all_changes() {
        let prefix = match change.tag() {
            ChangeTag::Delete => "-",
            ChangeTag::Insert => "+",
            ChangeTag::Equal => " ",
        };
        output.push_str(prefix);
        output.push_str(change.value());
        // Ensure trailing newline for each line
        if !change.value().ends_with('\n') {
            output.push('\n');
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_text() {
        let result = compute_diff("hello\nworld\n", "hello\nworld\n");
        assert_eq!(result, " hello\n world\n");
    }

    #[test]
    fn added_line() {
        let result = compute_diff("line1\n", "line1\nline2\n");
        assert!(result.contains("+line2"));
    }

    #[test]
    fn removed_line() {
        let result = compute_diff("line1\nline2\n", "line1\n");
        assert!(result.contains("-line2"));
    }

    #[test]
    fn changed_line() {
        let result = compute_diff("old text\n", "new text\n");
        assert!(result.contains("-old text"));
        assert!(result.contains("+new text"));
    }
}
