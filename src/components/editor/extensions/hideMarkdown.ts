/**
 * Hide Markdown Extension for CodeMirror 6
 * 
 * Hides markdown formatting characters (*, #, `, $) when the cursor
 * is not INSIDE that specific formatted segment, giving a clean WYSIWYG look.
 * 
 * This is the Obsidian-style "Live Preview" behavior.
 */

import { EditorView, Decoration, DecorationSet, ViewPlugin, ViewUpdate } from '@codemirror/view';
import { RangeSetBuilder } from '@codemirror/state';

// Decoration to hide text (zero-width, invisible)
const hiddenMark = Decoration.mark({ class: 'cm-hidden-formatting' });

// CSS for hidden marks
export const hideMarkdownTheme = EditorView.theme({
    '.cm-hidden-formatting': {
        fontSize: '0 !important',
        width: '0 !important',
        display: 'inline-block',
        overflow: 'hidden',
        verticalAlign: 'baseline',
    },
});

interface FormatMatch {
    fullStart: number;  // Start of entire match including markers
    fullEnd: number;    // End of entire match including markers
    openStart: number;  // Start of opening marker
    openEnd: number;    // End of opening marker
    closeStart: number; // Start of closing marker
    closeEnd: number;   // End of closing marker
}

/**
 * Plugin that hides markdown formatting when cursor is outside that segment
 */
export const hideMarkdownPlugin = ViewPlugin.fromClass(class {
    decorations: DecorationSet;

    constructor(view: EditorView) {
        this.decorations = this.buildDecorations(view);
    }

    update(update: ViewUpdate) {
        if (update.docChanged || update.selectionSet || update.viewportChanged) {
            this.decorations = this.buildDecorations(update.view);
        }
    }

    buildDecorations(view: EditorView): DecorationSet {
        const builder = new RangeSetBuilder<Decoration>();
        const doc = view.state.doc;
        const cursorPos = view.state.selection.main.head;

        const matches: FormatMatch[] = [];

        // Process line by line
        for (let i = 1; i <= doc.lines; i++) {
            const line = doc.line(i);
            const text = line.text;
            const lineStart = line.from;

            // Note: We DON'T hide heading markers (# ## etc) because:
            // 1. They're at the start of the line and hiding shifts text
            // 2. They're already styled differently (large, colored)
            // 3. Obsidian also doesn't fully hide them in Live Preview

            // Bold **text**
            let match;
            const boldRegex = /\*\*([^*]+)\*\*/g;
            while ((match = boldRegex.exec(text)) !== null) {
                const start = lineStart + match.index;
                matches.push({
                    fullStart: start,
                    fullEnd: start + match[0].length,
                    openStart: start,
                    openEnd: start + 2,
                    closeStart: start + match[0].length - 2,
                    closeEnd: start + match[0].length,
                });
            }

            // Italic *text* (not inside bold)
            const italicRegex = /(?<!\*)\*([^*]+)\*(?!\*)/g;
            while ((match = italicRegex.exec(text)) !== null) {
                const start = lineStart + match.index;
                matches.push({
                    fullStart: start,
                    fullEnd: start + match[0].length,
                    openStart: start,
                    openEnd: start + 1,
                    closeStart: start + match[0].length - 1,
                    closeEnd: start + match[0].length,
                });
            }

            // Inline code `text`
            const codeRegex = /`([^`]+)`/g;
            while ((match = codeRegex.exec(text)) !== null) {
                const start = lineStart + match.index;
                matches.push({
                    fullStart: start,
                    fullEnd: start + match[0].length,
                    openStart: start,
                    openEnd: start + 1,
                    closeStart: start + match[0].length - 1,
                    closeEnd: start + match[0].length,
                });
            }

            // Inline math $text$
            const mathRegex = /\$([^$]+)\$/g;
            while ((match = mathRegex.exec(text)) !== null) {
                const start = lineStart + match.index;
                matches.push({
                    fullStart: start,
                    fullEnd: start + match[0].length,
                    openStart: start,
                    openEnd: start + 1,
                    closeStart: start + match[0].length - 1,
                    closeEnd: start + match[0].length,
                });
            }

            // Strikethrough ~~text~~
            const strikeRegex = /~~([^~]+)~~/g;
            while ((match = strikeRegex.exec(text)) !== null) {
                const start = lineStart + match.index;
                matches.push({
                    fullStart: start,
                    fullEnd: start + match[0].length,
                    openStart: start,
                    openEnd: start + 2,
                    closeStart: start + match[0].length - 2,
                    closeEnd: start + match[0].length,
                });
            }
        }

        // Sort matches by position to ensure decorations are added in order
        matches.sort((a, b) => a.openStart - b.openStart);

        // Add decorations for matches where cursor is NOT inside
        for (const m of matches) {
            const cursorInside = cursorPos >= m.fullStart && cursorPos <= m.fullEnd;

            if (!cursorInside) {
                // Hide opening marker
                if (m.openStart >= 0 && m.openEnd > m.openStart) {
                    builder.add(m.openStart, m.openEnd, hiddenMark);
                }
                // Hide closing marker (if exists)
                if (m.closeStart >= 0 && m.closeEnd > m.closeStart) {
                    builder.add(m.closeStart, m.closeEnd, hiddenMark);
                }
            }
        }

        return builder.finish();
    }
}, {
    decorations: v => v.decorations
});

// Combined extension
export const hideMarkdown = [hideMarkdownTheme, hideMarkdownPlugin];
