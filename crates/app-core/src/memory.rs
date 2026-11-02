//! Memory the panes' processes use: how it reads, and when it is worth a warning.

use std::collections::{HashMap, HashSet};
use std::hash::Hash;

/// `bytes` the way a pane shows it: `340M`, `1.4G`, `12G`.
pub fn short(bytes: u64) -> String {
    const MIB: f64 = 1024.0 * 1024.0;
    let mib = bytes as f64 / MIB;
    if mib < 1000.0 {
        format!("{}M", mib.round() as u64)
    } else if mib < 10.0 * 1024.0 {
        format!("{:.1}G", mib / 1024.0)
    } else {
        format!("{}G", (mib / 1024.0).round() as u64)
    }
}

/// A limit a pane or all panes together went over.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MemoryWarning<K> {
    /// One pane uses this many bytes.
    Pane(K, u64),
    /// All panes together use this many bytes.
    Total(u64),
}

/// Which limits are over, so each crossing warns once. Something over stays so until it drops below 90% of its limit.
#[derive(Debug)]
pub struct MemoryWatch<K> {
    over: HashSet<K>,
    total_over: bool,
}

impl<K> Default for MemoryWatch<K> {
    fn default() -> Self {
        MemoryWatch { over: HashSet::new(), total_over: false }
    }
}

impl<K: Eq + Hash + Clone> MemoryWatch<K> {
    /// The limits newly crossed by `panes` (bytes each); a limit of 0 is off.
    pub fn check(&mut self, panes: &HashMap<K, u64>, pane_limit: u64, total_limit: u64) -> Vec<MemoryWarning<K>> {
        let still = |bytes: u64, limit: u64| limit > 0 && bytes >= limit / 10 * 9;
        self.over.retain(|key| panes.get(key).is_some_and(|bytes| still(*bytes, pane_limit)));
        let mut warnings: Vec<_> = panes
            .iter()
            .filter(|(key, bytes)| pane_limit > 0 && **bytes >= pane_limit && self.over.insert((*key).clone()))
            .map(|(key, bytes)| MemoryWarning::Pane(key.clone(), *bytes))
            .collect();
        let total = panes.values().sum();
        if total_limit > 0 && total >= total_limit && !self.total_over {
            warnings.push(MemoryWarning::Total(total));
        }
        self.total_over = still(total, total_limit) && (self.total_over || total >= total_limit);
        warnings
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_sizes() {
        const M: u64 = 1024 * 1024;
        assert_eq!(short(0), "0M");
        assert_eq!(short(340 * M), "340M");
        assert_eq!(short(999 * M), "999M");
        assert_eq!(short(1400 * M), "1.4G");
        assert_eq!(short(12 * 1024 * M + 300 * M), "12G");
    }

    #[test]
    fn each_crossing_warns_once() {
        const G: u64 = 1 << 30;
        let mut watch = MemoryWatch::default();
        let mut panes = HashMap::from([(1, 3 * G), (2, 2 * G)]);
        assert_eq!(watch.check(&panes, 4 * G, 6 * G), []);
        panes.insert(1, 5 * G);
        assert_eq!(watch.check(&panes, 4 * G, 6 * G), [MemoryWarning::Pane(1, 5 * G), MemoryWarning::Total(7 * G)]);
        assert_eq!(watch.check(&panes, 4 * G, 6 * G), []);
        // Just under the limit is still over: no second warning.
        panes.insert(1, 4 * G - 1);
        assert_eq!(watch.check(&panes, 4 * G, 6 * G), []);
        panes.insert(1, 5 * G);
        assert_eq!(watch.check(&panes, 4 * G, 6 * G), []);
        // Well under, then over again: warns again.
        panes.insert(1, G);
        assert_eq!(watch.check(&panes, 4 * G, 6 * G), []);
        panes.insert(1, 5 * G);
        assert_eq!(watch.check(&panes, 4 * G, 6 * G), [MemoryWarning::Pane(1, 5 * G), MemoryWarning::Total(7 * G)]);
        // A limit of 0 is off.
        assert_eq!(MemoryWatch::default().check(&panes, 0, 0), []);
    }
}
