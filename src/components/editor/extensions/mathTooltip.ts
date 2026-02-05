import { StateField, EditorState } from "@codemirror/state";
import { EditorView, Tooltip, showTooltip } from "@codemirror/view";
import katex from "katex";

function getMathTooltip(state: EditorState): Tooltip | null {
    const { from, to } = state.selection.main;
    const text = state.doc.sliceString(0, state.doc.length);

    // Scan for math ranges around cursor

    // Check if cursor is inside $...$ or $$...$$
    // We iterate matches to find one covering 'from'

    const inlineMathRegex = /\$([^$\n]+?)\$/g;
    const blockMathRegex = /\$\$([\s\S]+?)\$\$/g;

    let match;
    let foundContent = null;
    let isBlock = false;
    let start = 0, end = 0;

    // Check Block First
    while ((match = blockMathRegex.exec(text)) !== null) {
        if (from >= match.index && to <= match.index + match[0].length) {
            foundContent = match[1];
            isBlock = true;
            start = match.index;
            end = match.index + match[0].length;
            break;
        }
    }

    if (!foundContent) {
        // Check Inline
        while ((match = inlineMathRegex.exec(text)) !== null) {
            if (from >= match.index && to <= match.index + match[0].length) {
                foundContent = match[1];
                isBlock = false;
                start = match.index;
                end = match.index + match[0].length;
                break;
            }
        }
    }

    if (!foundContent) return null;

    return {
        pos: start,
        end: end,
        above: false, // Render BELOW
        create(_view: EditorView) {
            const dom = document.createElement("div");
            dom.className = "cm-math-tooltip";
            dom.style.padding = "4px 8px";
            dom.style.backgroundColor = "#27272a"; // Zinc-800
            dom.style.border = "1px solid #3f3f46"; // Zinc-700
            dom.style.borderRadius = "4px";
            dom.style.marginTop = "4px";
            dom.style.zIndex = "100";
            dom.style.minWidth = "20px";
            dom.style.color = "#ececec";
            dom.style.pointerEvents = "none"; // Don't block clicking

            try {
                katex.render(foundContent!, dom, {
                    displayMode: isBlock,
                    throwOnError: false
                });
            } catch (e) {
                dom.textContent = "Invalid LaTeX";
                dom.style.color = "#ef4444";
            }
            return { dom };
        }
    };
}

const mathTooltipField = StateField.define<Tooltip | null>({
    create: getMathTooltip,
    update(tooltip, tr) {
        if (tr.docChanged || tr.selection) return getMathTooltip(tr.state);
        return tooltip;
    },
    provide: f => showTooltip.from(f)
});

export const mathTooltip = [mathTooltipField];
