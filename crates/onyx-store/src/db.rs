// ─── Embedded SurrealDB Store ──────────────────────────────────────
// Thin wrapper around SurrealDB running in embedded / kv-mem mode.
// Gated behind the `surrealdb-backend` feature flag to avoid
// linking ~25MB of database engine when not needed.
// ────────────────────────────────────────────────────────────────────
#![cfg(feature = "surrealdb-backend")]

use onyx_core::document::Document;
use onyx_core::error::{OnyxError, OnyxResult};
use onyx_core::id::OnyxId;
use surrealdb::engine::local::{Db, Mem};
use surrealdb::Surreal;
use tracing::info;

/// Handle to the embedded database.
#[derive(Clone)]
pub struct Store {
    db: Surreal<Db>,
}

impl Store {
    /// Boot a new in-memory SurrealDB instance.
    pub async fn init() -> OnyxResult<Self> {
        let db = Surreal::new::<Mem>(())
            .await
            .map_err(|e| OnyxError::Storage(e.to_string()))?;

        db.use_ns("onyx")
            .use_db("void")
            .await
            .map_err(|e| OnyxError::Storage(e.to_string()))?;

        info!("SurrealDB (mem) initialized — ns:onyx db:void");
        Ok(Self { db })
    }

    /// Upsert a document.
    pub async fn save_document(&self, doc: &Document) -> OnyxResult<()> {
        let key = doc.id.to_string();
        let _: Option<Document> = self
            .db
            .update(("documents", &key))
            .content(doc.clone())
            .await
            .map_err(|e| OnyxError::Storage(e.to_string()))?;
        Ok(())
    }

    /// Fetch a document by ID.
    pub async fn get_document(&self, id: &OnyxId) -> OnyxResult<Option<Document>> {
        let key = id.to_string();
        let doc: Option<Document> = self
            .db
            .select(("documents", &key))
            .await
            .map_err(|e| OnyxError::Storage(e.to_string()))?;
        Ok(doc)
    }

    /// List all documents (title + id only — lightweight).
    pub async fn list_documents(&self) -> OnyxResult<Vec<Document>> {
        let docs: Vec<Document> = self
            .db
            .select("documents")
            .await
            .map_err(|e| OnyxError::Storage(e.to_string()))?;
        Ok(docs)
    }

    /// Delete a document.
    pub async fn delete_document(&self, id: &OnyxId) -> OnyxResult<()> {
        let key = id.to_string();
        let _: Option<Document> = self
            .db
            .delete(("documents", &key))
            .await
            .map_err(|e| OnyxError::Storage(e.to_string()))?;
        Ok(())
    }
}
