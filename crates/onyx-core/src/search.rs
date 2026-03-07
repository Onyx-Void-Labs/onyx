// ─── Onyx Core — Tantivy Full-Text Search Index ────────────────────
// MmapDirectory-backed Tantivy index for memory-efficient full-text search.
// ───────────────────────────────────────────────────────────────────

use std::path::PathBuf;

use anyhow::{Context, Result};
use tantivy::collector::TopDocs;
use tantivy::directory::MmapDirectory;
use tantivy::query::QueryParser;
use tantivy::schema::{Schema, Value, STORED, TEXT};
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
        schema_builder.add_text_field("content", TEXT);
        schema_builder.add_text_field("void_id", STORED);
        schema_builder.add_text_field("note_id", TEXT | STORED);
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
        let reader = self
            .index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()?;
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

        Ok(results)
    }

    /// Remove a note from the index.
    pub fn remove_note(&mut self, note_id: &str) -> Result<()> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blocks::{Block, BlockType};

    fn make_block(text: &str) -> Block {
        Block {
            id: uuid::Uuid::new_v4().to_string(),
            kind: BlockType::Paragraph,
            content: text.to_string(),
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
