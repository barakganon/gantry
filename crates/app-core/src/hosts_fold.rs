//! The SSH hosts card's collapse rule: a manual toggle always wins, and the automatic rule only ever collapses.

/// Whether the sidebar's SSH hosts card is folded, and when the sessions list may fold it by itself.
#[derive(Clone, Copy, Debug)]
pub struct HostsFold {
    collapsed: bool,
    /// The sessions list has fit once with the card expanded, so a list that starts to scroll may fold it.
    armed: bool,
    /// The card was just toggled: the next measurement predates the new layout and is ignored.
    settling: bool,
}

impl HostsFold {
    /// A fold that starts as saved, armed and not settling.
    pub fn new(saved_collapsed: bool) -> HostsFold {
        HostsFold { collapsed: saved_collapsed, armed: true, settling: false }
    }

    /// Whether the card is folded to one line.
    pub fn collapsed(&self) -> bool {
        self.collapsed
    }

    /// Flips the card by hand; returns the new `collapsed`, which is what gets saved. Expanding disarms the automatic
    /// rule until the list has fit once.
    pub fn toggle(&mut self) -> bool {
        self.collapsed = !self.collapsed;
        if !self.collapsed {
            self.armed = false;
            self.settling = true;
        }
        self.collapsed
    }

    /// Takes whether the sessions list scrolled in the last paint; returns `true` when the card must be redrawn.
    pub fn observe(&mut self, overflowing: bool) -> bool {
        if self.collapsed {
            return false;
        }
        if self.settling {
            self.settling = false;
            return true;
        }
        if overflowing && self.armed {
            self.collapsed = true;
            return true;
        }
        if !overflowing && !self.armed {
            self.armed = true;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_as_saved() {
        assert!(HostsFold::new(true).collapsed());
        assert!(!HostsFold::new(false).collapsed());
    }

    #[test]
    fn a_scrolling_list_collapses_an_expanded_card_once() {
        let mut fold = HostsFold::new(false);
        assert!(fold.observe(true));
        assert!(fold.collapsed());
        assert!(!fold.observe(true));
    }

    #[test]
    fn it_never_expands_by_itself() {
        let mut fold = HostsFold::new(true);
        for _ in 0..5 {
            assert!(!fold.observe(false));
        }
        assert!(fold.collapsed());
    }

    #[test]
    fn a_manual_expand_survives_a_stale_measurement() {
        let mut fold = HostsFold::new(true);
        assert!(!fold.toggle());
        assert!(fold.observe(false));
        assert!(!fold.collapsed());
        assert!(!fold.observe(true));
        assert!(!fold.collapsed());
        assert!(!fold.observe(true));
        assert!(!fold.collapsed());
    }

    #[test]
    fn it_re_arms_after_the_list_fits_once_while_expanded() {
        let mut fold = HostsFold::new(true);
        fold.toggle();
        fold.observe(false);
        fold.observe(true);
        fold.observe(false);
        assert!(fold.observe(true));
        assert!(fold.collapsed());
    }

    #[test]
    fn a_manual_collapse_and_expand_with_a_fitting_list_re_arms() {
        let mut fold = HostsFold::new(false);
        fold.toggle();
        fold.toggle();
        fold.observe(false);
        fold.observe(false);
        assert!(fold.observe(true));
        assert!(fold.collapsed());
    }

    #[test]
    fn toggle_returns_what_to_save() {
        let mut fold = HostsFold::new(false);
        assert!(fold.toggle());
        assert!(!fold.toggle());
    }
}
