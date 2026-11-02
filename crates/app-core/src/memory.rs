//! Memory the panes' processes use: how it reads, and when it is worth a warning.

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
}
