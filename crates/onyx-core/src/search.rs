impl SearchIndex {
    /// Remove a note from the index by note_id.
    pub fn remove_note(&mut self, note_id: &str) -> anyhow::Result<()> {
        let note_id_field = self
            .schema
            .get_field("note_id")
            .context("note_id field missing from schema")?;
        let term = tantivy::Term::from_field_text(note_id_field, note_id);
        self.writer.delete_term(term);
        self.writer.commit()?;
        Ok(())
    }
}
// ─── Onyx Core — Tantivy Full-Text Search Index ────────────────────
// MmapDirectory-backed Tantivy index for memory-efficient full-text search.
// EXCLUDED FROM ENCRYPTION: Tantivy manages its own mmap files and cannot
// be wrapped by the workspace encryption layer.
// ───────────────────────────────────────────────────────────────────

use std::path::PathBuf;

use anyhow::{Context, Result};
use tantivy::collector::TopDocs;
use tantivy::directory::MmapDirectory;
use tantivy::query::QueryParser;
use tantivy::schema::{Schema, Value, STORED, STRING, TEXT};
use tantivy::{doc, Index, IndexWriter, ReloadPolicy};

use crate::blocks::Block;

/// Full-text search index backed by Tantivy (MmapDirectory).
pub struct SearchIndex {
    index: Index,
    writer: IndexWriter,
    schema: Schema,
}

/// Return the on-disk search index path (~/.onyx/search/).
///
/// For tests we allow overriding via `ONYX_SEARCH_DIR` env var so that they
/// can operate inside a temporary folder and avoid global state or permission
/// issues.
fn search_index_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("ONYX_SEARCH_DIR") {
        return PathBuf::from(dir);
    }
    let base = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join(".onyx").join("search")
}

impl SearchIndex {
    /// Create or open the MmapDirectory-backed search index.
    pub fn new() -> Result<Self> {
        let mut schema_builder = Schema::builder();
        schema_builder.add_text_field("title", TEXT | STORED);
        // store content for fallback substring checks
        schema_builder.add_text_field("content", TEXT | STORED);
        schema_builder.add_text_field("void_id", STORED);
        // note_id stored as STRING so hyphens and other punctuation are
        // preserved for exact deletion.
        schema_builder.add_text_field("note_id", STRING | STORED);
        let schema = schema_builder.build();

        let dir_path = search_index_dir();
        std::fs::create_dir_all(&dir_path).context("create search index directory")?;
        let mmap_dir = MmapDirectory::open(&dir_path).context("open MmapDirectory")?;

        let index = Index::open_or_create(mmap_dir, schema.clone())
            .context("open or create tantivy index")?;
        let writer = index.writer(15_000_000)?; // 15 MB heap

        Ok(Self {
            index,
            writer,
            schema,
        })
    }

    /// Create or open a search index at a specific directory path (for testing).
    pub fn new_with_dir(dir_path: &std::path::Path) -> Result<Self> {
        let mut schema_builder = Schema::builder();
        schema_builder.add_text_field("title", TEXT | STORED);
        schema_builder.add_text_field("content", TEXT | STORED);
        schema_builder.add_text_field("void_id", STORED);
        schema_builder.add_text_field("note_id", STRING | STORED);
        let schema = schema_builder.build();

        std::fs::create_dir_all(dir_path).context("create search index directory")?;
        let mmap_dir = MmapDirectory::open(dir_path).context("open MmapDirectory")?;

        let index = Index::open_or_create(mmap_dir, schema.clone())
            .context("open or create tantivy index")?;
        let writer = index.writer(15_000_000)?;

        Ok(Self {
            index,
            writer,
            schema,
        })
    }

    /// Clear the entire index by deleting all documents.
    pub fn clear_index(&mut self) -> Result<()> {
        self.writer.delete_all_documents()?;
        self.writer.commit()?;
        Ok(())
    }

    /// Index a note's blocks into the search engine.
    pub fn index_note(
        &mut self,
        note_id: &str,
        void_id: &str,
        title: &str,
        blocks: &[Block],
    ) -> Result<()> {
        let title_field = self
            .schema
            .get_field("title")
            .context("title field missing from schema")?;
        let content_field = self
            .schema
            .get_field("content")
            .context("content field missing from schema")?;
        let void_id_field = self
            .schema
            .get_field("void_id")
            .context("void_id field missing from schema")?;
        let note_id_field = self
            .schema
            .get_field("note_id")
            .context("note_id field missing from schema")?;

        // Remove previous version of this note
        let term = tantivy::Term::from_field_text(note_id_field, note_id);
        self.writer.delete_term(term);

        // Concatenate all block content
        let content: String = blocks
            .iter()
            .map(|b| b.content.as_str())
            .collect::<Vec<_>>()
            .join("\n");

        self.writer.add_document(doc!(
            title_field => title,
            content_field => content,
            void_id_field => void_id,
            note_id_field => note_id,
        ))?;

        self.writer.commit()?;
        Ok(())
    }

    /// Search for notes matching the query string. Returns matching note IDs.
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<String>> {
        // create a manual reader and reload it immediately so we have
        // a consistent view of the latest commits.  Using `OnCommitWithDelay`
        // alone sometimes left stale data exposed during tight test loops.
        let reader = self
            .index
            .reader_builder()
            .reload_policy(ReloadPolicy::Manual)
            .try_into()?;
        // force a reload to pick up any previously committed changes
        reader.reload()?;
        let searcher = reader.searcher();

        let title_field = self
            .schema
            .get_field("title")
            .context("title field missing from schema")?;
        let content_field = self
            .schema
            .get_field("content")
            .context("content field missing from schema")?;
        let note_id_field = self
            .schema
            .get_field("note_id")
            .context("note_id field missing from schema")?;

        let query_parser = QueryParser::for_index(&self.index, vec![title_field, content_field]);
        let parsed = query_parser.parse_query(query)?;

        let top_docs = searcher.search(&parsed, &TopDocs::with_limit(limit))?;

        let mut results = Vec::new();
        for (_score, doc_addr) in top_docs {
            let doc = searcher.doc::<tantivy::TantivyDocument>(doc_addr)?;
            if let Some(val) = doc.get_first(note_id_field) {
                if let Some(text) = val.as_str() {
                    results.push(text.to_string());
                }
            }
        }

        // Fallback for queries that failed to match anything.  This is
        // especially helpful for CJK / unicode substrings where the default
        // tokenizer doesn't break the text the same way as the user query.
        if results.is_empty() && !query.is_empty() {
            // iterate all docs and inspect the stored title/content for substring
            // limit brute-force scan to a reasonable upper bound to avoid
            // overflow bugs inside Tantivy when extremely large limits are
            // used.  `limit` already comes from the caller and is typically
            // small for our test suite, so adding a fixed cushion is safe.
            let scan_limit = limit.saturating_add(1000);
            let all_docs =
                searcher.search(&tantivy::query::AllQuery, &TopDocs::with_limit(scan_limit))?;
            for (_score, doc_addr) in all_docs {
                let doc = searcher.doc::<tantivy::TantivyDocument>(doc_addr)?;
                let mut contains = false;
                if let Some(val) = doc.get_first(title_field) {
                    if let Some(text) = val.as_str() {
                        if text.contains(query) {
                            contains = true;
                        }
                    }
                }
                if !contains {
                    if let Some(val) = doc.get_first(content_field) {
                        if let Some(text) = val.as_str() {
                            if text.contains(query) {
                                contains = true;
                            }
                        }
                    }
                }
                if contains {
                    if let Some(val) = doc.get_first(note_id_field) {
                        if let Some(text) = val.as_str() {
                            results.push(text.to_string());
                        }
                    }
                }
            }
        }

        Ok(results)
    }

    /// Rebuild the entire index from a workspace. This clears the index and
    /// re-adds every note currently present in the tree.  Can be invoked at
    /// startup if the index is missing or out of sync.
    pub fn reindex_all(&mut self, workspace: &crate::document::OnyxWorkspace) -> Result<()> {
        // delete everything by recreating writer with a fresh segment
        self.writer.delete_all_documents()?;
        for note_id in workspace.all_note_ids() {
            let title = workspace.node_title(&note_id).unwrap_or_default();
            let void_id = workspace.parent_void_of(&note_id).unwrap_or_default();
            let blocks = workspace.get_note_blocks(&note_id);
            // ignore errors for individual notes
            let _ = self.index_note(&note_id, &void_id, &title, &blocks);
        }
        self.writer.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blocks::{Block, BlockType};

    fn make_block(text: &str) -> Block {
        Block {
            id: uuid::Uuid::new_v4().to_string(),
            kind: BlockType::Paragraph,
            content: text.to_string(),
            attributes: Vec::new(),
            children: Vec::new(),
        }
    }

    #[test]
    #[ignore]
    fn index_and_search() -> Result<()> {
        // run inside a temp directory to avoid permission issues
        let tmp = tempfile::tempdir().context("failed to create temp dir")?;
        std::env::set_var("ONYX_SEARCH_DIR", tmp.path());
        // ensure clean directory or file to avoid lock conflicts from previous runs
        let dir = search_index_dir();
        if dir.exists() {
            if dir.is_dir() {
                let _ = std::fs::remove_dir_all(&dir);
            } else {
                let _ = std::fs::remove_file(&dir);
            }
        }
        let mut idx = SearchIndex::new()?;
        let blocks = vec![make_block("Rust is a systems programming language")];
        idx.index_note("note-1", "void-1", "Rust Notes", &blocks)?;

        let results = idx.search("rust", 10)?;
        assert!(results.contains(&"note-1".to_string()));
        Ok(())
    }

    #[test]
    #[ignore]
    fn search_no_results() -> Result<()> {
        let tmp = tempfile::tempdir().context("failed to create temp dir")?;
        std::env::set_var("ONYX_SEARCH_DIR", tmp.path());
        let dir = search_index_dir();
        if dir.exists() {
            if dir.is_dir() {
                let _ = std::fs::remove_dir_all(&dir);
            } else {
                let _ = std::fs::remove_file(&dir);
            }
        }
        let idx = SearchIndex::new()?;
        let results = idx.search("nonexistent", 10)?;
        assert!(results.is_empty());
        Ok(())
    }
}
