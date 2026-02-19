import { EditorView, ViewPlugin, ViewUpdate } from "@codemirror/view";
import { EditorState } from "@codemirror/state";

// Greek letters map
const GREEK_LETTERS = [
    "alpha", "beta", "gamma", "delta", "epsilon", "zeta", "eta", "theta", "iota", "kappa",
    "lambda", "mu", "nu", "xi", "omicron", "pi", "rho", "sigma", "tau", "upsilon",
    "phi", "chi", "psi", "omega", "Delta", "Gamma", "Theta", "Lambda", "Xi", "Pi",
    "Sigma", "Upsilon", "Phi", "Psi", "Omega"
];

// Helper to check if we are inside inline math ($...$) strictly on the same line
function isInMath(state: EditorState, pos: number): boolean {
    const line = state.doc.lineAt(pos);
    const lineText = line.text;
    const relPos = pos - line.from;
    const before = lineText.slice(0, relPos);
    // Count unescaped dollars
    let dollars = 0;
    for (let i = 0; i < before.length; i++) {
        if (before[i] === '$' && (i === 0 || before[i - 1] !== '\\')) {
            dollars++;
        }
    }
    // If odd, we are open
    return dollars % 2 !== 0;
}

export const mathAutoReplace = ViewPlugin.fromClass(class {
    constructor(readonly view: EditorView) { }

    update(update: ViewUpdate) {
        if (!update.docChanged || !update.selectionSet) return;

        // We only care about user input (transaction)
        const tr = update.transactions.find(t => t.isUserEvent('input'));
        if (!tr) return;

        const state = update.state;
        const main = state.selection.main;
        if (!main.empty) return;

        const pos = main.head;
        if (!isInMath(state, pos)) return;

        const line = state.doc.lineAt(pos);
        const lineText = line.text;
        const relPos = pos - line.from;

        // 1. Fraction Auto-Replace: "num/den" -> "\frac{num}{den}"
        // Trigger on typing '/'? Or after? The user said "2/5 to \frac{2}{5}".
        // Assuming they type '5' then maybe space or just immediate? 
        // Or maybe they type '2/5' and it triggers?
        // Let's trigger on 'Space' or any delimiter?
        // Actually, detecting '2/5' immediately is risky if they want '2/5'.
        // But in LaTeX, '2/5' is usually \frac.
        // Let's try to detect pattern "number/number" immediately after the second number is typed?
        // Or better: trigger when typing `/`. "2/" -> wait for denominator?
        // Let's trigger on SPACE or Close Brace '}'.

        // Let's look at the character just typed.
        // We can't easily know WHICH char was typed from ViewUpdate without inspecting changes.
        // But we can check the text BEFORE cursor.

        // Check for GREEK LETTERS
        // " alpha" -> " \alpha"
        for (const greek of GREEK_LETTERS) {
            const pattern = `${greek}`;
            // Check if we just finished typing this word
            if (lineText.slice(0, relPos).endsWith(pattern)) {
                // Ensure it's a whole word (preceded by space, start of line, or non-letter)
                const charBefore = lineText[relPos - pattern.length - 1];
                if (charBefore === undefined || /[^a-zA-Z]/.test(charBefore)) {
                    // Check if strictly inside math (already done)
                    // Check if ALREADY has backslash
                    if (charBefore === '\\') continue;

                    // REPLACE
                    const from = pos - pattern.length;
                    // We need to dispatch a change.
                    // TIMEOUT to avoid conflict with current update cycle
                    setTimeout(() => {
                        this.view.dispatch({
                            changes: { from, to: pos, insert: `\\${greek}` }
                        });
                    }, 0);
                    return;
                }
            }
        }

        // Check for FRACTIONS
        // Pattern: "digits/digits" followed by SPACE
        // We check if the last char typed was space.
        // But we don't know the char typed. We check the content.
        // If content ends with "NUM/DEN ", replace.
        const beforeCursor = lineText.slice(0, relPos);

        // Regex: /(\d+)\/(\d+)\s$/
        // Matches "1/2 "
        const fracMatch = beforeCursor.match(/(\d+)\/(\d+)\s$/);

        if (fracMatch) {
            const [full, num, den] = fracMatch;
            // full includes the space.
            // We want to replace "NUM/DEN " with "\frac{NUM}{DEN}" (no trailing space? or keep it?)
            // Usually we want to keep typing. 
            // Let's replace with "\frac{num}{den} " (keep space) or just cursor after?
            // If we remove space, user might type next word attached.
            // Let's insert "\frac{num}{den}" and let user type space again if needed?
            // Or better: The space was the trigger.
            // Let's replace "NUM/DEN " with "\frac{NUM}{DEN}".

            setTimeout(() => {
                this.view.dispatch({
                    changes: {
                        from: line.from + (relPos - full.length),
                        to: pos,
                        insert: `\\frac{${num}}{${den}}`
                    }
                });
            }, 0);
            return;
        }

    }
});
