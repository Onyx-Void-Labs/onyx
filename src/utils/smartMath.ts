import { MATH_SYMBOLS } from '../data/mathSymbols';

/**
 * Processes content for 'Smart Math' transformations.
 * Handles:
 * 1. Auto-Fractions: x/y -> \frac{x}{y}
 * 2. Auto-Symbols: alpha -> \alpha
 * 
 * @param content The current block content
 * @returns The transformed content (or original if no changes)
 */
export const processSmartInput = (content: string): string => {
    let finalContent = content;

    // 1. Auto-Fractions
    // We remove the '$' anchor to allow matching fractions anywhere (e.g. inside parens or while editing middle)
    // Regex Groups:
    // Group 1 (Numerator): 
    //   - (...)       : Parenthesized group (e.g. (x/y))
    //   - \\frac{...} : Existing fraction (e.g. \frac{1}{2})
    //   - [a-zA-Z0-9] : Simple alphanumeric variable/number
    // Group 2 (Denominator):
    //   - [a-zA-Z0-9] : Simple alphanumeric
    // Check for "Numerator"/"Denominator" pattern.
    // NOTE: This simpler regex replaces the FIRST occurrence. 
    // Since we run this on every keystroke, handling the "just typed" instance is usually sufficient.
    const fractionRegex = /(\([^)]+\)|\\frac\{[^}]+\}\{[^}]+\}|[a-zA-Z0-9]+)\/([a-zA-Z0-9]+)/;
    const fractionMatch = finalContent.match(fractionRegex);

    if (fractionMatch) {
        const [full, num, den] = fractionMatch;
        // Check if the match is "fresh" (i.e., we just typed the slash or part of denom)
        // Actually, if we remove '$', we might re-match things? 
        // No, because replacement converts to \frac{...}.
        // The regex `.../([a-zA-Z0-9]+)` expects a literal slash. 
        // \frac{...} does not contain a literal slash (unless inside content).
        // So safe to replace.

        finalContent = finalContent.replace(full, `\\frac{${num}}{${den}}`);
    }

    // 2. Auto-Symbols (alpha -> \alpha)
    for (const sym of MATH_SYMBOLS) {
        if (sym.trigger && finalContent.endsWith(sym.trigger)) {
            const triggerLen = sym.trigger.length;

            // Get character before the trigger
            const charBefore = finalContent[finalContent.length - triggerLen - 1];

            // CRITICAL: Prevent double conversion
            // If charBefore is '\', we're already at a LaTeX command - skip!
            if (charBefore === '\\') continue;

            // Also check if the content already ends with the command
            // This prevents re-processing on cursor re-entry
            if (finalContent.endsWith(sym.cmd)) continue;

            // Safe to replace
            finalContent = finalContent.slice(0, -triggerLen) + sym.cmd + " ";
            break;
        }
    }

    return finalContent;
};
