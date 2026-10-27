//! Find a URL at a position in a line of terminal text.

use std::ops::Range;
use std::sync::LazyLock;

use regex::Regex;

static URL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"https?://[^\s<>"'`]+"#).expect("valid URL regex"));

/// A URL found in a line of terminal text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Link {
    /// Columns (char indices) the URL covers.
    pub cols: Range<usize>,
    /// The URL, without trailing punctuation.
    pub url: String,
}

/// Return the link covering char index `col` of `line`.
pub fn link_at(line: &str, col: usize) -> Option<Link> {
    URL.find_iter(line).find_map(|found| {
        let url = found.as_str().trim_end_matches(['.', ',', ')', ']', ';', ':', '!', '?']);
        let start = line[..found.start()].chars().count();
        let cols = start..start + url.chars().count();
        cols.contains(&col).then(|| Link { cols, url: url.to_string() })
    })
}

static PATH: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"[^\s"'`<>()\[\]{}|│,;]+"#).expect("valid path regex"));
static LINE_SUFFIX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(.+?)(?::(\d+))?(?::(\d+))?$").expect("valid line suffix regex"));

/// What looks like a file path in a line of terminal text; whether it exists is the caller's to check.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PathLink {
    /// Columns (char indices) the path and its `:line:col` cover.
    pub cols: Range<usize>,
    /// The path as written, e.g. `~/x` or `src/a.rs`.
    pub path: String,
    /// The line from a `:line` suffix.
    pub line: Option<u32>,
    /// The column from a `:line:col` suffix.
    pub column: Option<u32>,
}

/// Return the path-like word covering char index `col` of `line`: one with a `/`, a leading `~`, or an extension,
/// e.g. `src/main.rs:42:7`, `~/notes`, `Cargo.toml`; a leading `@` (Claude's file mention) is dropped.
pub fn path_at(line: &str, col: usize) -> Option<PathLink> {
    PATH.find_iter(line).find_map(|found| {
        let word = found.as_str().trim_end_matches(['.', ',', ':', '!', '?']);
        let skip = usize::from(word.starts_with('@'));
        let word = &word[skip..];
        let start = line[..found.start()].chars().count() + skip;
        let cols = start..start + word.chars().count();
        if !cols.contains(&col) || word.contains("://") {
            return None;
        }
        let parts = LINE_SUFFIX.captures(word)?;
        let path = parts.get(1)?.as_str();
        let number = |at: usize| parts.get(at).and_then(|n| n.as_str().parse().ok());
        let named = path.contains('/')
            || path.starts_with('~')
            || path.rsplit_once('.').is_some_and(|(stem, ext)| {
                !stem.is_empty() && !ext.is_empty() && ext.chars().all(|ch| ch.is_ascii_alphanumeric())
            });
        named.then(|| PathLink { cols, path: path.to_string(), line: number(2), column: number(3) })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_paths_with_lines_and_skips_plain_words() {
        let link = path_at("⎿  Read crates/app/src/main.rs:42:7 (12 lines)", 12).unwrap();
        assert_eq!(
            (link.cols, link.path.as_str(), link.line, link.column),
            (8..35, "crates/app/src/main.rs", Some(42), Some(7))
        );
        assert_eq!(path_at("see ~/notes.", 6).map(|l| l.path).as_deref(), Some("~/notes"));
        assert_eq!(path_at("edit @Cargo.toml now", 7).map(|l| (l.cols, l.path)), Some((6..16, "Cargo.toml".into())));
        assert_eq!(path_at("plain words here", 2), None);
        assert_eq!(path_at("go to https://x.io/p", 8), None);
    }

    fn url_at(line: &str, col: usize) -> Option<String> {
        link_at(line, col).map(|link| link.url)
    }

    #[test]
    fn finds_url_under_column() {
        let line = "see https://example.com/a?b=1 now";
        assert_eq!(url_at(line, 4).as_deref(), Some("https://example.com/a?b=1"));
        assert_eq!(url_at(line, 28).as_deref(), Some("https://example.com/a?b=1"));
        assert_eq!(url_at(line, 2), None);
        assert_eq!(url_at(line, 30), None);
    }

    #[test]
    fn trailing_punctuation_is_not_part_of_url() {
        assert_eq!(url_at("(https://x.io/p).", 5).as_deref(), Some("https://x.io/p"));
    }

    #[test]
    fn link_reports_its_column_range() {
        let link = link_at("see https://x.io/p now", 6).unwrap();
        assert_eq!((link.cols, link.url.as_str()), (4..18, "https://x.io/p"));
    }

    #[test]
    fn columns_count_chars_not_bytes() {
        let line = "שלום http://a.b";
        assert_eq!(url_at(line, 5).as_deref(), Some("http://a.b"));
    }
}
