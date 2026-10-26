//! Spell checking of the prompt line with the system spell checker (macOS; nothing elsewhere), and rephrasing with
//! Claude.

use std::io::Write;
use std::ops::Range;
use std::path::PathBuf;
use std::process::{Command, Stdio};

/// The misspelled words of `text`, as char ranges.
pub fn misspelled(text: &str) -> Vec<Range<usize>> {
    imp::misspelled(text).into_iter().map(|range| char_range(text, range)).collect()
}

/// Suggestions for the word at char range `word` of `text`, best first.
pub fn guesses(text: &str, word: Range<usize>) -> Vec<String> {
    imp::guesses(text, utf16_range(text, word))
}

/// Stop flagging `word`, here and in every other app.
pub fn learn(word: &str) {
    imp::learn(word);
}

/// Flag `word` again where it is misspelled.
pub fn unlearn(word: &str) {
    imp::unlearn(word);
}

/// The words taught with Learn Spelling (in any app), sorted.
pub fn learned() -> Vec<String> {
    let path = std::env::home_dir().map(|home| home.join("Library/Spelling/LocalDictionary"));
    let text = path.and_then(|path| std::fs::read_to_string(path).ok()).unwrap_or_default();
    learned_words(&text)
}

fn learned_words(text: &str) -> Vec<String> {
    let mut words: Vec<String> =
        text.lines().map(str::trim).filter(|word| !word.is_empty()).map(str::to_string).collect();
    words.sort_by_key(|word| word.to_lowercase());
    words.dedup();
    words
}

/// `text` rewritten by Claude with `~/.local/bin/claude-rephrase` (stdin to stdout); `None` when that fails or is
/// missing. Blocks for a few seconds.
pub fn rephrase(text: &str) -> Option<String> {
    let script = PathBuf::from(std::env::var_os("HOME")?).join(".local/bin/claude-rephrase");
    let mut child =
        Command::new(script).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().ok()?;
    child.stdin.take()?.write_all(text.as_bytes()).ok()?;
    let output = child.wait_with_output().ok()?;
    let text = String::from_utf8(output.stdout).ok()?.trim_end().to_string();
    (output.status.success() && !text.is_empty()).then_some(text)
}

/// A UTF-16 range of `text` (what the spell checker speaks) as a char range.
fn char_range(text: &str, range: Range<usize>) -> Range<usize> {
    let mut units = 0;
    let mut start = None;
    for (index, ch) in text.chars().enumerate() {
        if start.is_none() && units >= range.start {
            start = Some(index);
        }
        if units >= range.end {
            return start.unwrap_or(index)..index;
        }
        units += ch.len_utf16();
    }
    let count = text.chars().count();
    start.unwrap_or(count)..count
}

fn utf16_range(text: &str, range: Range<usize>) -> Range<usize> {
    let units = |chars: usize| text.chars().take(chars).map(char::len_utf16).sum::<usize>();
    units(range.start)..units(range.end)
}

#[cfg(target_os = "macos")]
mod imp {
    use std::ops::Range;

    use objc2_app_kit::NSSpellChecker;
    use objc2_foundation::{NSRange, NSString};

    /// Checked in the system language, and in Hebrew too when the text has Hebrew letters: each skips the other's
    /// script, and guessing the language of one short line fails.
    pub fn misspelled(text: &str) -> Vec<Range<usize>> {
        let checker = NSSpellChecker::sharedSpellChecker();
        let string = NSString::from_str(text);
        let system = checker.language();
        let mut languages = vec![system.clone()];
        if text.chars().any(|ch| ('\u{05d0}'..='\u{05ea}').contains(&ch)) {
            languages.push(NSString::from_str("he"));
        }
        let identifies = checker.automaticallyIdentifiesLanguages();
        checker.setAutomaticallyIdentifiesLanguages(false);
        let mut found = Vec::new();
        for language in &languages {
            checker.setLanguage(language);
            let mut from = 0;
            while from < string.length() {
                let range = checker.checkSpellingOfString_startingAt(&string, from as isize);
                // NSNotFound, or a wrap back to an earlier word.
                if range.length == 0 || range.location < from {
                    break;
                }
                found.push(range.location..range.location + range.length);
                from = range.location + range.length;
            }
        }
        checker.setLanguage(&system);
        checker.setAutomaticallyIdentifiesLanguages(identifies);
        found.sort_by_key(|range| range.start);
        found.dedup();
        found
    }

    pub fn guesses(text: &str, word: Range<usize>) -> Vec<String> {
        let checker = NSSpellChecker::sharedSpellChecker();
        let string = NSString::from_str(text);
        let range = NSRange::new(word.start, word.len());
        checker
            .guessesForWordRange_inString_language_inSpellDocumentWithTag(range, &string, None, 0)
            .map(|guesses| guesses.iter().map(|guess| guess.to_string()).collect())
            .unwrap_or_default()
    }

    pub fn learn(word: &str) {
        NSSpellChecker::sharedSpellChecker().learnWord(&NSString::from_str(word));
    }

    pub fn unlearn(word: &str) {
        NSSpellChecker::sharedSpellChecker().unlearnWord(&NSString::from_str(word));
    }
}

#[cfg(not(target_os = "macos"))]
mod imp {
    use std::ops::Range;

    pub fn misspelled(_: &str) -> Vec<Range<usize>> {
        Vec::new()
    }

    pub fn guesses(_: &str, _: Range<usize>) -> Vec<String> {
        Vec::new()
    }

    pub fn learn(_: &str) {}

    pub fn unlearn(_: &str) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf16_ranges_map_to_chars() {
        // "😀" is two UTF-16 units, one char.
        let text = "😀 teh cat";
        assert_eq!(char_range(text, 3..6), 2..5);
        assert_eq!(utf16_range(text, 2..5), 3..6);
        assert_eq!(char_range(text, 7..10), 6..9, "a word at the end");
        assert_eq!(char_range("שלום wrld", 5..9), 5..9);
    }

    #[test]
    fn learned_words_are_one_per_line_sorted() {
        assert_eq!(learned_words("zed\n\nGantry\nzed\n"), ["Gantry", "zed"]);
        assert!(learned_words("").is_empty());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn the_system_checker_flags_and_suggests() {
        let text = "please fix teh bug";
        assert_eq!(misspelled(text), vec![11..14]);
        assert!(guesses(text, 11..14).iter().any(|guess| guess == "the"));
        // Hebrew and English on one line: each language's words are checked in that language.
        let mixed = "תקן את הבאג teh │ > שלוםם";
        let flagged: Vec<String> = misspelled(mixed)
            .into_iter()
            .map(|range| mixed.chars().skip(range.start).take(range.len()).collect())
            .collect();
        assert_eq!(flagged, ["teh", "שלוםם"]);
    }
}
