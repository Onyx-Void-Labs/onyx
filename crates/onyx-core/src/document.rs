// ─── Onyx Core — Workspace Document (LoroTree + LoroMap) ───────────

use std::collections::HashMap;

use loro::{LoroDoc, LoroMap, LoroTree, TreeID, ValueOrContainer};

use crate::model::{NodeType, OnyxNode};

/// The core CRDT-backed workspace. Uses LoroTree for hierarchy and
/// LoroMap for void-scoped properties.
pub struct OnyxWorkspace {
    pub doc: LoroDoc,
    pub tree: LoroTree,
    pub properties: LoroMap,
    id_map: HashMap<String, TreeID>,
}

impl OnyxWorkspace {
    pub fn new() -> Self {
        let doc = LoroDoc::new();
        let tree = doc.get_tree("nodes");
        let properties = doc.get_map("properties");
        Self {
            doc,
            tree,
            properties,
            id_map: HashMap::new(),
        }
    }

    /// Create a new Void node. If `parent_id` is None, creates a root void.
    pub fn create_void(&mut self, parent_id: Option<&str>, title: &str) -> String {
        let tree_id = if let Some(pid) = parent_id {
            let parent = *self.id_map.get(pid).expect("parent void not found");
            self.tree.create(parent).expect("create child void")
        } else {
            self.tree.create(None).expect("create root void")
        };

        let meta = self.tree.get_meta(tree_id).expect("get meta");
        meta.insert("node_type", "void").expect("set node_type");
        meta.insert("title", title).expect("set title");
        self.doc.commit();

        let id_str = tree_id.to_string();
        self.id_map.insert(id_str.clone(), tree_id);
        id_str
    }

    /// Create a Note node under the given parent void.
    pub fn create_note(&mut self, parent_void_id: &str, title: &str) -> String {
        let parent = *self
            .id_map
            .get(parent_void_id)
            .expect("parent void not found");
        let tree_id = self.tree.create(parent).expect("create note");

        let meta = self.tree.get_meta(tree_id).expect("get meta");
        meta.insert("node_type", "note").expect("set node_type");
        meta.insert("title", title).expect("set title");
        self.doc.commit();

        let id_str = tree_id.to_string();
        self.id_map.insert(id_str.clone(), tree_id);
        id_str
    }

    /// Set a void-scoped property on a note.
    pub fn set_property(&mut self, note_id: &str, void_context_id: &str, key: &str, value: &str) {
        let prop_key = format!("{}:{}", note_id, void_context_id);
        let note_props = self
            .properties
            .get_or_create_container(&prop_key, LoroMap::new())
            .expect("get or create property map");
        note_props.insert(key, value).expect("set property");
        self.doc.commit();
    }

    /// Collect all tree nodes with their depth level for rendering.
    pub fn get_tree_nodes(&self) -> Vec<(OnyxNode, usize)> {
        let mut result = Vec::new();
        for root in self.tree.roots() {
            self.collect_subtree(root, 0, &mut result);
        }
        result
    }

    fn collect_subtree(&self, id: TreeID, depth: usize, result: &mut Vec<(OnyxNode, usize)>) {
        let meta = match self.tree.get_meta(id) {
            Ok(m) => m,
            Err(_) => return,
        };

        let title = extract_string(&meta, "title");
        let node_type_str = extract_string(&meta, "node_type");
        let node_type = if node_type_str == "note" {
            NodeType::Note
        } else {
            NodeType::Void
        };

        result.push((
            OnyxNode {
                id: id.to_string(),
                node_type,
                title,
            },
            depth,
        ));

        if let Some(children) = self.tree.children(id) {
            for child in children {
                self.collect_subtree(child, depth + 1, result);
            }
        }
    }

    /// Return the string ID of the first root void, if any.
    pub fn first_void_id(&self) -> Option<String> {
        self.tree.roots().first().map(|id| id.to_string())
    }
}

fn extract_string(map: &LoroMap, key: &str) -> String {
    match map.get(key) {
        Some(ValueOrContainer::Value(v)) => {
            v.as_string().map(|s| s.to_string()).unwrap_or_default()
        }
        _ => String::new(),
    }
}
