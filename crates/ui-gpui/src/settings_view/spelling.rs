//! The Spelling section: the words taught with Learn Spelling, which every Mac app shares.

use app_core::settings::Settings;
use gpui::{Context, Div, Window, div};

use super::*;
use crate::spelling;
use crate::theme::Theme;

/// The field that adds a learned word.
pub(super) const ADD_WORD: &str = "spell-add";

impl SettingsView {
    pub(super) fn spelling_page(
        &mut self,
        theme: &Theme,
        _settings: &Settings,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Div {
        let words = self.learned_words.get_or_insert_with(spelling::learned).clone();
        let mut rows: Vec<Div> = Vec::new();
        for (index, word) in words.iter().enumerate() {
            let unlearned = word.clone();
            let remove = self.link(
                "spell-remove",
                index,
                "Remove",
                true,
                handler(move |this, _, cx| {
                    spelling::unlearn(&unlearned);
                    if let Some(words) = &mut this.learned_words {
                        words.retain(|word| *word != unlearned);
                    }
                    cx.notify();
                }),
                window,
                cx,
            );
            rows.push(row_shell(theme, div().child(word.clone()), div(), actions_slot(theme).child(remove)));
        }
        if words.is_empty() {
            rows.push(row_shell(theme, muted(theme, "No learned words yet."), div(), empty_slot(theme)));
        }
        let add = self.text_field((ADD_WORD, 0), "", "Add a word", false, theme, window, cx).w(theme.ui(200.));
        rows.push(row_shell(theme, muted(theme, "Type a word, then Enter."), add, empty_slot(theme)));
        group(
            theme,
            "Learned words",
            Some(
                "Words the spell checker accepts. Right-click a misspelled word on a Claude prompt, then Learn Spelling, \
                 to add one. Shared with every Mac app.",
            ),
            rows,
        )
    }

    /// Teach the spell checker the word typed in the Add field.
    pub(super) fn add_learned_word(&mut self, text: &str) {
        let word = text.trim();
        if word.is_empty() {
            return;
        }
        spelling::learn(word);
        let words = self.learned_words.get_or_insert_with(Vec::new);
        if !words.iter().any(|known| known == word) {
            words.push(word.to_string());
            words.sort_by_key(|word| word.to_lowercase());
        }
    }
}
