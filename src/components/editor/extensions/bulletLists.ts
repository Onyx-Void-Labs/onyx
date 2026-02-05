/**
 * Bullet Lists Extension for CodeMirror 6
 * 
 * Styles markdown list dashes (-) as proper bullets:
 * - Level 1: • (Big Solid Dot)
 * - Level 2: ◦ (Hollow Circle)
 * - Level 3: ▪ (Small Solid Square)
 */

import { EditorView, Decoration, DecorationSet, ViewPlugin, ViewUpdate, WidgetType } from '@codemirror/view';
import { RangeSetBuilder } from '@codemirror/state';

// CSS for the bullets
const bulletTheme = EditorView.theme({
    '.cm-bullet': {
        display: 'inline-flex',
        alignItems: 'center',
        justifyContent: 'center',
        width: '20px',
        height: '100%',
        marginRight: '2px',
        fontWeight: 'bold',
        color: '#d4d4d8',
        verticalAlign: 'text-top',
        lineHeight: '1',
    },
    '.cm-bullet-1': { fontSize: '1.4em', transform: 'translateY(-2px)' },
    '.cm-bullet-2': { fontSize: '1.2em', fontWeight: 'bold', transform: 'translateY(3px)' }, // Lowered more
    '.cm-bullet-3': { fontSize: '0.9em', transform: 'translateY(5px)' }, // Lowered more
});

// Widgets for replacement
class BulletWidget extends WidgetType {
    constructor(readonly level: number) { super(); }

    toDOM() {
        const span = document.createElement("span");
        const mod = this.level % 3;
        span.className = `cm-bullet cm-bullet-${mod + 1}`;

        // Explicit content for accessibility/copy
        if (mod === 0) span.textContent = "•";
        else if (mod === 1) span.textContent = "◦";
        else span.textContent = "▪";

        return span;
    }
}

const bulletPlugin = ViewPlugin.fromClass(class {
    decorations: DecorationSet;

    constructor(view: EditorView) {
        this.decorations = this.buildDecorations(view);
    }

    update(update: ViewUpdate) {
        if (update.docChanged || update.viewportChanged) {
            this.decorations = this.buildDecorations(update.view);
        }
    }

    buildDecorations(view: EditorView): DecorationSet {
        const builder = new RangeSetBuilder<Decoration>();
        const doc = view.state.doc;

        for (let i = 1; i <= doc.lines; i++) {
            const line = doc.line(i);
            const text = line.text;

            // Match: optional indent + dash + space
            const match = text.match(/^(\s*)-\s/);

            if (match) {
                const indent = match[1].length;
                const dashStart = line.from + indent;
                const dashEnd = dashStart + 1; // Length of "-"

                const level = Math.floor(indent / 2);

                builder.add(dashStart, dashEnd, Decoration.replace({
                    widget: new BulletWidget(level)
                }));
            }
        }

        return builder.finish();
    }
}, {
    decorations: v => v.decorations
});

export const bulletLists = [bulletTheme, bulletPlugin];
