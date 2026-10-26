//! The saved order of the sidebar: which project comes first, and where each tab sits inside its project.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::split_tree::NodeId;

/// The order the user gave the sidebar, saved in the layout. Projects are listed by their key; tabs by the leaf ids of
/// their panes (a split tab's panes follow each other), top to bottom. Only the relative order inside a project
/// matters for tabs, so one flat list serves all projects.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SidebarOrder {
    /// Project keys, top to bottom.
    #[serde(default)]
    pub projects: Vec<PathBuf>,
    /// Leaf ids of the listed tabs, top to bottom.
    #[serde(default)]
    pub items: Vec<NodeId>,
}

impl SidebarOrder {
    /// Nothing saved.
    pub fn is_empty(&self) -> bool {
        self.projects.is_empty() && self.items.is_empty()
    }

    /// Where the tab made of `leaves` sits: the smallest position in `items` of any of them, `None` if none is listed.
    pub fn rank_of(&self, leaves: &[NodeId]) -> Option<usize> {
        self.items.iter().position(|item| leaves.contains(item))
    }

    /// Forget leaves that no longer exist.
    pub fn retain_leaves(&mut self, live: impl Fn(NodeId) -> bool) {
        self.items.retain(|leaf| live(*leaf));
    }

    /// Remember the order shown now: `projects` replaces the saved projects (the caller passes every project to keep,
    /// in place), the shown tabs come first, in order, then the saved ones that are not shown (an SSH host that is
    /// away, a collapsed project), in their saved order.
    pub fn record(&mut self, projects: &[PathBuf], shown_items: &[NodeId]) {
        self.projects = projects.to_vec();
        let rest: Vec<NodeId> = self.items.iter().filter(|item| !shown_items.contains(item)).copied().collect();
        self.items = shown_items.iter().copied().chain(rest).collect();
    }
}

/// Move the block at `from` before the block at index `before` of the original list (`None` = to the end). Moving a
/// block before itself, or before the block that already follows it, changes nothing.
pub fn move_block(blocks: &mut Vec<Vec<NodeId>>, from: usize, before: Option<usize>) {
    if from >= blocks.len() || before == Some(from) {
        return;
    }
    let block = blocks.remove(from);
    let at = match before {
        Some(before) if before > from => before - 1,
        Some(before) => before,
        None => blocks.len(),
    };
    blocks.insert(at.min(blocks.len()), block);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn move_block_moves_before_and_to_end() {
        let start = vec![vec![1], vec![2, 3], vec![4]];
        let mut blocks = start.clone();
        move_block(&mut blocks, 0, Some(2));
        assert_eq!(blocks, vec![vec![2, 3], vec![1], vec![4]]);
        let mut blocks = start.clone();
        move_block(&mut blocks, 0, None);
        assert_eq!(blocks, vec![vec![2, 3], vec![4], vec![1]]);
        let mut blocks = start.clone();
        move_block(&mut blocks, 2, Some(0));
        assert_eq!(blocks, vec![vec![4], vec![1], vec![2, 3]]);
        for (from, before) in [(1, Some(1)), (0, Some(1)), (2, None), (9, Some(0))] {
            let mut blocks = start.clone();
            move_block(&mut blocks, from, before);
            assert_eq!(blocks, start, "{from} before {before:?}");
        }
    }

    #[test]
    fn record_replaces_projects_and_keeps_unshown_items_after_shown_ones() {
        let mut order =
            SidebarOrder { projects: vec!["/a".into(), "/remote".into(), "/b".into()], items: vec![9, 1, 2] };
        order.record(&["/b".into(), "/a".into()], &[2, 3]);
        assert_eq!(order.projects, vec![PathBuf::from("/b"), "/a".into()], "the caller passes every project to keep");
        assert_eq!(order.items, vec![2, 3, 9, 1]);
    }

    #[test]
    fn retain_leaves_forgets_leaves_that_are_gone() {
        let mut order = SidebarOrder { projects: vec![], items: vec![9, 1, 2, 7] };
        order.retain_leaves(|leaf| leaf == 1 || leaf == 2);
        assert_eq!(order.items, vec![1, 2]);
    }

    #[test]
    fn rank_of_uses_the_smallest_position() {
        let order = SidebarOrder { projects: vec![], items: vec![5, 6, 7] };
        assert_eq!(order.rank_of(&[7, 6]), Some(1));
        assert_eq!(order.rank_of(&[7]), Some(2));
        assert_eq!(order.rank_of(&[8, 9]), None);
        assert_eq!(order.rank_of(&[]), None);
        assert!(!order.is_empty());
        assert!(SidebarOrder::default().is_empty());
    }
}
