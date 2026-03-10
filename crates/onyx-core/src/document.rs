impl OnyxWorkspace {
    /// Atomic snapshot export helper (used by persistence).
    pub fn export_snapshot(&self) -> anyhow::Result<Vec<u8>> {
        Ok(self.doc.export(loro::ExportMode::Snapshot)?)
    }

    /// Delete a node from the tree, remove from search index, and clean up
    /// internal maps immediately so that `node_exists` returns false.
    pub fn delete_node(&mut self, node_id: &str) -> anyhow::Result<()> {
        if let Some(tree_id) = self.id_map.get(node_id) {
            self.tree.delete(*tree_id)?;
        }
        // Remove from maps first so callers observe the deletion
        self.id_map.remove(node_id);
        self.parent_map.remove(node_id);
        if let Some(idx) = self.search_index.as_mut() {
            idx.remove_note(node_id)?;
        }
        self.maybe_commit();
        Ok(())
    }

    /// Return true if a node with the given ID is present in the workspace.
    pub fn node_exists(&self, id: &str) -> bool {
        self.id_map.contains_key(id)
    }

    /// Total number of nodes currently tracked by the workspace.
    pub fn node_count(&self) -> usize {
        self.id_map.len()
    }

    /// Return true if a layout row ID exists.
    pub fn row_exists(&self, id: &str) -> bool {
        self.layout_id_map.contains_key(id)
    }

    /// Check whether `slot_id` is currently a direct child of `row_id`.
    pub fn row_contains_slot(&self, row_id: &str, slot_id: &str) -> bool {
        self.layout_parent_map
            .get(slot_id)
            .map(|p| p == row_id)
            .unwrap_or(false)
    }

    /// Expand a previously-collapsed layout row.
    pub fn expand_row(&mut self, row_id: &str) -> anyhow::Result<()> {
        let tid = *self.layout_id_map.get(row_id).context("row not found")?;
        let meta = self.layout_tree.get_meta(tid).context("get meta")?;
        meta.insert("collapsed", "false").context("set collapsed")?;
        self.maybe_commit();
        Ok(())
    }

    /// Change the title of any node.
    pub fn set_node_title(&mut self, id: &str, title: &str) -> anyhow::Result<()> {
        let &tree_id = self.id_map.get(id).context("node not found")?;
        let meta = self.tree.get_meta(tree_id).context("get meta")?;
        meta.insert("title", title).context("set title")?;
        self.maybe_commit();
        Ok(())
    }

    /// Move a node to a new parent (or root if `None`), rejecting cyclic
    /// reparenting attempts.
    pub fn move_node(&mut self, node_id: &str, new_parent: Option<&str>) -> anyhow::Result<()> {
        let &node_tid = self.id_map.get(node_id).context("node not found")?;

        if let Some(pid) = new_parent {
            let &parent_tid = self.id_map.get(pid).context("parent not found")?;

            // Cycle check: walk upward from the new parent.  We avoid
            // the `TreeParentId` mismatch by converting to string and
            // consulting `id_map` for the corresponding `TreeID`.
            let node_str = node_tid.to_string();
            let mut curr_tid_opt = Some(parent_tid);
            while let Some(curr_tid) = curr_tid_opt {
                if curr_tid.to_string() == node_str {
                    anyhow::bail!("cyclic reparenting rejected");
                }
                curr_tid_opt = if let Some(parent_parent) = self.tree.parent(curr_tid) {
                    parent_parent.tree_id()
                } else {
                    None
                };
            }
            self.tree.mov(node_tid, parent_tid)?;
        } else {
            self.tree.mov(node_tid, None)?;
        }
        self.maybe_commit();
        Ok(())
    }

    /// Rebuild all of the internal ID->TreeID maps from the LoroDoc state.
    /// Used after importing snapshots or other operations that bypass the
    /// normal mutators.
    pub fn rebuild_id_map(&mut self) {
        self.id_map.clear();
        self.parent_map.clear();
        self.layout_id_map.clear();
        self.layout_parent_map.clear();

        // 1. Nodes
        let mut stack = Vec::new();
        for root in self.tree.roots() {
            stack.push((root, None::<String>));
        }

        while let Some((id, parent_void)) = stack.pop() {
            let id_str = id.to_string();
            self.id_map.insert(id_str.clone(), id);

            let mut is_void = false;
            if let Ok(meta) = self.tree.get_meta(id) {
                if let Some(ValueOrContainer::Value(v)) = meta.get("node_type") {
                    if let Some(s) = v.as_string() {
                        if s.as_str() == "void" {
                            is_void = true;
                        }
                    }
                }
            }

            if !is_void {
                if let Some(ref p) = parent_void {
                    self.parent_map.insert(id_str.clone(), p.clone());
                }
            }
            let next_ctx = if is_void { Some(id_str) } else { parent_void };
            if let Some(children) = self.tree.children(id) {
                for child in children {
                    stack.push((child, next_ctx.clone()));
                }
            }
        }

        // 2. Layout
        let mut stack = Vec::new();
        for root in self.layout_tree.roots() {
            stack.push((root, None::<String>));
        }
        while let Some((id, parent_id)) = stack.pop() {
            let id_str = id.to_string();
            self.layout_id_map.insert(id_str.clone(), id);
            if let Some(ref p) = parent_id {
                self.layout_parent_map.insert(id_str.clone(), p.clone());
            }
            if let Some(children) = self.layout_tree.children(id) {
                for child in children {
                    stack.push((child, Some(id_str.clone())));
                }
            }
        }
    }
}
// ─── Onyx Core — Workspace Document (LoroTree + LoroMap) ───────────

use std::collections::HashMap;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use loro::{LoroDoc, LoroMap, LoroTree, LoroValue, TreeID, ValueOrContainer};

use crate::blob::BlobStore;
use crate::blocks::{Block, BlockType};
use crate::fsrs::FlashcardData;
use crate::graph::BacklinkIndex;
use crate::history::HistoryStack;
use crate::import;
use crate::model::{NodeType, OnyxNode, PropertyDefinition, PropertyType};
use crate::search::SearchIndex;

/// The core CRDT-backed workspace. Uses LoroTree for hierarchy and
/// LoroMap for void-scoped properties.
pub struct OnyxWorkspace {
    pub doc: LoroDoc,
    pub tree: LoroTree,
    /// Additional tree used for grid layout rows/slots. Slots are
    /// nodes within this tree so that moves can be performed atomically
    /// with a single `loro_tree.move` operation.
    pub layout_tree: LoroTree,
    pub properties: LoroMap,
    pub schemas: LoroMap,
    pub node_values: LoroMap,
    pub vectors: LoroMap,
    pub blocks: LoroMap,
    pub flashcards: LoroMap,
    /// map of canvas element id -> serialized CanvasElement json
    pub canvas_elements: LoroMap,
    pub blob_store: BlobStore,
    pub search_index: Option<SearchIndex>,
    pub history: HistoryStack,
    pub graph: BacklinkIndex,
    id_map: HashMap<String, TreeID>,
    parent_map: HashMap<String, String>,
    /// Maps used only for the layout tree (rows and slots).
    layout_id_map: HashMap<String, TreeID>,
    layout_parent_map: HashMap<String, String>,
    /// When true, `doc.commit()` is deferred until `end_transaction()`.
    batching: bool,
}

impl Default for OnyxWorkspace {
    fn default() -> Self {
        Self::new()
    }
}

impl OnyxWorkspace {
    pub fn id_map(&self) -> &HashMap<String, TreeID> {
        &self.id_map
    }

    pub fn new() -> Self {
        let doc = LoroDoc::new();
        let tree = doc.get_tree("nodes");
        let layout_tree = doc.get_tree("layout");
        let properties = doc.get_map("properties");
        let schemas = doc.get_map("schemas");
        let node_values = doc.get_map("node_values");
        let vectors = doc.get_map("vectors");
        let blocks = doc.get_map("blocks");
        let flashcards = doc.get_map("flashcards");
        let canvas_elements = doc.get_map("canvas_elements");

        let mut ws = Self {
            doc,
            tree,
            layout_tree,
            properties,
            schemas,
            node_values,
            vectors,
            blocks,
            flashcards,
            canvas_elements,
            blob_store: BlobStore::new(),
            search_index: SearchIndex::new().ok(),
            history: HistoryStack::new(),
            graph: BacklinkIndex::new(),
            id_map: HashMap::new(),
            parent_map: HashMap::new(),
            layout_id_map: HashMap::new(),
            layout_parent_map: HashMap::new(),
            batching: false,
        };

        // --- GENESIS BLOCK CREATION ---
        // make a root void and a note with a single styled block so that
        // an editor rendering pipeline has something to display immediately.
        if let Ok(root_void_id) = ws.create_void(None, "Welcome to the Void") {
            if let Ok(note_id) = ws.create_note(&root_void_id, "Genesis Note") {
                let block_content = "";
                let block = Block {
                    id: uuid::Uuid::new_v4().to_string(),
                    kind: BlockType::Paragraph,
                    content: block_content.to_string(),
                    attributes: vec![],
                    children: Vec::new(),
                    align: String::from("left"),
                    indent_level: 0,
                };
                let _ = ws.set_note_blocks(&note_id, &[block]);
            }
        }
        // --- END GENESIS BLOCK ---

        ws
    }

    /// Begin a transaction — suppress individual `doc.commit()` calls.
    pub fn begin_transaction(&mut self) {
        self.batching = true;
    }

    /// End a transaction — flush a single `doc.commit()`.
    pub fn end_transaction(&mut self) {
        self.batching = false;
        self.doc.commit();
    }

    /// Conditionally commit: only if not inside a transaction batch.
    fn maybe_commit(&mut self) {
        if !self.batching {
            self.doc.commit();
        }
    }

    /// Restore a workspace from a LoroDoc snapshot (exported bytes).
    pub fn from_snapshot(data: &[u8]) -> anyhow::Result<Self> {
        let doc = LoroDoc::new();
        doc.import(data)?;
        let tree = doc.get_tree("nodes");
        let layout_tree = doc.get_tree("layout");
        let properties = doc.get_map("properties");
        let schemas = doc.get_map("schemas");
        let node_values = doc.get_map("node_values");
        let vectors = doc.get_map("vectors");
        let blocks = doc.get_map("blocks");
        let flashcards = doc.get_map("flashcards");
        let canvas_elements = doc.get_map("canvas_elements");

        // Rebuild id_map and parent_map from the tree
        let mut id_map = HashMap::new();
        let mut parent_map = HashMap::new();
        let mut layout_id_map = HashMap::new();
        let mut layout_parent_map = HashMap::new();
        fn walk_tree(
            tree: &LoroTree,
            id: loro::TreeID,
            parent_void: Option<&str>,
            id_map: &mut HashMap<String, TreeID>,
            parent_map: &mut HashMap<String, String>,
        ) {
            let id_str = id.to_string();
            id_map.insert(id_str.clone(), id);

            let meta = match tree.get_meta(id) {
                Ok(m) => m,
                Err(_) => return,
            };
            let node_type = match meta.get("node_type") {
                Some(ValueOrContainer::Value(v)) => {
                    v.as_string().map(|s| s.to_string()).unwrap_or_default()
                }
                _ => String::new(),
            };

            let current_void = if node_type == "void" {
                Some(id_str.as_str())
            } else {
                if let Some(pv) = parent_void {
                    parent_map.insert(id_str.clone(), pv.to_string());
                }
                parent_void
            };

            if let Some(children) = tree.children(id) {
                for child in children {
                    walk_tree(tree, child, current_void, id_map, parent_map);
                }
            }
        }

        for root in tree.roots() {
            walk_tree(&tree, root, None, &mut id_map, &mut parent_map);
        }
        // build layout maps as well (empty if none)
        fn walk_layout(
            tree: &LoroTree,
            id: loro::TreeID,
            parent: Option<&str>,
            id_map: &mut HashMap<String, TreeID>,
            parent_map: &mut HashMap<String, String>,
        ) {
            let id_str = id.to_string();
            id_map.insert(id_str.clone(), id);
            if let Some(p) = parent {
                parent_map.insert(id_str.clone(), p.to_string());
            }
            if let Some(children) = tree.children(id) {
                for child in children {
                    walk_layout(tree, child, Some(&id_str), id_map, parent_map);
                }
            }
        }
        for root in layout_tree.roots() {
            walk_layout(
                &layout_tree,
                root,
                None,
                &mut layout_id_map,
                &mut layout_parent_map,
            );
        }

        Ok(Self {
            doc,
            tree,
            layout_tree,
            properties,
            schemas,
            node_values,
            vectors,
            blocks,
            flashcards,
            canvas_elements,
            blob_store: BlobStore::new(),
            search_index: SearchIndex::new().ok(),
            history: HistoryStack::new(),
            graph: BacklinkIndex::new(),
            id_map,
            parent_map,
            layout_id_map,
            layout_parent_map,
            batching: false,
        })
    }

    /// Create a new Void node. If `parent_id` is None, creates a root void.
    /// Returns the newly-created node ID or an error if the operation fails.
    pub fn create_void(&mut self, parent_id: Option<&str>, title: &str) -> Result<String> {
        self.begin_transaction();
        let tree_id = if let Some(pid) = parent_id {
            let parent = *self.id_map.get(pid).context("parent void not found")?;
            self.tree.create(parent).context("create child void")?
        } else {
            self.tree.create(None).context("create root void")?
        };

        let meta = self.tree.get_meta(tree_id).context("get meta")?;
        meta.insert("node_type", "void").context("set node_type")?;
        meta.insert("title", title).context("set title")?;
        self.end_transaction();

        let id_str = tree_id.to_string();
        self.id_map.insert(id_str.clone(), tree_id);
        Ok(id_str)
    }

    /// Create a Note node under the given parent void.
    pub fn create_note(&mut self, parent_void_id: &str, title: &str) -> Result<String> {
        self.begin_transaction();
        let parent = *self
            .id_map
            .get(parent_void_id)
            .context("parent void not found")?;
        let tree_id = self.tree.create(parent).context("create note")?;

        let meta = self.tree.get_meta(tree_id).context("get meta")?;
        meta.insert("node_type", "note").context("set node_type")?;
        meta.insert("title", title).context("set title")?;
        self.end_transaction();

        let id_str = tree_id.to_string();
        self.id_map.insert(id_str.clone(), tree_id);
        self.parent_map
            .insert(id_str.clone(), parent_void_id.to_string());
        Ok(id_str)
    }

    /// Set a void-scoped property on a note.
    pub fn set_property(
        &mut self,
        note_id: &str,
        void_context_id: &str,
        key: &str,
        value: &str,
    ) -> Result<()> {
        self.begin_transaction();
        let prop_key = format!("{}:{}", note_id, void_context_id);
        let note_props = self
            .properties
            .get_or_create_container(&prop_key, LoroMap::new())
            .context("get or create property map")?;
        note_props.insert(key, value).context("set property")?;
        self.end_transaction();
        Ok(())
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

    /// Return the NodeType of the given node, if it exists.
    pub fn node_type_of(&self, node_id: &str) -> Option<NodeType> {
        let tree_id = self.id_map.get(node_id)?;
        let meta = self.tree.get_meta(*tree_id).ok()?;
        let nt = extract_string(&meta, "node_type");
        Some(if nt == "note" {
            NodeType::Note
        } else {
            NodeType::Void
        })
    }

    /// Return the title of the given node, if it exists.
    pub fn node_title(&self, node_id: &str) -> Option<String> {
        let tree_id = self.id_map.get(node_id)?;
        let meta = self.tree.get_meta(*tree_id).ok()?;
        Some(extract_string(&meta, "title"))
    }

    /// Return the parent void ID of a note.
    pub fn parent_void_of(&self, note_id: &str) -> Option<String> {
        self.parent_map.get(note_id).cloned()
    }

    /// Recursively build the path of parent voids from the given node back to root.
    /// Returns a list of (id, title) pairs.
    pub fn get_path_to_root(&self, node_id: &str) -> Vec<(String, String)> {
        let mut path = Vec::new();
        let Some(&tid) = self.id_map.get(node_id) else {
            return path;
        };

        let mut curr_tid = tid;
        while let Some(parent_id) = self.tree.parent(curr_tid) {
            if let Some(pid) = parent_id.tree_id() {
                if let Some(title) = self.node_title(&pid.to_string()) {
                    path.push((pid.to_string(), title));
                }
                curr_tid = pid;
            } else {
                break;
            }
        }
        path.reverse();
        path
    }

    /// Return all immediate child voids of a given void.
    pub fn get_sub_voids(&self, void_id: &str) -> Vec<OnyxNode> {
        let mut result = Vec::new();
        let Some(&tid) = self.id_map.get(void_id) else {
            return result;
        };

        if let Some(children) = self.tree.children(tid) {
            for child in children {
                if let Some(node_type) = self.node_type_of(&child.to_string()) {
                    if node_type == NodeType::Void {
                        if let Some(title) = self.node_title(&child.to_string()) {
                            result.push(OnyxNode {
                                id: child.to_string(),
                                node_type: NodeType::Void,
                                title,
                            });
                        }
                    }
                }
            }
        }
        result
    }

    /// Set the canvas elements for a void.
    pub fn set_canvas_elements(&mut self, void_id: &str, elements: &[crate::canvas::CanvasElement]) -> Result<()> {
        self.begin_transaction();
        let json = serde_json::to_string(elements).context("serialize canvas elements")?;
        self.canvas_elements.insert(void_id, json).context("set canvas elements")?;
        self.end_transaction();
        Ok(())
    }

    /// Get the canvas elements for a void.
    pub fn get_canvas_elements(&self, void_id: &str) -> Vec<crate::canvas::CanvasElement> {
        match self.canvas_elements.get(void_id) {
            Some(ValueOrContainer::Value(v)) => v
                .as_string()
                .and_then(|s| serde_json::from_str(s).ok())
                .unwrap_or_default(),
            _ => Vec::new(),
        }
    }

    /// Add a property definition to a void's schema.
    pub fn add_property_schema(
        &mut self,
        void_id: &str,
        name: &str,
        kind: PropertyType,
    ) -> Result<()> {
        self.begin_transaction();
        let mut defs = self.get_active_schema(void_id);
        defs.push(PropertyDefinition {
            name: name.to_string(),
            kind,
        });
        let json = serde_json::to_string(&defs).context("serialize schema")?;
        self.schemas.insert(void_id, json).context("set schema")?;
        self.end_transaction();
        Ok(())
    }

    /// Get the active schema for a void.
    pub fn get_active_schema(&self, void_id: &str) -> Vec<PropertyDefinition> {
        match self.schemas.get(void_id) {
            Some(ValueOrContainer::Value(v)) => v
                .as_string()
                .and_then(|s| serde_json::from_str(s).ok())
                .unwrap_or_default(),
            _ => Vec::new(),
        }
    }

    /// Set a property value on a note within a void context.
    pub fn set_note_property(
        &mut self,
        note_id: &str,
        void_context_id: &str,
        key: &str,
        value: &str,
    ) -> Result<()> {
        self.begin_transaction();
        let note_map = self
            .node_values
            .get_or_create_container(note_id, LoroMap::new())
            .context("note map")?;
        let void_map = note_map
            .get_or_create_container(void_context_id, LoroMap::new())
            .context("void map")?;
        void_map.insert(key, value).context("set value")?;
        self.end_transaction();
        Ok(())
    }

    /// Return all note IDs in the workspace.
    pub fn all_note_ids(&self) -> Vec<String> {
        self.parent_map.keys().cloned().collect()
    }

    /// Store an embedding vector for a note.
    pub fn set_vector(&mut self, note_id: &str, vec: &[f32]) -> Result<()> {
        self.begin_transaction();
        let json = serde_json::to_string(vec).context("serialize vector")?;
        self.vectors.insert(note_id, json).context("set vector")?;
        self.end_transaction();
        Ok(())
    }

    /// Retrieve the embedding vector for a note.
    pub fn get_vector(&self, note_id: &str) -> Option<Vec<f32>> {
        match self.vectors.get(note_id) {
            Some(ValueOrContainer::Value(v)) => {
                v.as_string().and_then(|s| serde_json::from_str(s).ok())
            }
            _ => None,
        }
    }

    /// Get all property values for a note within a void context.
    pub fn get_note_values(&self, note_id: &str, void_context_id: &str) -> HashMap<String, String> {
        let mut result = HashMap::new();
        let deep = self.node_values.get_deep_value();
        if let LoroValue::Map(top) = &deep {
            if let Some(LoroValue::Map(note_map)) = top.get(note_id) {
                if let Some(LoroValue::Map(void_map)) = note_map.get(void_context_id) {
                    for (k, v) in void_map.iter() {
                        if let LoroValue::String(s) = v {
                            result.insert(k.clone(), s.to_string());
                        }
                    }
                }
            }
        }
        result
    }

    // ── Layout helpers (layout_tree based) ─────────────────────────

    /// Create a new layout row (root or as child of another row).
    pub fn create_layout_row(&mut self, parent_row: Option<&str>) -> Result<String> {
        self.begin_transaction();
        let tree_id = if let Some(pid) = parent_row {
            let parent = *self
                .layout_id_map
                .get(pid)
                .context("parent layout row not found")?;
            self.layout_tree
                .create(parent)
                .context("create child layout row")?
        } else {
            self.layout_tree
                .create(None)
                .context("create root layout row")?
        };
        let id_str = tree_id.to_string();
        self.layout_id_map.insert(id_str.clone(), tree_id);
        if let Some(pid) = parent_row {
            self.layout_parent_map
                .insert(id_str.clone(), pid.to_string());
        }
        self.end_transaction();
        Ok(id_str)
    }

    /// Create a slot node under an existing row.
    pub fn create_layout_slot(&mut self, row_id: &str) -> Result<String> {
        let parent = *self
            .layout_id_map
            .get(row_id)
            .context("layout row not found")?;
        self.begin_transaction();
        let slot_id = self.layout_tree.create(parent).context("create slot")?;
        let id_str = slot_id.to_string();
        self.layout_id_map.insert(id_str.clone(), slot_id);
        self.layout_parent_map
            .insert(id_str.clone(), row_id.to_string());
        self.end_transaction();
        Ok(id_str)
    }

    /// Move a slot node to another row at the given index.
    /// Performs a single LoroTree move operation.
    pub fn move_slot(&mut self, slot_id: &str, to_row_id: &str, _index: usize) -> Result<()> {
        let slot_tid = *self
            .layout_id_map
            .get(slot_id)
            .context("slot id not found")?;
        let dest_tid = *self
            .layout_id_map
            .get(to_row_id)
            .context("destination row not found")?;
        // LoroTree's method is now `mov` rather than `move` and it returns a Result.
        // we don't need the index parameter any more so ignore it.
        self.layout_tree
            .mov(slot_tid, dest_tid)
            .context("move slot")?;
        // update our auxiliary map so that callers can query the new parent
        self.layout_parent_map
            .insert(slot_id.to_string(), to_row_id.to_string());
        self.maybe_commit();
        Ok(())
    }

    /// Mark a row collapsed rather than deleting it.
    pub fn collapse_row(&mut self, row_id: &str) -> Result<()> {
        let tid = *self.layout_id_map.get(row_id).context("row id not found")?;
        let meta = self.layout_tree.get_meta(tid).context("get row meta")?;
        meta.insert("collapsed", "true")
            .context("set collapsed flag")?;
        self.maybe_commit();
        Ok(())
    }

    /// Query whether a layout row is collapsed.
    pub fn is_row_collapsed(&self, row_id: &str) -> bool {
        if let Some(tid) = self.layout_id_map.get(row_id) {
            if let Ok(meta) = self.layout_tree.get_meta(*tid) {
                if let Some(ValueOrContainer::Value(v)) = meta.get("collapsed") {
                    // `as_string` returns an `Option<&String>` so we double-deref
                    // or convert to &str before comparing.
                    return v.as_string().map(|s| s.as_str() == "true").unwrap_or(false);
                }
            }
        }
        false
    }

    // ── Block Engine ────────────────────────────────────────────

    /// Set the blocks for a note (JSON-serialized Vec<Block>).
    /// Automatically triggers backlink graph updates.
    pub fn set_note_blocks(&mut self, note_id: &str, blocks: &[Block]) -> Result<()> {
        self.begin_transaction();
        let json = serde_json::to_string(blocks).context("serialize blocks")?;
        self.blocks.insert(note_id, json).context("set blocks")?;
        self.end_transaction();

        // Update backlink graph from block content
        let content: String = blocks
            .iter()
            .map(|b| b.content.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        self.graph.update_links(note_id, &content);
        Ok(())
    }

    /// Get the blocks for a note.
    pub fn get_note_blocks(&self, note_id: &str) -> Vec<Block> {
        match self.blocks.get(note_id) {
            Some(ValueOrContainer::Value(v)) => v
                .as_string()
                .and_then(|s| serde_json::from_str(s).ok())
                .unwrap_or_default(),
            _ => Vec::new(),
        }
    }

    /// Return a list of block IDs belonging to the specified note.
    ///
    /// Previously this returned a bare `Vec` but callers were often
    /// interested only in non‑empty results.  The new signature yields
    /// `None` when the note has no blocks, simplifying rendering loops
    /// like `let Some(ids) = ws.get_note_block_ids(id) else { return };`.
    pub fn get_note_block_ids(&self, note_id: &str) -> Option<Vec<String>> {
        let ids: Vec<String> = self
            .get_note_blocks(note_id)
            .into_iter()
            .map(|b| b.id.clone())
            .collect();
        if ids.is_empty() {
            None
        } else {
            Some(ids)
        }
    }

    /// Retrieve the raw content string for a given block ID, or `None` if not
    /// found anywhere in the workspace.
    pub fn get_block_content(&self, block_id: &str) -> Option<String> {
        self.find_block(block_id).map(|b| b.content.clone())
    }

    /// Internal helper: scan every note's block list for a block with the
    /// given ID.
    fn find_block(&self, block_id: &str) -> Option<Block> {
        let deep = self.blocks.get_deep_value();
        if let LoroValue::Map(map) = &deep {
            for (_note_id, val) in map.iter() {
                if let LoroValue::String(json) = val {
                    if let Ok(vec) = serde_json::from_str::<Vec<Block>>(json) {
                        for b in vec {
                            if b.id == block_id {
                                return Some(b);
                            }
                        }
                    }
                }
            }
        }
        None
    }

    /// Helper that converts a given block's text and attribute spans into a
    /// sequence of `(segment, attributes)` tuples suitable for rendering by
    /// a Parley text buffer.  Overlapping attributes are merged into the
    /// vector for each character range; contiguous runs with identical
    /// attribute sets are collapsed.
    pub fn get_styled_text(
        &self,
        block_id: &str,
    ) -> Option<Vec<(String, Vec<crate::blocks::Attribute>)>> {
        // locate the block among stored note blocks by scanning every
        // note's serialized vector.  This is O(n) in number of blocks but
        // acceptable during rendering; an index can be added later.
        let mut found: Option<crate::blocks::Block> = None;
        let deep = self.blocks.get_deep_value();
        if let LoroValue::Map(map) = &deep {
            for (_note_id, val) in map.iter() {
                if let LoroValue::String(json) = val {
                    if let Ok(vec) = serde_json::from_str::<Vec<crate::blocks::Block>>(json) {
                        for b in vec {
                            if b.id == block_id {
                                found = Some(b);
                                break;
                            }
                        }
                    }
                }
                if found.is_some() {
                    break;
                }
            }
        }
        let block = found?;

        // build vector of (pos, start/end, attr) events
        let mut events: Vec<(usize, bool, crate::blocks::Attribute)> = Vec::new();
        for span in &block.attributes {
            events.push((span.start, true, span.attr.clone()));
            events.push((span.end, false, span.attr.clone()));
        }
        events.sort_by_key(|e| e.0);

        let mut segments: Vec<(String, Vec<crate::blocks::Attribute>)> = Vec::new();
        let mut active: Vec<crate::blocks::Attribute> = Vec::new();
        let mut last_pos = 0;
        let text = &block.content;
        let bytes = text.as_bytes();
        for (pos, is_start, attr) in events {
            if pos > last_pos {
                // take slice of bytes last_pos..pos maintaining UTF8 correctness
                if let Ok(seg_str) = std::str::from_utf8(&bytes[last_pos..pos]) {
                    segments.push((seg_str.to_string(), active.clone()));
                }
            }
            if is_start {
                active.push(attr);
            } else {
                if let Some(pos) = active.iter().position(|a| a == &attr) {
                    active.remove(pos);
                }
            }
            last_pos = pos;
        }
        if last_pos < bytes.len() {
            if let Ok(seg_str) = std::str::from_utf8(&bytes[last_pos..]) {
                segments.push((seg_str.to_string(), active.clone()));
            }
        }
        Some(segments)
    }

    // ── Flashcards ──────────────────────────────────────────────

    /// Store a flashcard.
    pub fn set_flashcard(&mut self, card_id: &str, data: &FlashcardData) -> Result<()> {
        self.begin_transaction();
        let json = serde_json::to_string(data).context("serialize flashcard")?;
        self.flashcards
            .insert(card_id, json)
            .context("set flashcard")?;
        self.end_transaction();
        Ok(())
    }

    /// Get a flashcard by ID.
    pub fn get_flashcard(&self, card_id: &str) -> Option<FlashcardData> {
        match self.flashcards.get(card_id) {
            Some(ValueOrContainer::Value(v)) => {
                v.as_string().and_then(|s| serde_json::from_str(s).ok())
            }
            _ => None,
        }
    }

    // ── Canvas Elements ───────────────────────────────────────────

    /// Store or update a canvas element in the workspace.
    pub fn set_canvas_element(
        &mut self,
        elem_id: &str,
        element: &crate::canvas::CanvasElement,
    ) -> Result<()> {
        self.begin_transaction();
        let json = serde_json::to_string(element).context("serialize canvas element")?;
        self.canvas_elements
            .insert(elem_id, json)
            .context("set canvas element")?;
        self.end_transaction();
        Ok(())
    }

    /// Retrieve a canvas element by ID.
    pub fn get_canvas_element(&self, elem_id: &str) -> Option<crate::canvas::CanvasElement> {
        match self.canvas_elements.get(elem_id) {
            Some(ValueOrContainer::Value(v)) => {
                v.as_string().and_then(|s| serde_json::from_str(s).ok())
            }
            _ => None,
        }
    }

    // ── Calendar / Due Dates ────────────────────────────────────

    /// Return note IDs that have flashcards due between `start` and `end`.
    pub fn get_notes_due_between(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Vec<String> {
        let deep = self.flashcards.get_deep_value();
        let mut result = Vec::new();
        if let LoroValue::Map(map) = &deep {
            for (_card_id, val) in map.iter() {
                if let LoroValue::String(json) = val {
                    if let Ok(card) = serde_json::from_str::<FlashcardData>(json) {
                        let due = card.state.last_review;
                        if due >= start && due <= end {
                            result.push(card.note_id.clone());
                        }
                    }
                }
            }
        }
        result
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

// ── Phase 2: Integration Methods ────────────────────────────────

impl OnyxWorkspace {
    /// Return all flashcard IDs stored in the workspace.
    pub fn all_flashcard_ids(&self) -> Vec<String> {
        let deep = self.flashcards.get_deep_value();
        if let LoroValue::Map(map) = &deep {
            map.keys().cloned().collect()
        } else {
            Vec::new()
        }
    }

    // ── Blob Store ──────────────────────────────────────────────

    /// Store a blob and return its content hash.
    pub fn store_blob(&mut self, data: &[u8], mime: &str) -> String {
        self.blob_store.store_blob(data, mime)
    }

    /// Retrieve a blob by hash.
    pub fn get_blob(&self, hash: &str) -> Result<Vec<u8>, crate::blob::BlobError> {
        self.blob_store.get_blob(hash)
    }

    // ── Search Index ────────────────────────────────────────────

    /// Index a note's content for full-text search.
    pub fn index_note_for_search(&mut self, note_id: &str) -> Result<()> {
        // title and void_id retrieval are fallible but we can default to empty
        let title = self.node_title(note_id).unwrap_or_default();
        let void_id = self.parent_void_of(note_id).unwrap_or_default();
        let blocks = self.get_note_blocks(note_id);
        if let Some(ref mut idx) = self.search_index {
            idx.index_note(note_id, &void_id, &title, &blocks)?;
        }
        Ok(())
    }

    /// Full-text search across all indexed notes.
    pub fn search_notes(&self, query: &str, limit: usize) -> Result<Vec<String>> {
        if let Some(ref idx) = self.search_index {
            idx.search(query, limit)
        } else {
            Ok(Vec::new())
        }
    }

    // ── Markdown Import / Export ────────────────────────────────

    /// Import markdown text as a new note under the specified void.
    /// Returns the new note ID.
    pub fn import_markdown(
        &mut self,
        parent_void_id: &str,
        md: &str,
        fallback_title: &str,
    ) -> Result<String> {
        let (title, blocks) = import::import_markdown_text(md, fallback_title);
        let note_id = self.create_note(parent_void_id, &title)?;
        self.set_note_blocks(&note_id, &blocks)?;
        let _ = self.index_note_for_search(&note_id);
        Ok(note_id)
    }

    /// Export a note's blocks as a Markdown string.
    pub fn export_markdown(&self, note_id: &str) -> String {
        let blocks = self.get_note_blocks(note_id);
        import::export_markdown(&blocks)
    }

    // ── History (Undo / Redo) ───────────────────────────────────

    /// Save the current document state as an undo snapshot.
    pub fn push_history_snapshot(&mut self) -> Result<()> {
        self.history.push_snapshot(&self.doc)?;
        Ok(())
    }

    /// Undo the last change. Returns true if successful.
    pub fn undo(&mut self) -> bool {
        match self.history.undo(&self.doc) {
            Ok(Some(restored)) => {
                self.doc = restored;
                self.tree = self.doc.get_tree("nodes");
                self.properties = self.doc.get_map("properties");
                self.schemas = self.doc.get_map("schemas");
                self.node_values = self.doc.get_map("node_values");
                self.vectors = self.doc.get_map("vectors");
                self.blocks = self.doc.get_map("blocks");
                self.flashcards = self.doc.get_map("flashcards");
                self.canvas_elements = self.doc.get_map("canvas_elements");
                true
            }
            _ => false,
        }
    }

    /// Redo the last undone change. Returns true if successful.
    pub fn redo(&mut self) -> bool {
        match self.history.redo(&self.doc) {
            Ok(Some(restored)) => {
                self.doc = restored;
                self.tree = self.doc.get_tree("nodes");
                self.properties = self.doc.get_map("properties");
                self.schemas = self.doc.get_map("schemas");
                self.node_values = self.doc.get_map("node_values");
                self.vectors = self.doc.get_map("vectors");
                self.blocks = self.doc.get_map("blocks");
                self.flashcards = self.doc.get_map("flashcards");
                self.canvas_elements = self.doc.get_map("canvas_elements");
                true
            }
            _ => false,
        }
    }

    /// Whether undo is available.
    pub fn can_undo(&self) -> bool {
        self.history.can_undo()
    }

    /// Whether redo is available.
    pub fn can_redo(&self) -> bool {
        self.history.can_redo()
    }

    // ── Phase 3: Power Features ─────────────────────────────────

    /// Validate whether a LaTeX string is syntactically plausible.
    /// Uses a basic heuristic: balanced braces and no empty input.
    pub fn render_latex_validation(latex: &str) -> bool {
        if latex.trim().is_empty() {
            return false;
        }
        // Check balanced braces
        let mut depth: i32 = 0;
        for ch in latex.chars() {
            match ch {
                '{' => depth += 1,
                '}' => depth -= 1,
                _ => {}
            }
            if depth < 0 {
                return false;
            }
        }
        depth == 0
    }

    /// Get all note IDs that link to the given note.
    pub fn get_backlinks(&self, note_id: &str) -> Vec<String> {
        self.graph.get_backlinks(note_id)
    }

    // ── Convenience wrappers for other core modules ──────────────

    /// Save this workspace to disk (encrypted) using the persistence layer.
    pub fn save_to_path(&self, path: &std::path::Path) -> anyhow::Result<()> {
        crate::persistence::save_workspace(self, path)
    }

    /// Load a workspace from disk (encrypted).
    pub fn load_from_path(path: &std::path::Path) -> anyhow::Result<Self> {
        crate::persistence::load_workspace(path)
    }

    /// Start an autosave thread for this workspace.
    pub fn start_autosave_thread(
        ws: std::sync::Arc<std::sync::Mutex<OnyxWorkspace>>,
        path: String,
        interval: u64,
    ) {
        crate::persistence::start_autosave(ws, path, interval);
    }
}

// ── Layout tree unit tests (CRDT topology) ───────────────────────
#[cfg(test)]
mod layout_tests {
    use super::*;

    #[test]
    fn layout_row_and_slot_operations() -> anyhow::Result<()> {
        let mut ws = OnyxWorkspace::new();
        // create two rows
        let r1 = ws.create_layout_row(None)?;
        let r2 = ws.create_layout_row(None)?;
        // insert slots into first row
        let _s1 = ws.create_layout_slot(&r1)?;
        let s2 = ws.create_layout_slot(&r1)?;
        // move second slot into row2 at index 0
        ws.move_slot(&s2, &r2, 0)?;
        // verify parent maps updated
        assert_eq!(ws.layout_parent_map.get(&s2), Some(&r2));
        // collapse row1 and check flag
        ws.collapse_row(&r1)?;
        assert!(ws.is_row_collapsed(&r1));
        assert!(!ws.is_row_collapsed(&r2));
        Ok(())
    }
}
