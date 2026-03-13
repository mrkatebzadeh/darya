use crate::config::SortMode;
use std::{
    cmp::Ordering,
    collections::HashMap,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub type NodeId = usize;

/// Represents the in-memory tree of filesystem nodes.
#[derive(Debug)]
pub struct FileTree {
    nodes: Vec<TreeNode>,
    pub navigation: NavigationState,
    path_index: HashMap<PathBuf, NodeId>,
}

impl FileTree {
    /// Create a tree containing only the root node.
    pub fn new(root_path: PathBuf) -> Self {
        let mut root = TreeNode::new(root_path.clone(), NodeType::Directory);
        root.id = 0;
        root.expanded = true;

        let mut path_index = HashMap::new();
        path_index.insert(root_path.clone(), 0);

        Self {
            nodes: vec![root],
            navigation: NavigationState::default(),
            path_index,
        }
    }

    pub fn root(&self) -> NodeId {
        0
    }

    pub fn node(&self, id: NodeId) -> Option<&TreeNode> {
        self.nodes.get(id)
    }

    pub fn node_mut(&mut self, id: NodeId) -> Option<&mut TreeNode> {
        self.nodes.get_mut(id)
    }

    pub fn nodes(&self) -> &[TreeNode] {
        &self.nodes
    }

    pub fn add_child(&mut self, parent: NodeId, mut node: TreeNode) -> NodeId {
        let id = self.nodes.len();
        node.id = id;
        node.parent = Some(parent);
        self.path_index.insert(node.path.clone(), id);
        self.nodes.push(node);
        if let Some(parent_node) = self.nodes.get_mut(parent) {
            parent_node.children.push(id);
        }
        id
    }

    pub fn sort_children(&mut self, parent: NodeId, mode: SortMode) {
        if let Some(children) = self.nodes.get(parent).map(|node| node.children.clone()) {
            let mut sorted = children;
            sorted.sort_unstable_by(|left, right| {
                let left_node = &self.nodes[*left];
                let right_node = &self.nodes[*right];
                compare_nodes(left_node, right_node, mode)
            });

            if let Some(parent_node) = self.nodes.get_mut(parent) {
                parent_node.children = sorted;
            }
        }
    }

    pub fn visible_ids(&self) -> Vec<NodeId> {
        let mut ids = Vec::new();
        self.collect_visible(self.root(), &mut ids);
        ids
    }

    pub fn ensure_node(&mut self, path: PathBuf, kind: NodeType) -> NodeId {
        if let Some(&id) = self.path_index.get(&path) {
            return id;
        }

        if path == self.nodes[self.root()].path {
            return self.root();
        }

        let parent_path = path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| self.nodes[self.root()].path.clone());
        let parent_id = self.ensure_node(parent_path, NodeType::Directory);

        let mut node = TreeNode::new(path.clone(), kind);
        node.expanded = kind == NodeType::Directory && path == self.nodes[self.root()].path;
        self.add_child(parent_id, node)
    }

    pub fn add_size(&mut self, node_id: NodeId, size: u64) {
        let mut current = Some(node_id);
        while let Some(id) = current {
            if let Some(node) = self.nodes.get_mut(id) {
                node.size = node.size.saturating_add(size);
                current = node.parent;
            } else {
                break;
            }
        }
    }

    pub fn node_id_for_path(&self, path: &Path) -> Option<NodeId> {
        self.path_index.get(path).copied()
    }

    fn collect_visible(&self, node_id: NodeId, ids: &mut Vec<NodeId>) {
        ids.push(node_id);
        if let Some(node) = self.nodes.get(node_id)
            && node.expanded
        {
            for &child in &node.children {
                self.collect_visible(child, ids);
            }
        }
    }
}

/// Metadata that tracks the user's current selection and scroll offset.
#[derive(Debug, Clone, Copy, Default)]
pub struct NavigationState {
    pub selected: Option<NodeId>,
    pub scroll_offset: usize,
}

impl NavigationState {
    pub fn select(&mut self, id: NodeId) {
        self.selected = Some(id);
    }

    pub fn clear(&mut self) {
        self.selected = None;
    }

    pub fn set_scroll_offset(&mut self, offset: usize) {
        self.scroll_offset = offset;
    }
}

/// Represents a single entry in the filesystem tree.
#[derive(Debug, Clone)]
pub struct TreeNode {
    pub id: NodeId,
    pub parent: Option<NodeId>,
    pub children: Vec<NodeId>,
    pub path: PathBuf,
    pub name: String,
    pub file_type: NodeType,
    pub size: u64,
    pub modified: Option<SystemTime>,
    pub expanded: bool,
}

impl TreeNode {
    pub fn new(path: PathBuf, file_type: NodeType) -> Self {
        let name = extract_name(&path);
        Self {
            id: usize::MAX,
            parent: None,
            children: Vec::new(),
            path,
            name,
            file_type,
            size: 0,
            modified: None,
            expanded: file_type == NodeType::Directory,
        }
    }

    pub fn with_size(mut self, size: u64) -> Self {
        self.size = size;
        self
    }

    pub fn with_modified(mut self, modified: SystemTime) -> Self {
        self.modified = Some(modified);
        self
    }

    pub fn collapsed(mut self) -> Self {
        self.expanded = false;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeType {
    File,
    Directory,
    Symlink,
    Other,
}

fn extract_name(path: &Path) -> String {
    path.file_name()
        .and_then(|os| os.to_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| path.display().to_string())
}

fn compare_nodes(a: &TreeNode, b: &TreeNode, mode: SortMode) -> Ordering {
    match mode {
        SortMode::SizeDesc => b.size.cmp(&a.size).then_with(|| a.name.cmp(&b.name)),
        SortMode::SizeAsc => a.size.cmp(&b.size).then_with(|| a.name.cmp(&b.name)),
        SortMode::Name => a.name.cmp(&b.name),
        SortMode::ModifiedTime => compare_modified(a, b).then_with(|| a.name.cmp(&b.name)),
    }
}

fn compare_modified(a: &TreeNode, b: &TreeNode) -> Ordering {
    duration(b.modified).cmp(&duration(a.modified))
}

fn duration(value: Option<SystemTime>) -> Option<Duration> {
    value.and_then(|time| time.duration_since(UNIX_EPOCH).ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, SystemTime};

    fn make_child(path: &str, size: u64) -> TreeNode {
        TreeNode::new(PathBuf::from(path), NodeType::File).with_size(size)
    }

    #[test]
    fn tree_root_initialized() {
        let tree = FileTree::new(PathBuf::from("/tmp"));
        assert_eq!(tree.root(), 0);
        let root = tree.node(0).expect("root exists");
        assert!(root.expanded);
    }

    #[test]
    fn add_child_updates_parent() {
        let mut tree = FileTree::new(PathBuf::from("/tmp"));
        let child_id = tree.add_child(0, make_child("/tmp/file", 1));
        let root_children = &tree.node(0).unwrap().children;
        assert_eq!(root_children, &[child_id]);
    }

    #[test]
    fn sorts_children_by_size() {
        let mut tree = FileTree::new(PathBuf::from("/tmp"));
        let first = tree.add_child(0, make_child("/tmp/a", 10));
        let second = tree.add_child(0, make_child("/tmp/b", 2));

        tree.sort_children(0, SortMode::SizeDesc);
        assert_eq!(tree.node(0).unwrap().children, vec![first, second]);

        tree.sort_children(0, SortMode::SizeAsc);
        assert_eq!(tree.node(0).unwrap().children, vec![second, first]);
    }

    #[test]
    fn sorts_children_by_modified_time() {
        let mut tree = FileTree::new(PathBuf::from("/tmp"));
        let recent = TreeNode::new(PathBuf::from("/tmp/new"), NodeType::File)
            .with_size(1)
            .with_modified(SystemTime::now());
        let older = TreeNode::new(PathBuf::from("/tmp/old"), NodeType::File)
            .with_size(1)
            .with_modified(SystemTime::now() - Duration::from_secs(10));

        let recent_id = tree.add_child(0, recent);
        let older_id = tree.add_child(0, older);

        tree.sort_children(0, SortMode::ModifiedTime);
        assert_eq!(tree.node(0).unwrap().children, vec![recent_id, older_id]);
    }

    #[test]
    fn navigation_state_tracks_selection() {
        let mut nav = NavigationState::default();
        assert_eq!(nav.selected, None);
        nav.select(5);
        assert_eq!(nav.selected, Some(5));
        nav.clear();
        assert!(nav.selected.is_none());
    }
}
