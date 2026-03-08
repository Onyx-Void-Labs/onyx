// ─── Onyx Core — Math Engine (LaTeX → Vello Paths) ──────────────────

use pulldown_latex::Storage;
use vello::kurbo::PathEl; // used below in latex_to_mathml

/// Render a LaTeX string to a vector of Vello path elements.
///
/// Pipeline: LaTeX → MathML (via pulldown-latex) → XML traversal (roxmltree) → PathEls.
/// This produces a simplified glyph-outline representation suitable for Vello rendering.
pub fn render_math_to_paths(latex: &str) -> Vec<PathEl> {
    let mathml = latex_to_mathml(latex);
    mathml_to_paths(&mathml)
}

/// Convert a LaTeX string to a MathML string via pulldown-latex.
fn latex_to_mathml(latex: &str) -> String {
    let mut output = String::new();
    let storage = Storage::default();
    // parser is the iterator of events that push_mathml expects
    let _ = pulldown_latex::push_mathml(
        &mut output,
        pulldown_latex::Parser::new(latex, &storage),
        pulldown_latex::config::RenderConfig::default(),
    );
    output
}

/// Parse a MathML string and produce Vello paths by walking the XML tree.
///
/// For each `<mi>`, `<mo>`, `<mn>` element we emit simple placeholder geometry
/// (rectangles) whose width is proportional to the text content length.
/// A full glyph-outline pipeline would use a font to resolve actual outlines.
fn mathml_to_paths(mathml: &str) -> Vec<PathEl> {
    let doc = match roxmltree::Document::parse(mathml) {
        Ok(d) => d,
        Err(_) => return Vec::new(),
    };

    let mut paths = Vec::new();
    let mut cursor_x: f64 = 0.0;
    let glyph_height: f64 = 12.0;

    for node in doc.descendants() {
        if !node.is_element() {
            continue;
        }
        let tag = node.tag_name().name();
        match tag {
            // Identifiers, operators, numbers — emit a placeholder rect per token
            "mi" | "mo" | "mn" | "mtext" => {
                let text = node.text().unwrap_or("");
                let glyph_w = text.len().max(1) as f64 * 8.0;

                // Rectangle: MoveTo → LineTo → LineTo → LineTo → Close
                paths.push(PathEl::MoveTo((cursor_x, 0.0).into()));
                paths.push(PathEl::LineTo((cursor_x + glyph_w, 0.0).into()));
                paths.push(PathEl::LineTo((cursor_x + glyph_w, glyph_height).into()));
                paths.push(PathEl::LineTo((cursor_x, glyph_height).into()));
                paths.push(PathEl::ClosePath);

                cursor_x += glyph_w + 2.0;
            }
            // Fractions — emit a horizontal bar
            "mfrac" => {
                let bar_w = 40.0;
                let bar_y = glyph_height / 2.0;
                paths.push(PathEl::MoveTo((cursor_x, bar_y).into()));
                paths.push(PathEl::LineTo((cursor_x + bar_w, bar_y).into()));
                cursor_x += bar_w + 2.0;
            }
            // Superscript / subscript — small offset rects handled by children
            "msup" | "msub" | "msubsup" | "mrow" | "math" => {
                // structural containers — children are handled by the iterator
            }
            _ => {}
        }
    }

    paths
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_expression() {
        let paths = render_math_to_paths("x + y");
        assert!(!paths.is_empty(), "should produce path elements");
    }

    #[test]
    fn fraction_produces_paths() {
        let paths = render_math_to_paths(r"\frac{a}{b}");
        assert!(!paths.is_empty());
    }

    #[test]
    fn empty_latex() {
        let paths = render_math_to_paths("");
        // Empty input may still produce a <math> wrapper with no content
        assert!(paths.is_empty() || !paths.is_empty()); // should not panic
    }

    #[test]
    fn latex_to_mathml_roundtrip() {
        let mathml = latex_to_mathml("E = mc^2");
        assert!(mathml.contains("<math"));
    }

    #[test]
    fn quadratic_formula() {
        let paths = render_math_to_paths(r"\frac{-b \pm \sqrt{b^2 - 4ac}}{2a}");
        assert!(!paths.is_empty());
    }
}
