import React, { useMemo } from 'react';
import { LazyInlineMath } from './MathWrappers';

interface MarkdownRendererProps {
    content: string;
    className?: string;
}

// Simple parser to handle: Math ($), Code (`), Bold (**), Italic (*)
// Priority: Math > Code > Bold > Italic
export const MarkdownRenderer: React.FC<MarkdownRendererProps> = ({ content, className }) => {

    const renderedContent = useMemo(() => {
        if (!content) return <span className="opacity-0">Empty</span>; // Keep height?

        const elements: React.ReactNode[] = [];
        let lastIndex = 0;

        // 1. Split by MATH ($...$)
        const mathRegex = /\$([^\$]+)\$/g;
        let match;

        while ((match = mathRegex.exec(content)) !== null) {
            // Push text before math
            if (match.index > lastIndex) {
                elements.push(recursiveParseFormat(content.slice(lastIndex, match.index)));
            }

            // Push MATH
            elements.push(
                <span key={`math-${match.index}`} className="inline-block relative z-10 rounded select-none cursor-text align-middle" contentEditable={false}>
                    <LazyInlineMath math={match[1]} />
                </span>
            );

            lastIndex = match.index + match[0].length;
        }

        // Push remaining text
        if (lastIndex < content.length) {
            elements.push(recursiveParseFormat(content.slice(lastIndex)));
        }

        return elements.length > 0 ? elements : recursiveParseFormat(content);
    }, [content]);

    return (
        <div className={`whitespace-pre-wrap break-words min-h-[1.5em] ${className}`}>
            {renderedContent}
        </div>
    );
};

// Helper: Parse Bold, Italic, Code within non-math text segments
const recursiveParseFormat = (text: string): React.ReactNode => {
    // Tokenize by `code` first (highest priority after math)
    const codeRegex = /`([^`]+)`/g;
    const splitByCode = text.split(codeRegex);

    if (splitByCode.length > 1) {
        return splitByCode.map((part, i) => {
            // Even indices are text, Odd indices are code (captured group)
            if (i % 2 === 1) {
                return (
                    <code key={`code-${i}`} className="bg-zinc-800 text-purple-300 rounded px-1.5 py-0.5 font-mono text-[0.85em] align-middle">
                        {part}
                    </code>
                );
            }
            return parseStyles(part);
        });
    }

    return parseStyles(text);
};

// Parse Bold (**) and Italic (*)
const parseStyles = (text: string): React.ReactNode => {
    // Strategy: Split by **, then for text parts, split by *
    // Note: This regex splits and captures the content between **
    const splitBold = text.split(/\*\*(.*?)\*\*/g);

    return splitBold.map((boldPart, i) => {
        if (i % 2 === 1) {
            // This is Bold content
            return <strong key={`b-${i}`} className="font-bold text-zinc-100">{parseItalic(boldPart)}</strong>;
        }
        return parseItalic(boldPart);
    });
};

const parseItalic = (text: string): React.ReactNode => {
    // Note: The regex needs to be careful. Simple * matches.
    const splitItalic = text.split(/\*([^\*]+)\*/g);

    return splitItalic.map((italicPart, i) => {
        if (i % 2 === 1) {
            return <em key={`i-${i}`} className="italic text-zinc-300">{italicPart}</em>;
        }
        return <span key={`text-${i}`}>{italicPart}</span>;
    });
};
