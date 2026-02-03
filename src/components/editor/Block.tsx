import React, { useRef, useEffect, useLayoutEffect } from 'react';
import { LazyBlockMath, LazyInlineMath } from './MathWrappers';
import { Block, BlockType } from '../../types';
import { MarkdownRenderer } from './MarkdownRenderer';

interface BlockProps {
    block: Block;
    isFocused: boolean;
    index: number;
    updateBlock: (id: string, content: string, type?: BlockType, pushHistory?: boolean, cursorPos?: number) => void;
    onFocus: (id: string) => void;
    onKeyDown: (e: React.KeyboardEvent, id: string, index: number) => void;
    onPaste: (e: React.ClipboardEvent, id: string) => void;
    zoom: number;
    onResize?: (id: string, height: number) => void;
}

export const EditorBlock = React.memo(({ block, isFocused, index, updateBlock, onFocus, onKeyDown, onPaste, zoom, onResize }: BlockProps) => {
    const textareaRef = useRef<HTMLTextAreaElement>(null);
    const containerRef = useRef<HTMLDivElement>(null);
    const wasClicked = useRef(false);
    const [mathPreviewPos, setMathPreviewPos] = React.useState<{ x: number, y: number, math: string } | null>(null);

    useLayoutEffect(() => {
        if (textareaRef.current) {
            textareaRef.current.style.height = 'auto';
            textareaRef.current.style.height = textareaRef.current.scrollHeight + 'px';
        }
        if (containerRef.current && onResize) {
            onResize(block.id, containerRef.current.getBoundingClientRect().height);
        }

        // Check for active math segment under cursor if focused
        if (isFocused && textareaRef.current && block.content.includes('$')) {
            const el = textareaRef.current;
            const caret = el.selectionStart;
            const text = el.value;

            // Find if cursor is inside a math pair $...$
            // Simple heuristic for now: check segments
            // Or better: regex scan and check if caret is inside match range
            const mathRegex = /\$([^\$]+)\$/g;
            let match;
            let activeMath: string | null = null;
            let activeStart = 0;

            while ((match = mathRegex.exec(text)) !== null) {
                const start = match.index;
                const end = start + match[0].length;

                // If caret is inside or at edges of a math block
                if (caret >= start && caret <= end) {
                    activeMath = match[1];
                    activeStart = start;
                    break;
                }
            }

            if (activeMath) {
                // Calculate position!
                // We create a temp span to measure width up to start of math
                const span = document.createElement('span');
                span.style.font = window.getComputedStyle(el).font;
                span.style.visibility = 'hidden';
                span.style.whiteSpace = 'pre-wrap';
                // Text before the math starts + maybe indentation?
                span.textContent = text.substring(0, activeStart);
                document.body.appendChild(span);
                const width = span.offsetWidth;
                document.body.removeChild(span);

                // Rough estimate for Y (one line height down?)
                // Since rows=1 usually, it's just below.
                const lineHeight = parseFloat(window.getComputedStyle(el).lineHeight || '24');

                // Adjust for wrapping? If wrapped, width is modulo container width?
                // Complex for wrapped lines. simpler: assume 1 line for now or simple wrapping.
                // For true caret pos we need gettingCoordinates library or mirror div.
                // Let's stick to X offset for now.

                setMathPreviewPos({ x: width, y: lineHeight, math: activeMath });
            } else {
                setMathPreviewPos(null);
            }
        } else {
            setMathPreviewPos(null);
        }

    }, [block.content, zoom, isFocused, block.type]); // Needs to re-run on selection change? selection change doesn't trigger effect...

    // We need to listen to selection change more actively?
    // onKeyUp / onClick triggers re-render? No.
    // We can add onSelect handler to textarea

    const checkMathPreview = () => {
        // Re-run the logic above. Refactor into helper? 
        // For optimization, let's just trigger a lightweight update or duplicate logic lightly.
        if (textareaRef.current && block.content.includes('$')) {
            const el = textareaRef.current;
            const caret = el.selectionStart;
            const text = el.value;
            const mathRegex = /\$([^\$]+)\$/g;
            let match;
            let found = null;
            while ((match = mathRegex.exec(text)) !== null) {
                if (caret >= match.index && caret <= match.index + match[0].length) {
                    found = { math: match[1], index: match.index };
                    break;
                }
            }

            if (found) {
                const span = document.createElement('span');
                span.style.font = window.getComputedStyle(el).font;
                span.style.whiteSpace = 'pre-wrap';
                span.textContent = text.substring(0, found.index);
                document.body.appendChild(span);
                const width = span.offsetWidth;
                document.body.removeChild(span);
                // Clamp width to container?
                setMathPreviewPos({ x: Math.min(width, el.offsetWidth - 200), y: 24, math: found.math });
            } else setMathPreviewPos(null);
        } else setMathPreviewPos(null);
    };


    useEffect(() => {
        if (isFocused && textareaRef.current) {
            textareaRef.current.focus();
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
                onMouseDown={() => { wasClicked.current = true; }}
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
                            onChange={(e) => updateBlock(block.id, e.target.value, undefined, undefined, e.target.selectionStart)}
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
                <div className="relative w-full">
                    <textarea
                        ref={textareaRef}
                        data-block-id={block.id}
                        style={{ fontSize, lineHeight, fontWeight }}
                        value={block.content}
                        onChange={(e) => { updateBlock(block.id, e.target.value, undefined, undefined, e.target.selectionStart); checkMathPreview(); }}
                        onKeyDown={(e) => { onKeyDown(e, block.id, index); setTimeout(checkMathPreview, 10); }}
                        onClick={checkMathPreview}
                        onPaste={(e) => onPaste(e, block.id)}
                        onSelect={checkMathPreview} // Trigger on any cursor move
                        className={`bg-transparent outline-none w-full resize-none ${textClasses} overflow-hidden whitespace-pre-wrap break-words`}
                        placeholder={block.type === 'p' ? "Type '/' for commands..." : `Heading ${block.type.replace('h', '')}`}
                        rows={1}
                    />

                    {/* Positioned Live Preview */}
                    {mathPreviewPos && (
                        <div
                            className="absolute z-50 pointer-events-none"
                            style={{
                                left: `${mathPreviewPos.x}px`,
                                top: '100%',
                                marginTop: '4px'
                            }}
                        >
                            <div className="bg-zinc-900/90 border border-zinc-700/50 p-2 rounded-lg shadow-2xl backdrop-blur-md">
                                <div className="text-lg text-zinc-100">
                                    <LazyInlineMath math={mathPreviewPos.math} />
                                </div>
                            </div>
                        </div>
                    )}
                </div>
            ) : (
                <div
                    onMouseDown={() => { wasClicked.current = true; }}
                    onClick={(e) => { e.stopPropagation(); onFocus(block.id); }}
                    style={{ fontSize, lineHeight, fontWeight }}
                    className={`w-full ${textClasses} whitespace-pre-wrap break-words cursor-text min-h-[1.5em]`}
                >
                    <MarkdownRenderer content={block.content} />
                </div>
            )}
        </div>
    );
});