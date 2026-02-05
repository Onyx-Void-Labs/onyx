import { Extension } from "@codemirror/state";
import { EditorView, KeyBinding, keymap } from "@codemirror/view";
import { MATH_SYMBOLS } from "../../../data/mathSymbols";

// Build Snippet Map
const snippets: Record<string, string> = {};
MATH_SYMBOLS.forEach(sym => {
    if (sym.trigger) snippets[sym.trigger] = sym.cmd;
});

// Manual overrides
snippets["inf"] = "\\infty";
snippets["sqrt"] = "\\sqrt";
snippets["~="] = "\\approx";
snippets["!="] = "\\neq";
snippets["<="] = "\\leq";
snippets[">="] = "\\geq";
snippets["->"] = "\\to";

// --- LAZY FRACTION HELPER ---
function checkLazyFraction(view: EditorView, from: number, triggerText: string): boolean {
    const textBefore = view.state.sliceDoc(0, from);

    // Robust Backwards Scanner for "Atom"
    // Supports chaining adjacent groups: {a}{b}...
    const scanAtom = (endIdx: number): { start: number, content: string } | null => {
        let i = endIdx - 1;
        // Skip trailing whitespace
        while (i >= 0 && /\s/.test(textBefore[i])) i--;
        if (i < 0) return null;

        const contentEnd = i + 1;
        let start = i;
        let chainStart = i;
        let hasScannedGroup = false;

        // Loop to chain adjacent groups backwards
        while (i >= 0) {
            const char = textBefore[i];
            if (char === '}' || char === ')') {
                // Scan balanced group
                let balance = 0;
                // Save position before scanning this group
                let tempI = i;
                while (tempI >= 0) {
                    const c = textBefore[tempI];
                    if (c === '}' || c === ')') balance++;
                    else if (c === '{' || c === '(') balance--;

                    if (balance === 0) {
                        // Successfully scanned one group
                        hasScannedGroup = true;
                        chainStart = tempI; // Valid start of this group
                        i = tempI - 1; // Move index before this group

                        // Skip spaces between groups to check strictly for adjacent groups? 
                        // LaTeX args usually consume spaces: \frac {a} {b}. So yes, skip spaces.
                        while (i >= 0 && /\s/.test(textBefore[i])) i--;

                        break; // Continue outer loop to find MORE groups
                    }
                    tempI--;
                }
                if (balance !== 0) {
                    break;
                }
            } else {
                // Not a group char. Stop chaining.
                break;
            }
        }

        if (!hasScannedGroup) {
            // Simple Word scanning (fallback)
            i = start; // Reset to original end position

            // Allow word chars + backslash + dots
            while (i >= 0 && /[a-zA-Z0-9.\\]/.test(textBefore[i])) i--;

            // If stopped at `\`, include it.
            if (i >= 0 && textBefore[i] === '\\') i--;

            start = i + 1;
        } else {
            // Groups found. Check for preceding command.
            // i is currently at char before the chain (and before spaces).
            // BUT we might have spaces between command and group: \frac {a}

            start = chainStart;

            // SKIP SPACES BEFORE CHAIN
            let k = i;
            while (k >= 0 && /\s/.test(textBefore[k])) k--; // Skip spaces

            let j = k;
            while (j >= 0 && /[a-zA-Z]/.test(textBefore[j])) j--;
            if (j >= 0 && textBefore[j] === '\\') {
                start = j;
            }
        }

        if (start >= contentEnd) return null; // Should not happen but safety
        return { start: start, content: textBefore.slice(start, contentEnd) };
    };

    // Scan backwards for a chain of atoms separated by '/'
    // e.g. "1 / 2 / 3" -> atoms: [3, 2, 1]
    const atoms: { start: number, content: string }[] = [];
    let currentEnd = from;

    while (true) {
        const atom = scanAtom(currentEnd);
        if (!atom) break;
        atoms.push(atom);

        // Check for preceding slash
        let slashIdx = atom.start - 1;
        while (slashIdx >= 0 && /\s/.test(textBefore[slashIdx])) slashIdx--;

        if (slashIdx >= 0 && textBefore[slashIdx] === '/') {
            // Found slash, continue scanning previous atom
            currentEnd = slashIdx;
        } else {
            // No slash, stop chain
            break;
        }
    }

    if (atoms.length >= 2) {
        // We have a chain! Construct nested fraction.
        // Atoms are in reverse order: [3, 2, 1] for "1/2/3"
        // Real order: 1, 2, 3
        const reversedAtoms = atoms.reverse();

        // Start with the first atom
        let currentString = reversedAtoms[0].content;

        // Iteratively wrap with next atom
        for (let k = 1; k < reversedAtoms.length; k++) {
            const nextAtom = reversedAtoms[k].content;
            currentString = `\\frac{${currentString}}{${nextAtom}}`;
        }

        const startPos = reversedAtoms[0].start;
        const replacement = currentString;

        view.dispatch({
            changes: {
                from: startPos,
                to: from,
                insert: replacement + triggerText
            },
            selection: { anchor: startPos + replacement.length + triggerText.length }
        });
        return true;
    }

    return false;
}

const snippetHandler = EditorView.inputHandler.of((view, from, _to, text) => {
    const textBefore = view.state.sliceDoc(0, from);
    const dollars = (textBefore.match(/\$/g) || []).length;
    const isMath = dollars % 2 !== 0;

    // Check Lazy Fraction on Separators
    if (isMath && (text === " " || text === ")" || text === "}" || /[+\-=*]/.test(text))) {
        if (checkLazyFraction(view, from, text)) return true;
    }

    // Check Snippets
    const lookBack = textBefore.slice(-10);
    const candidate = lookBack + text;

    for (const key of Object.keys(snippets)) {
        if (candidate.endsWith(key)) {
            const matchStart = from - (key.length - text.length);

            // Recursion Prevention
            if (matchStart > 0) {
                const charBefore = textBefore.slice(matchStart - 1, matchStart);
                if (charBefore === "\\") return false;
            }

            const replacement = snippets[key];
            const hasArgs = replacement.endsWith("{}");
            const insertText = hasArgs ? replacement : replacement + " ";

            view.dispatch({
                changes: { from: matchStart, to: from, insert: insertText },
                selection: { anchor: matchStart + insertText.length - (hasArgs ? 1 : 0) }
            });
            return true;
        }
    }

    return false;
});

export const snippetKeymap: KeyBinding[] = [
    {
        key: "Enter",
        run: (view: EditorView) => {
            const from = view.state.selection.main.head;
            const textBefore = view.state.sliceDoc(0, from);
            const dollars = (textBefore.match(/\$/g) || []).length;
            const isMath = dollars % 2 !== 0;

            if (isMath) {
                if (checkLazyFraction(view, from, "")) return true;
            }
            return false; // Propagate Enter
        }
    }
];

export const mathSnippets = [snippetHandler, keymap.of(snippetKeymap)];
