// ─── Onyx Core — Tantivy Full-Text Search Index ────────────────────
// In-memory Tantivy index for fast full-text search across notes.
// ───────────────────────────────────────────────────────────────────

use anyhow::Result;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::{Schema, Value, STORED, TEXT};
use tantivy::{doc, Index, IndexWriter, ReloadPolicy};

use crate::blocks::Block;

/// Full-text search index backed by Tantivy.
pub struct SearchIndex {
    index: Index,
    writer: IndexWriter,
    schema: Schema,
}

impl SearchIndex {
    /// Create a new in-memory search index.
    pub fn new() -> Result<Self> {
        let mut schema_builder = Schema::builder();
        schema_builder.add_text_field("title", TEXT | STORED);
        schema_builder.add_text_field("content", TEXT);
        schema_builder.add_text_field("void_id", STORED);
        schema_builder.add_text_field("note_id", TEXT | STORED);
        let schema = schema_builder.build();

        let index = Index::create_in_ram(schema.clone());
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
        let title_field = self.schema.get_field("title").unwrap();
        let content_field = self.schema.get_field("content").unwrap();
        let void_id_field = self.schema.get_field("void_id").unwrap();
        let note_id_field = self.schema.get_field("note_id").unwrap();

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

        let title_field = self.schema.get_field("title").unwrap();
        let content_field = self.schema.get_field("content").unwrap();
        let note_id_field = self.schema.get_field("note_id").unwrap();

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
        let note_id_field = self.schema.get_field("note_id").unwrap();
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
    fn index_and_search() {
        let mut idx = SearchIndex::new().unwrap();
        let blocks = vec![make_block("Rust is a systems programming language")];
        idx.index_note("note-1", "void-1", "Rust Notes", &blocks)
            .unwrap();

        let results = idx.search("rust", 10).unwrap();
        assert!(results.contains(&"note-1".to_string()));
    }

    #[test]
    fn search_no_results() {
        let idx = SearchIndex::new().unwrap();
        let results = idx.search("nonexistent", 10).unwrap();
        assert!(results.is_empty());
    }
}
