import React, { useRef, useEffect, useLayoutEffect } from 'react';
import { LazyBlockMath, LazyInlineMath } from './MathWrappers';
import { Block, BlockType } from '../../types';

// Matching user's simple RenderedParagraph logic but keeping markdown support for "Phase 4"
const RenderedContent = ({ text }: { text: string }) => {
    if (!text) return <span className="opacity-0">Empty</span>;

    const parts = text.split('$$');
    return (
        <>
            {parts.map((part, i) => {
                if (i % 2 === 1) return <span key={i} className="text-purple-400 px-1 inline-block"><LazyInlineMath math={part} /></span>;
                return <span key={i}>{part}</span>;
            })}
        </>
    );
};

interface BlockProps {
    block: Block;
    isFocused: boolean;
    index: number;
    updateBlock: (id: string, content: string, type?: BlockType, pushHistory?: boolean) => void;
    onFocus: (id: string) => void;
    onKeyDown: (e: React.KeyboardEvent, id: string, index: number) => void;
    onPaste: (e: React.ClipboardEvent, id: string) => void;
    zoom: number;
    onResize?: (id: string, height: number) => void;
}

export const EditorBlock = React.memo(({ block, isFocused, index, updateBlock, onFocus, onKeyDown, onPaste, zoom, onResize }: BlockProps) => {
    const textareaRef = useRef<HTMLTextAreaElement>(null);
    const containerRef = useRef<HTMLDivElement>(null);
    const wasClicked = useRef(false); // Track if focus came from a click

    useLayoutEffect(() => {
        if (textareaRef.current) {
            textareaRef.current.style.height = 'auto';
            textareaRef.current.style.height = textareaRef.current.scrollHeight + 'px';
        }
        // Report height for virtualization
        if (containerRef.current && onResize) {
            // Add a small buffer or ensure exact measurement
            onResize(block.id, containerRef.current.getBoundingClientRect().height);
        }
    }, [block.content, zoom, isFocused, block.type]);

    useEffect(() => {
        if (isFocused && textareaRef.current) {
            textareaRef.current.focus();
            // REMOVED: Aggressive cursor reset. Handled by Editor.tsx now.
            wasClicked.current = false;
        }
    }, [isFocused]);

    const getStyles = () => {
        const base = zoom;
        switch (block.type) {
            case 'h1': return { fontSize: `${base * 3.75}rem`, lineHeight: 1.1, fontWeight: 900, marginBottom: '0.5em', marginTop: '0.5em' };
            case 'h2': return { fontSize: `${base * 3}rem`, lineHeight: 1.2, fontWeight: 800, marginBottom: '0.5em', marginTop: '0.5em' };
            case 'h3': return { fontSize: `${base * 2.25}rem`, lineHeight: 1.3, fontWeight: 700, marginBottom: '0.5em', marginTop: '0.5em' };
            default: return { fontSize: `${base * 1.25}rem`, lineHeight: 1.6, fontWeight: 500 };
        }
    };

    if (block.type === 'math') {
        return (
            <div
                ref={containerRef}
                className="group relative w-full mb-6 px-8"
                onMouseDown={() => { wasClicked.current = true; }} // Logic for Math click
                onClick={(e) => { e.stopPropagation(); onFocus(block.id); }}
            >
                <div className={`transition-all p-6 rounded-xl border ${isFocused ? 'bg-zinc-900 border-purple-500/50 shadow-[0_0_20px_rgba(168,85,247,0.1)]' : 'bg-transparent border-transparent hover:bg-zinc-900/20'}`}>
                    {isFocused && (
                        <textarea
                            ref={textareaRef}
                            data-block-id={block.id}
                            autoFocus
                            className="w-full bg-transparent font-mono text-sm text-purple-400 outline-none resize-none mb-4"
                            value={block.content}
                            onChange={(e) => updateBlock(block.id, e.target.value)}
                            onKeyDown={(e) => onKeyDown(e, block.id, index)}
                            placeholder="LaTeX Syntax... (e.g. 1/2 + space)"
                            rows={1}
                            onPaste={(e) => onPaste(e, block.id)}
                        />
                    )}
                    <div className="flex justify-center overflow-x-auto py-2 custom-scrollbar">
                        <div className="text-3xl text-zinc-200">
                            <LazyBlockMath math={block.content || "\\text{Empty Equation}"} />
                        </div>
                    </div>
                </div>
            </div>
        );
    }

    const { fontSize, lineHeight, fontWeight } = getStyles();
    const textClasses = block.type.startsWith('h') ? 'text-zinc-100 tracking-tighter' : 'text-zinc-400';

    return (
        <div ref={containerRef} className="group relative w-full px-8 py-1">
            <div className="absolute left-2 top-2.5 bottom-2.5 w-1 rounded-full transition-all duration-300 bg-zinc-800 opacity-20 group-hover:opacity-100" />

            {isFocused ? (
                <textarea
                    ref={textareaRef}
                    data-block-id={block.id}
                    style={{ fontSize, lineHeight, fontWeight }}
                    value={block.content}
                    onChange={(e) => updateBlock(block.id, e.target.value)}
                    onKeyDown={(e) => onKeyDown(e, block.id, index)}
                    onPaste={(e) => onPaste(e, block.id)}
                    className={`bg-transparent outline-none w-full resize-none ${textClasses} overflow-hidden whitespace-pre-wrap break-words`}
                    placeholder={block.type === 'p' ? "Type '/' for commands..." : `Heading ${block.type.replace('h', '')}`}
                    rows={1}
                />
            ) : (
                <div
                    onMouseDown={() => { wasClicked.current = true; }} // Set flag BEFORE focus triggers
                    onClick={(e) => { e.stopPropagation(); onFocus(block.id); }}
                    style={{ fontSize, lineHeight, fontWeight }}
                    className={`w-full ${textClasses} whitespace-pre-wrap break-words cursor-text min-h-[1.5em]`}
                >
                    <RenderedContent text={block.content} />
                </div>
            )}
        </div>
    );
});