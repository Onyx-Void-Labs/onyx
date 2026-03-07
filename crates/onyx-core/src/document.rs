// ─── Onyx Core — Workspace Document (LoroTree + LoroMap) ───────────

use std::collections::HashMap;

use anyhow::Result;
use chrono::{DateTime, Utc};
use loro::{LoroDoc, LoroMap, LoroTree, LoroValue, TreeID, ValueOrContainer};

use crate::blob::BlobStore;
use crate::blocks::Block;
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
    pub properties: LoroMap,
    pub schemas: LoroMap,
    pub node_values: LoroMap,
    pub vectors: LoroMap,
    pub blocks: LoroMap,
    pub flashcards: LoroMap,
    pub blob_store: BlobStore,
    pub search_index: Option<SearchIndex>,
    pub history: HistoryStack,
    pub graph: BacklinkIndex,
    id_map: HashMap<String, TreeID>,
    parent_map: HashMap<String, String>,
}

impl OnyxWorkspace {
    pub fn new() -> Self {
        let doc = LoroDoc::new();
        let tree = doc.get_tree("nodes");
        let properties = doc.get_map("properties");
        let schemas = doc.get_map("schemas");
        let node_values = doc.get_map("node_values");
        let vectors = doc.get_map("vectors");
        let blocks = doc.get_map("blocks");
        let flashcards = doc.get_map("flashcards");
        Self {
            doc,
            tree,
            properties,
            schemas,
            node_values,
            vectors,
            blocks,
            flashcards,
            blob_store: BlobStore::new(),
            search_index: SearchIndex::new().ok(),
            history: HistoryStack::new(),
            graph: BacklinkIndex::new(),
            id_map: HashMap::new(),
            parent_map: HashMap::new(),
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
        self.parent_map
            .insert(id_str.clone(), parent_void_id.to_string());
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

    /// Add a property definition to a void's schema.
    pub fn add_property_schema(&mut self, void_id: &str, name: &str, kind: PropertyType) {
        let mut defs = self.get_active_schema(void_id);
        defs.push(PropertyDefinition {
            name: name.to_string(),
            kind,
        });
        let json = serde_json::to_string(&defs).expect("serialize schema");
        self.schemas.insert(void_id, json).expect("set schema");
        self.doc.commit();
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
    ) {
        let note_map = self
            .node_values
            .get_or_create_container(note_id, LoroMap::new())
            .expect("note map");
        let void_map = note_map
            .get_or_create_container(void_context_id, LoroMap::new())
            .expect("void map");
        void_map.insert(key, value).expect("set value");
        self.doc.commit();
    }

    /// Return all note IDs in the workspace.
    pub fn all_note_ids(&self) -> Vec<String> {
        self.parent_map.keys().cloned().collect()
    }

    /// Store an embedding vector for a note.
    pub fn set_vector(&mut self, note_id: &str, vec: &[f32]) {
        let json = serde_json::to_string(vec).expect("serialize vector");
        self.vectors.insert(note_id, json).expect("set vector");
        self.doc.commit();
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

    // ── Block Engine ────────────────────────────────────────────

    /// Set the blocks for a note (JSON-serialized Vec<Block>).
    /// Automatically triggers backlink graph updates.
    pub fn set_note_blocks(&mut self, note_id: &str, blocks: &[Block]) {
        let json = serde_json::to_string(blocks).expect("serialize blocks");
        self.blocks.insert(note_id, json).expect("set blocks");
        self.doc.commit();

        // Update backlink graph from block content
        let content: String = blocks
            .iter()
            .map(|b| b.content.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        self.graph.update_links(note_id, &content);
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

    // ── Flashcards ──────────────────────────────────────────────

    /// Store a flashcard.
    pub fn set_flashcard(&mut self, card_id: &str, data: &FlashcardData) {
        let json = serde_json::to_string(data).expect("serialize flashcard");
        self.flashcards
            .insert(card_id, json)
            .expect("set flashcard");
        self.doc.commit();
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
    pub fn get_blob(&self, hash: &str) -> Option<Vec<u8>> {
        self.blob_store.get_blob(hash)
    }

    // ── Search Index ────────────────────────────────────────────

    /// Index a note's content for full-text search.
    pub fn index_note_for_search(&mut self, note_id: &str) -> Result<()> {
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
        let note_id = self.create_note(parent_void_id, &title);
        self.set_note_blocks(&note_id, &blocks);
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
    pub fn push_history_snapshot(&mut self) {
        self.history.push_snapshot(&self.doc);
    }

    /// Undo the last change. Returns true if successful.
    pub fn undo(&mut self) -> bool {
        if let Some(restored) = self.history.undo(&self.doc) {
            self.doc = restored;
            self.tree = self.doc.get_tree("nodes");
            self.properties = self.doc.get_map("properties");
            self.schemas = self.doc.get_map("schemas");
            self.node_values = self.doc.get_map("node_values");
            self.vectors = self.doc.get_map("vectors");
            self.blocks = self.doc.get_map("blocks");
            self.flashcards = self.doc.get_map("flashcards");
            true
        } else {
            false
        }
    }

    /// Redo the last undone change. Returns true if successful.
    pub fn redo(&mut self) -> bool {
        if let Some(restored) = self.history.redo(&self.doc) {
            self.doc = restored;
            self.tree = self.doc.get_tree("nodes");
            self.properties = self.doc.get_map("properties");
            self.schemas = self.doc.get_map("schemas");
            self.node_values = self.doc.get_map("node_values");
            self.vectors = self.doc.get_map("vectors");
            self.blocks = self.doc.get_map("blocks");
            self.flashcards = self.doc.get_map("flashcards");
            true
        } else {
            false
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
}
