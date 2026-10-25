//! Which pane a running process descends from. A Claude started through a wrapper (a launcher that starts a child
//! that starts the real Claude) writes its pid file from the innermost process, not from the pane's own.

use std::collections::HashMap;
use std::process::Command;

use protocol::message::PaneId;

/// Most parents followed from one process.
const MAX_DEPTH: usize = 16;

/// Parse the output of `ps -A -o pid=,ppid=` (two integers a line) into a pid → parent pid table. Lines that are not
/// two integers are skipped.
pub fn parse_ps(output: &str) -> HashMap<u32, u32> {
    output
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let (pid, ppid) = (fields.next()?.parse().ok()?, fields.next()?.parse().ok()?);
            fields.next().is_none().then_some((pid, ppid))
        })
        .collect()
}

/// Every process's parent, as `ps` reports it (the same on Linux and macOS). `None` when `ps` cannot be run.
pub fn read_parents() -> Option<HashMap<u32, u32>> {
    let output = Command::new("ps")
        .args(["-A", "-o", "pid=,ppid="])
        .output()
        .map_err(|err| tracing::warn!("could not run ps: {err}"))
        .ok()?;
    if !output.status.success() {
        tracing::warn!("ps failed: {}", output.status);
        return None;
    }
    Some(parse_ps(&String::from_utf8_lossy(&output.stdout)))
}

/// The pane whose process `pid` descends from: its parents are followed until one is a pane's process. Stops at pid 0
/// or 1, a missing entry, a cycle or `MAX_DEPTH` parents. `pid` itself is not looked up in `panes`.
pub fn owner(pid: u32, parents: &HashMap<u32, u32>, panes: &HashMap<u32, PaneId>) -> Option<PaneId> {
    let mut current = pid;
    for _ in 0..MAX_DEPTH {
        current = *parents.get(&current)?;
        if let Some(pane) = panes.get(&current) {
            return Some(*pane);
        }
        if current <= 1 || current == pid {
            return None;
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The chains seen on macOS: two panes' wrappers, and a Claude started in Warp.
    fn table() -> HashMap<u32, u32> {
        parse_ps(
            "
  77028 77018
  77018 77017
  77017 76546
  76546 76544
  76638 76624
  76624 76623
  76623 76546
  31833 31819
  31819 31818
  31818 31626
  31626  1946
   1946  1836
   1836     1
not a process line
  12 x
",
        )
    }

    #[test]
    fn parses_ps_output() {
        let parents = table();
        assert_eq!(parents.len(), 13);
        assert_eq!(parents[&77028], 77018);
        assert_eq!(parents[&1836], 1);
    }

    #[test]
    fn a_wrapped_claude_descends_from_its_pane() {
        let parents = table();
        let panes = HashMap::from([(77017, 1), (76623, 2)]);
        assert_eq!(owner(77028, &parents, &panes), Some(1));
        assert_eq!(owner(76638, &parents, &panes), Some(2));
        assert_eq!(owner(31833, &parents, &panes), None);
    }

    #[test]
    fn cycles_and_deep_chains_have_no_owner() {
        let cycle = HashMap::from([(5, 6), (6, 7), (7, 6)]);
        assert_eq!(owner(5, &cycle, &HashMap::from([(1000, 1)])), None);
        let long: HashMap<u32, u32> = (10..60).map(|pid| (pid, pid + 1)).collect();
        let panes = HashMap::from([(59, 1)]);
        assert_eq!(owner(10, &long, &panes), None);
        assert_eq!(owner(50, &long, &panes), Some(1));
    }

    // Proves the macOS path: the real `ps` and `sh` of this machine, not a canned table.
    #[cfg(unix)]
    #[test]
    fn a_real_grandchild_descends_from_its_ancestor() {
        let mut outer = Command::new("sh").args(["-c", "sh -c 'sleep 30; :'; :"]).spawn().expect("spawn sh");
        let outer_pid = outer.id();
        let panes = HashMap::from([(outer_pid, 7)]);
        let found = (0..50).find_map(|_| {
            std::thread::sleep(std::time::Duration::from_millis(100));
            let parents = read_parents()?;
            parents.iter().find_map(|(pid, ppid)| {
                (*ppid != outer_pid && owner(*pid, &parents, &panes) == Some(7)).then_some(*pid)
            })
        });
        if let Some(pid) = found {
            let _ = Command::new("kill").arg(pid.to_string()).status();
        }
        let _ = Command::new("pkill").args(["-P", &outer_pid.to_string()]).status();
        let _ = outer.kill();
        let _ = outer.wait();
        assert!(found.is_some(), "no descendant of the outer shell found through ps");
    }
}
