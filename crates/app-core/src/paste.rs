//! Prepare clipboard text for sending to a terminal program.

/// Encode pasted text: line endings become `CR`, `ESC` is removed (so pasted text can never
/// inject control sequences or end bracketed paste early), and bracketed-paste markers are
/// added when the program enabled that mode.
pub fn encode_paste(text: &str, bracketed: bool) -> Vec<u8> {
    let cleaned: String = text.replace("\r\n", "\r").replace('\n', "\r").chars().filter(|&c| c != '\x1b').collect();
    if bracketed { [b"\x1b[200~".as_slice(), cleaned.as_bytes(), b"\x1b[201~"].concat() } else { cleaned.into_bytes() }
}

/// Whether the newest `[Pasted text #N +L lines]` in Claude's input box stands for `text`: its line count matches and
/// the text is short enough (100,000 UTF-16 units) for Claude to expand it in place when it is pasted again.
pub fn newest_paste_token_is(box_text: &str, text: &str) -> bool {
    let newest = box_text
        .match_indices("[Pasted text #")
        .filter_map(|(at, marker)| {
            let rest = &box_text[at + marker.len()..];
            let (id, rest) = rest.split_at(rest.find(|c: char| !c.is_ascii_digit())?);
            let lines = match rest.strip_prefix(" +") {
                Some(rest) => rest.split_once(" lines]")?.0.parse().ok()?,
                None if rest.starts_with(']') => 0,
                None => return None,
            };
            Some((id.parse::<u64>().ok()?, lines))
        })
        .max();
    newest.is_some_and(|(_, lines): (u64, usize)| lines == text.matches('\n').count())
        && text.encode_utf16().count() <= 100_000
}

/// Paths as shell words, space-separated with a trailing space, as Terminal.app pastes dropped files: anything outside a
/// safe set gets a backslash, so both a shell and Claude's prompt read each path back whole.
pub fn quote_paths<'a>(paths: impl IntoIterator<Item = &'a str>) -> String {
    paths
        .into_iter()
        .map(|path| {
            path.chars().fold(String::new(), |mut word, c| {
                if !(c.is_alphanumeric() || "/._-+,:@%=^".contains(c)) {
                    word.push('\\');
                }
                word.push(c);
                word
            })
        })
        .map(|word| word + " ")
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newlines_become_carriage_returns() {
        assert_eq!(encode_paste("a\r\nb\nc", false), b"a\rb\rc".to_vec());
    }

    #[test]
    fn bracketed_paste_wraps_text() {
        assert_eq!(encode_paste("hi", true), b"\x1b[200~hi\x1b[201~".to_vec());
    }

    #[test]
    fn escape_cannot_break_out_of_bracketed_paste() {
        let pasted = encode_paste("x\x1b[201~rm -rf ~\n", true);
        assert_eq!(pasted, b"\x1b[200~x[201~rm -rf ~\r\x1b[201~".to_vec());
    }

    #[test]
    fn paths_are_escaped_and_space_separated() {
        assert_eq!(
            quote_paths(["/tmp/a.png", "/Users/me/My Shot (2).png"]),
            "/tmp/a.png /Users/me/My\\ Shot\\ \\(2\\).png "
        );
        assert_eq!(quote_paths(["/x/it's;$y"]), "/x/it\\'s\\;\\$y ");
        assert_eq!(quote_paths(["/דוח/צילום.pdf"]), "/דוח/צילום.pdf ");
        assert_eq!(quote_paths([]), "");
    }

    #[test]
    fn only_the_newest_token_with_the_same_line_count_stands_for_the_text() {
        let three = "a\nb\nc\nd";
        assert!(newest_paste_token_is("> x [Pasted text #2 +3 lines]\n", three));
        assert!(!newest_paste_token_is("> [Pasted text #2 +3 lines] [Pasted text #3 +9 lines]", three), "a newer one");
        assert!(!newest_paste_token_is("> no token here", three));
        assert!(newest_paste_token_is("> [Pasted text #1]", &"x".repeat(900)));
        assert!(!newest_paste_token_is("> [Pasted text #1]", &"x".repeat(100_001)), "too long to expand");
    }
}
