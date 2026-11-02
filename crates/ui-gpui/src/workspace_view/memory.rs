use super::WorkspaceView;
use std::collections::HashMap;
use std::time::Duration;

use app_core::split_tree::NodeId;
use gpui::{Context, Task, Window};
use protocol::host::{HostId, PaneKey};
use protocol::message::PaneId;

/// How often the daemons are asked for each pane's memory.
const MEMORY_INTERVAL: Duration = Duration::from_secs(5);

impl WorkspaceView {
    /// Every `MEMORY_INTERVAL`: ask each connected daemon that reports memory for its panes'.
    pub(super) fn start_memory_timer(window: &mut Window, cx: &mut Context<Self>) -> Task<()> {
        cx.spawn_in(window, async move |this, cx| {
            loop {
                if this.update(cx, |view, _| view.ask_memory()).is_err() {
                    return;
                }
                cx.background_executor().timer(MEMORY_INTERVAL).await;
            }
        })
    }

    fn ask_memory(&self) {
        for conn in self.hosts.values().filter(|conn| conn.connected && conn.client.reports_memory()) {
            conn.client.pane_memory();
        }
    }

    /// `host`'s answer: replaces what it said before.
    pub(super) fn on_pane_memory(&mut self, host: &HostId, panes: &[(PaneId, u64)], cx: &mut Context<Self>) {
        self.memory.retain(|key, _| key.host != *host);
        self.memory.extend(panes.iter().map(|(pane, bytes)| (PaneKey::new(host.clone(), *pane), *bytes)));
        let by_leaf: HashMap<NodeId, u64> =
            self.memory.iter().filter_map(|(key, bytes)| Some((self.workspace.leaf_for_pane(key)?, *bytes))).collect();
        self.sidebar.update(cx, |sidebar, cx| sidebar.set_memory(by_leaf, cx));
        cx.notify();
    }
}
