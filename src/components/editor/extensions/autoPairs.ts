/**
 * Auto-Pairing Extension for CodeMirror 6
 * 
 * Automatically inserts closing pairs for:
 * - $ (math)
 * - ` (code)
 * - * (italic/bold)
 * - (, [, { (brackets)
 * 
 * Logic includes upgrades:
 * - *|* -> **|**
 * - $|$ -> $$|$$
 */

import { EditorView, keymap } from '@codemirror/view';
import { Prec } from '@codemirror/state';

// Pair mappings
const PAIRS: Record<string, string> = {
    '$': '$',
    '`': '`',
    '*': '*',
    '(': ')',
    '[': ']',
    '{': '}',
};

// Auto-insert pairs on typing
const insertPairs = EditorView.inputHandler.of((view, from, to, text) => {
    // Only handle single character inputs
    if (text.length !== 1) return false;

    const closingChar = PAIRS[text];
    if (!closingChar) return false;

    const doc = view.state.doc;
    const line = doc.lineAt(from);
    const lineText = line.text;
    const posInLine = from - line.from;

    // Special case: typing * after * should upgrade to **|**
    if (text === '*' && posInLine > 0) {
        const prevChar = lineText[posInLine - 1];
        const nextChar = lineText[posInLine];

        if (prevChar === '*' && nextChar === '*') {
            view.dispatch({
                changes: [
                    { from: from, to: from, insert: '*' },
                    { from: to, to: to, insert: '*' }
                ],
                selection: { anchor: from + 1 }
            });
            return true;
        }
    }

    // Special case: typing $ after $ should upgrade to $$|$$
    if (text === '$' && posInLine > 0) {
        const prevChar = lineText[posInLine - 1];
        const nextChar = lineText[posInLine];

        if (prevChar === '$' && nextChar === '$') {
            view.dispatch({
                changes: [
                    { from: from, to: from, insert: '$' },
                    { from: to, to: to, insert: '$' }
                ],
                selection: { anchor: from + 1 }
            });
            return true;
        }
    }

    // Check if next char is already the closing pair (skip if user is typing over it)
    const nextChar = from < doc.length ? doc.sliceString(from, from + 1) : '';
    if (nextChar === closingChar && text === closingChar) {
        // Just move cursor forward instead of inserting
        view.dispatch({
            selection: { anchor: from + 1 }
        });
        return true;
    }

    // Insert pair and position cursor in middle
    view.dispatch({
        changes: { from, to, insert: text + closingChar },
        selection: { anchor: from + 1 }
    });

    return true;
});

// Auto-delete empty pairs on backspace (Highest Priority)
const deletePairs = Prec.highest(keymap.of([{
    key: 'Backspace',
    run: (view) => {
        const { from } = view.state.selection.main;
        if (from === 0) return false;

        const doc = view.state.doc;
        const prevChar = doc.sliceString(from - 1, from);
        const nextChar = doc.sliceString(from, from + 1);

        // Check if we're between a pair
        const closingForPrev = PAIRS[prevChar];
        if (closingForPrev && nextChar === closingForPrev) {
            // Delete both characters
            view.dispatch({
                changes: { from: from - 1, to: from + 1 },
                selection: { anchor: from - 1 }
            });
            return true;
        }

        return false; // Let default backspace handle it
    }
}]));

export const autoPairs = [insertPairs, deletePairs];
