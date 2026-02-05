import { StateField } from "@codemirror/state";
import { EditorView, Decoration, DecorationSet, WidgetType } from "@codemirror/view";
import katex from "katex";

// Widget that renders KaTeX
class MathPreviewWidget extends WidgetType {
    constructor(readonly src: string, readonly isBlock: boolean) { super(); }

    toDOM() {
        const outer = document.createElement("span");
        // Style wrapper slightly
        outer.className = "cm-math-preview";
        outer.style.display = this.isBlock ? "flex" : "inline-block";
        outer.style.justifyContent = this.isBlock ? "center" : "flex-start";
        outer.style.userSelect = "none";
        outer.style.cursor = "pointer";
        outer.style.verticalAlign = "middle";

        try {
            katex.render(this.src, outer, {
                displayMode: this.isBlock,
                throwOnError: false,
                macros: { "\\f": "#1f(#2)" }
            });
        } catch (e) {
            outer.textContent = this.src;
        }
        return outer;
    }

    eq(other: MathPreviewWidget) {
        return other.src == this.src && other.isBlock == this.isBlock;
    }

    ignoreEvent() { return false; }
}

// Convert logic to a StateField to handle Multi-line replacements properly
// (ViewPlugins cannot replace line breaks)
export const mathLivePreview = StateField.define<DecorationSet>({
    create(state) {
        return buildDecorations(state.doc.toString(), state.selection.main);
    },
    update(decorations, tr) {
        if (tr.docChanged || tr.selection) {
            return buildDecorations(tr.state.doc.toString(), tr.state.selection.main);
        }
        return decorations;
    },
    provide: f => EditorView.decorations.from(f)
});


function buildDecorations(text: string, selection: { from: number, to: number }): DecorationSet {
    const ranges: any[] = [];

    const inlineMathRegex = /\$([^$\n]+?)\$/g;
    const blockMathRegex = /\$\$([\s\S]+?)\$\$/g;

    let match;
    const blockedRanges = new Set<number>();

    // Find Block Math first 
    while ((match = blockMathRegex.exec(text)) !== null) {
        const from = match.index;
        const to = match.index + match[0].length;
        const content = match[1];

        // Check overlap with cursor
        if (selection.from >= from && selection.to <= to) {
            // Cursor inside -> Show Source
        } else {
            // Cursor outside -> Render
            ranges.push(Decoration.replace({
                widget: new MathPreviewWidget(content, true),
                block: false
            }).range(from, to));

            for (let i = from; i < to; i++) blockedRanges.add(i);
        }
    }

    // Find Inline Math
    while ((match = inlineMathRegex.exec(text)) !== null) {
        const from = match.index;
        const to = match.index + match[0].length;
        const content = match[1];

        if (blockedRanges.has(from)) continue;

        if (selection.from >= from && selection.to <= to) {
            // Cursor inside -> Show Source
        } else {
            ranges.push(Decoration.replace({
                widget: new MathPreviewWidget(content, false)
            }).range(from, to));
        }
    }

    return Decoration.set(ranges.sort((a, b) => a.from - b.from));
}
