// ─── Math Renderer ─────────────────────────────────────────────────
// Compiles Typst math source to an RGBA pixel buffer.
//
// This is a *stub* for Phase 1. The full pipeline requires Typst's
// World trait implementation (fonts, file access). In Phase 1 we
// validate the crate boundary and data flow. The actual rendering
// will be completed once we integrate Cosmic-Text font discovery
// with Typst's font loading in Phase 1.5.
// ────────────────────────────────────────────────────────────────────

use onyx_core::error::{OnyxError, OnyxResult};
use tracing::info;

/// Rendered math output — an RGBA pixel buffer.
#[derive(Debug, Clone)]
pub struct MathPixmap {
    /// RGBA pixels, row-major.
    pub data: Vec<u8>,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
}

/// Stateful math renderer (will hold font cache, Typst world, etc.).
pub struct MathRenderer {
    /// Pixels-per-em for rendered output.
    ppi: f32,
}

impl MathRenderer {
    /// Create a new renderer at the given pixels-per-inch.
    pub fn new(ppi: f32) -> Self {
        info!(ppi, "MathRenderer initialized");
        Self { ppi }
    }

    /// Render a Typst math expression to a pixel buffer.
    ///
    /// `source` should be raw Typst math markup, e.g. `"frac(a, b)"`.
    ///
    /// **Phase 1 stub**: Returns a placeholder 1×1 magenta pixel to
    /// prove the data flow works end-to-end. Real rendering lands in
    /// Phase 1.5 once we wire up Typst's World trait.
    pub fn render(&self, source: &str) -> OnyxResult<MathPixmap> {
        if source.trim().is_empty() {
            return Err(OnyxError::Math("empty math source".into()));
        }

        info!(source, ppi = self.ppi, "rendering math (stub)");

        // Placeholder: 1×1 magenta pixel (#FF00FF)
        Ok(MathPixmap {
            data: vec![255, 0, 255, 255],
            width: 1,
            height: 1,
        })
    }
}

impl Default for MathRenderer {
    fn default() -> Self {
        Self::new(144.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_renders_placeholder() {
        let r = MathRenderer::default();
        let px = r.render("frac(a, b)").unwrap();
        assert_eq!(px.width, 1);
        assert_eq!(px.height, 1);
        assert_eq!(px.data.len(), 4);
    }

    #[test]
    fn empty_source_errors() {
        let r = MathRenderer::default();
        assert!(r.render("").is_err());
    }
}
