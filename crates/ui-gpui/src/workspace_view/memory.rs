use super::WorkspaceView;
use std::collections::{HashMap, HashSet};
use std::time::Duration;

use app_core::attention::Channels;
use app_core::memory::{MemoryWarning, short};
use app_core::split_tree::NodeId;
use gpui::{
    Context, Div, FontWeight, KeyDownEvent, MouseButton, Stateful, Task, Window, div, prelude::*, px, rgb, rgba,
};
use protocol::host::{HostId, PaneKey};
use protocol::message::PaneId;

use crate::alerts::{self, Notice};
use crate::settings_store::SettingsStore;
use crate::theme::{RADIUS, RADIUS_SMALL, Theme};

/// One running pane in the memory view.
struct MemoryRow {
    leaf: NodeId,
    title: String,
    folder: String,
    hidden: bool,
    bytes: Option<u64>,
}

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

    pub(super) fn open_memory_view(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.memory_view = Some(HashSet::new());
        self.memory_focus.focus(window, cx);
        cx.notify();
    }

    fn close_memory_view(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.memory_view = None;
        self.focus_current(window, cx);
        cx.notify();
    }

    /// The running panes, hidden ones too, biggest first.
    fn memory_rows(&self, cx: &Context<Self>) -> Vec<MemoryRow> {
        let mut rows: Vec<MemoryRow> = self
            .workspace
            .leaves
            .iter()
            .filter_map(|(&leaf, entry)| {
                let key = entry.key()?;
                let title = self
                    .workspace
                    .leaf_name(leaf)
                    .map(String::from)
                    .or_else(|| self.slots.get(&leaf).map(|slot| slot.view.read(cx).title()))
                    .unwrap_or_default();
                let folder =
                    entry.launch.cwd.file_name().map_or_else(String::new, |name| name.to_string_lossy().into());
                let hidden = self.workspace.tab_of(leaf).is_none();
                Some(MemoryRow { leaf, title, folder, hidden, bytes: self.memory.get(&key).copied() })
            })
            .collect();
        rows.sort_by_key(|row| std::cmp::Reverse(row.bytes));
        rows
    }

    /// Every running pane with its memory; pick some to close or hide.
    pub(super) fn render_memory_view(&self, theme: &Theme, cx: &mut Context<Self>) -> Option<Stateful<Div>> {
        let picked = self.memory_view.as_ref()?;
        let rows = self.memory_rows(cx);
        let total: u64 = rows.iter().filter_map(|row| row.bytes).sum();
        let hidden: Vec<NodeId> = rows.iter().filter(|row| row.hidden).map(|row| row.leaf).collect();
        let tokens = theme.tokens;
        let list = rows.into_iter().enumerate().map(|(index, row)| {
            let on = picked.contains(&row.leaf);
            let leaf = row.leaf;
            let check = div()
                .flex_none()
                .size(theme.ui(14.))
                .rounded(px(3.))
                .border_1()
                .border_color(rgb(if on { tokens.accent } else { tokens.line }))
                .when(on, |check| check.bg(rgb(tokens.accent)));
            div()
                .id(("memory-row", index))
                .flex()
                .items_center()
                .gap_3()
                .px_3()
                .h(theme.ui(32.))
                .rounded(px(RADIUS_SMALL))
                .cursor_pointer()
                .hover(|style| style.bg(rgb(tokens.raised)))
                .when(on, |row| row.bg(rgba((tokens.accent << 8) | 0x2e)))
                .child(check)
                .child(div().flex_1().min_w_0().truncate().text_color(rgb(tokens.text)).child(row.title))
                .child(div().flex_none().text_color(rgb(tokens.muted)).child(row.folder))
                .child(div().flex_none().w(theme.ui(48.)).text_color(rgb(tokens.muted)).child(if row.hidden {
                    "hidden"
                } else {
                    ""
                }))
                .child(
                    div()
                        .flex_none()
                        .w(theme.ui(48.))
                        .flex()
                        .justify_end()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(tokens.text))
                        .child(row.bytes.map_or_else(|| "–".into(), short)),
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    if let Some(picked) = &mut this.memory_view
                        && !picked.remove(&leaf)
                    {
                        picked.insert(leaf);
                    }
                    cx.notify();
                }))
        });
        let count = picked.len();
        let button = |id: &'static str, label: String, enabled: bool, danger: bool| {
            div()
                .id(id)
                .flex_none()
                .px_3()
                .h(theme.ui(28.))
                .flex()
                .items_center()
                .rounded(px(RADIUS_SMALL))
                .border_1()
                .border_color(rgb(if danger && enabled { tokens.danger } else { tokens.line }))
                .text_color(rgb(if !enabled {
                    tokens.muted
                } else if danger {
                    tokens.danger
                } else {
                    tokens.text
                }))
                .when(enabled, |button| button.cursor_pointer().hover(|style| style.bg(rgb(tokens.raised))))
                .child(label)
        };
        let card = div()
            .id("memory-card")
            .w(theme.ui(620.))
            .max_h(theme.ui(560.))
            .flex()
            .flex_col()
            .gap_2()
            .p_4()
            .rounded(px(RADIUS))
            .border_1()
            .border_color(rgb(tokens.line))
            .bg(rgb(tokens.surface))
            .text_sm()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(
                div()
                    .flex()
                    .items_center()
                    .child(
                        div()
                            .flex_1()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(rgb(tokens.text))
                            .child("Pane memory"),
                    )
                    .child(div().text_color(rgb(tokens.muted)).child(format!("{} in all", short(total)))),
            )
            .child(div().id("memory-list").flex().flex_col().overflow_y_scroll().children(list))
            .child(
                div()
                    .flex()
                    .gap_2()
                    .pt_2()
                    .child(
                        button(
                            "memory-pick-hidden",
                            format!("Pick hidden ({})", hidden.len()),
                            !hidden.is_empty(),
                            false,
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.memory_view = Some(hidden.iter().copied().collect());
                            cx.notify();
                        })),
                    )
                    .child(div().flex_1())
                    .child(button("memory-hide", format!("Hide {count}"), count > 0, false).on_click(cx.listener(
                        |this, _, window, cx| {
                            let picked = this.memory_view.take().unwrap_or_default();
                            picked.into_iter().for_each(|leaf| this.hide_leaf(leaf, window, cx));
                            this.close_memory_view(window, cx);
                        },
                    )))
                    .child(button("memory-close", format!("Close {count}…"), count > 0, true).on_click(cx.listener(
                        |this, _, window, cx| {
                            let picked: Vec<NodeId> = this.memory_view.take().unwrap_or_default().into_iter().collect();
                            this.close_memory_view(window, cx);
                            this.confirm_close_picked(picked, window, cx);
                        },
                    )))
                    .child(
                        button("memory-done", "Done".into(), true, false)
                            .on_click(cx.listener(|this, _, window, cx| this.close_memory_view(window, cx))),
                    ),
            );
        Some(
            div()
                .id("memory-overlay")
                .occlude()
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .flex()
                .flex_col()
                .items_center()
                .pt(theme.ui(72.))
                .bg(rgba(0x0000_0055))
                .track_focus(&self.memory_focus)
                .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                    if event.keystroke.key == "escape" {
                        this.close_memory_view(window, cx);
                    }
                }))
                .on_mouse_down(MouseButton::Left, cx.listener(|this, _, window, cx| this.close_memory_view(window, cx)))
                .child(card),
        )
    }
}
