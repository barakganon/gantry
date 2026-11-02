use super::WorkspaceView;
use std::collections::HashMap;
use std::time::Duration;

use app_core::attention::Channels;
use app_core::memory::{MemoryWarning, short};
use app_core::split_tree::NodeId;
use gpui::{Context, Task, Window};
use protocol::host::{HostId, PaneKey};
use protocol::message::PaneId;

use crate::alerts::{self, Notice};
use crate::settings_store::SettingsStore;

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
        self.warn_memory(cx);
        cx.notify();
    }

    /// A notification for each memory limit newly crossed, focusing the pane (the biggest one, for the total).
    fn warn_memory(&mut self, cx: &mut Context<Self>) {
        const GB: u64 = 1 << 30;
        let alerts = cx.global::<SettingsStore>().settings().alerts;
        let warnings = self.memory_watch.check(
            &self.memory,
            alerts.memory_pane_gb as u64 * GB,
            alerts.memory_total_gb as u64 * GB,
        );
        for warning in warnings {
            let (key, summary, body) = match warning {
                MemoryWarning::Pane(key, bytes) => {
                    (Some(key), format!("A pane is using {}", short(bytes)), "Close or hide panes you no longer need.")
                }
                MemoryWarning::Total(bytes) => (
                    self.memory.iter().max_by_key(|(_, bytes)| **bytes).map(|(key, _)| key.clone()),
                    format!("Panes are using {} together", short(bytes)),
                    "Close panes you no longer need; the sidebar shows each one's memory.",
                ),
            };
            let Some(leaf) = key.and_then(|key| self.workspace.leaf_for_pane(&key)) else { continue };
            let title = self
                .workspace
                .leaf_name(leaf)
                .map(String::from)
                .or_else(|| self.slots.get(&leaf).map(|slot| slot.view.read(cx).title()))
                .unwrap_or_default();
            let notice = Notice { leaf, summary, body: format!("{title}\n{body}") };
            let channels = Channels { badge: false, notification: true, sound: None };
            alerts::deliver(notice, channels, alerts.volume, self.clicked.clone());
        }
    }
}
