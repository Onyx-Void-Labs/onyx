import React, { useState, useEffect, useRef, useMemo, useLayoutEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { EditorBlock } from './Block';
import { BlockType } from '../../types';
import { MATH_SYMBOLS, MathSymbol } from '../../data/mathSymbols';
import { useEditor } from '../../hooks/useEditor';
import { LazyInlineMath, preloadMath } from './MathWrappers';
import { processSmartInput } from '../../utils/smartMath';

const COMMAND_OPTIONS: { type: BlockType; label: string; indicator: string }[] = [
    { type: 'h1', label: 'Heading 1', indicator: 'H1' },
    { type: 'h2', label: 'Heading 2', indicator: 'H2' },
    { type: 'h3', label: 'Heading 3', indicator: 'H3' },
    { type: 'p', label: 'Paragraph', indicator: 'P' },
    { type: 'code', label: 'Code Block', indicator: '<>' },
    { type: 'math', label: 'Math Block', indicator: 'Σ' },
];

export default function Editor({ activeNoteId, onSave }: { activeNoteId: number | null; onSave: () => void; }) {
    const {
        blocks, setBlocks, updateBlock, addBlock,
        splitBlock, mergeBlock, undo, redo, handlePaste
    } = useEditor([]);

    const [title, setTitle] = useState("");
    const [focusedId, setFocusedId] = useState<string | null>(null);
    const [zoom, setZoom] = useState(1.0);
    const [isSaving, setIsSaving] = useState(false);
    const loadedRef = useRef<string | null>(null);
    const menuScrollRef = useRef<HTMLDivElement>(null);
    const mathMenuScrollRef = useRef<HTMLDivElement>(null);
    const titleRef = useRef<HTMLInputElement>(null);
    const pendingCursor = useRef<{ id: string; pos: number } | null>(null);
    const blocksRef = useRef<any[]>(blocks);
    const titleStateRef = useRef<string>(title);
    const scrollContainerRef = useRef<HTMLDivElement>(null);
    const loadedNoteIdRef = useRef<number | null>(null);

    // Keep refs in sync
    useEffect(() => { blocksRef.current = blocks; }, [blocks]);
    useEffect(() => { titleStateRef.current = title || ""; }, [title]);

    const [commandMenu, setCommandMenu] = useState<{
        isOpen: boolean; x: number; y: number; blockId: string | null; selectedIndex: number; filterText: string; direction: 'up' | 'down'; triggerIdx: number;
    } | null>(null);

    const [mathMenu, setMathMenu] = useState<{
        isOpen: boolean; x: number; y: number; blockId: string | null; selectedIndex: number; filterText: string; direction: 'up' | 'down'; triggerIdx: number;
    } | null>(null);

    // --- ZOOM LISTENER ---
    useEffect(() => {
        const handleWheel = (e: WheelEvent) => {
            if (e.ctrlKey) {
                e.preventDefault();
                setZoom(prev => Math.min(Math.max(prev - e.deltaY * 0.001, 0.5), 2.0));
            }
        };
        window.addEventListener('wheel', handleWheel, { passive: false });
        return () => window.removeEventListener('wheel', handleWheel);
    }, []);

    // --- CLICK OUTSIDE MENUS ---
    useEffect(() => {
        const handleClickOutside = (e: MouseEvent) => {
            const target = e.target as Node;
            if (commandMenu?.isOpen && !document.getElementById('onyx-context-menu')?.contains(target)) setCommandMenu(null);
            if (mathMenu?.isOpen && !document.getElementById('onyx-math-menu')?.contains(target)) setMathMenu(null);
        };
        window.addEventListener('mousedown', handleClickOutside);
        return () => window.removeEventListener('mousedown', handleClickOutside);
    }, [commandMenu, mathMenu]);

    // --- SCROLL TO SELECTED ITEM ---
    useEffect(() => {
        if (commandMenu?.isOpen && menuScrollRef.current) {
            const el = menuScrollRef.current.children[commandMenu.selectedIndex] as HTMLElement;
            el?.scrollIntoView({ block: 'nearest' });
        }
        if (mathMenu?.isOpen && mathMenuScrollRef.current) {
            const el = mathMenuScrollRef.current.children[mathMenu.selectedIndex] as HTMLElement;
            el?.scrollIntoView({ block: 'nearest' });
        }
    }, [commandMenu?.selectedIndex, mathMenu?.selectedIndex, commandMenu?.isOpen, mathMenu?.isOpen]);

    // --- LOAD NOTE ---
    // --- LOAD NOTE ---
    useEffect(() => {
        let isCancelled = false;
        if (!activeNoteId) {
            setTitle("");
            setBlocks([]);
            loadedNoteIdRef.current = null;
            return;
        }

        invoke<any>("get_note_content", { id: activeNoteId }).then(data => {
            if (isCancelled) return;
            if (data) {
                const parsed = data.content ? JSON.parse(data.content) : [{ id: crypto.randomUUID(), type: "p", content: "" }];
                setTitle(data.title || ""); // Keep empty for new notes
                setBlocks(parsed);
                loadedRef.current = JSON.stringify({ t: data.title || "", c: parsed });
                loadedNoteIdRef.current = activeNoteId;

                // Auto-focus title for new notes (empty title)
                if (!data.title) {
                    setTimeout(() => titleRef.current?.focus(), 50);
                }
            }
        });

        return () => { isCancelled = true; };
    }, [activeNoteId, setBlocks]);

    // --- AUTO SAVE ---
    useEffect(() => {
        if (!activeNoteId) return;

        // CRITICAL: Don't save if we haven't loaded the data for this note ID yet
        if (loadedNoteIdRef.current !== activeNoteId) return;

        // Capture current state for this effect cycle
        const noteIdToSave = activeNoteId;
        const titleToSave = titleStateRef.current?.trim() || "Untitled";
        const blocksToSave = [...blocksRef.current];
        const current = JSON.stringify({ t: titleStateRef.current, c: blocksRef.current });

        if (current === loadedRef.current) return;
        setIsSaving(true);

        const t = setTimeout(async () => {
            await invoke("update_note", { id: noteIdToSave, title: titleToSave, content: JSON.stringify(blocksToSave) });
            loadedRef.current = current;
            setIsSaving(false);
            onSave();
        }, 400);

        return () => {
            clearTimeout(t);
            // Instant save on unmount/switch - use captured values, not current refs
            const finalCurrent = JSON.stringify({ t: titleStateRef.current, c: blocksToSave });
            if (finalCurrent !== loadedRef.current) {
                invoke("update_note", { id: noteIdToSave, title: titleToSave, content: JSON.stringify(blocksToSave) })
                    .then(() => onSave());
            }
        };
    }, [blocks, title, activeNoteId, onSave]);

    // --- CURSOR RESTORATION (SYNCHRONOUS) ---
    useLayoutEffect(() => {
        if (pendingCursor.current) {
            const { id, pos } = pendingCursor.current;
            requestAnimationFrame(() => {
                const el = document.querySelector(`textarea[data-block-id="${id}"]`) as HTMLTextAreaElement;
                if (el) {
                    el.focus();
                    el.setSelectionRange(pos, pos);
                }
                pendingCursor.current = null;
            });
        }
    }, [blocks, focusedId]);

    // --- MENU FILTERING ---
    const filteredCommands = useMemo(() => COMMAND_OPTIONS.filter(c => c.label.toLowerCase().includes(commandMenu?.filterText.toLowerCase() || "")), [commandMenu?.filterText]);
    const filteredMath = useMemo(() => {
        if (!mathMenu) return [];
        const filter = mathMenu.filterText.toLowerCase();
        return MATH_SYMBOLS.filter(s => s.name.toLowerCase().includes(filter) || s.cmd.toLowerCase().includes(filter) || s.keywords.includes(filter)).slice(0, 50);
    }, [mathMenu]);


    // --- MENU POSITIONING HELPER ---
    const getMenuPosition = (rect: DOMRect, height: number = 300): { y: number; direction: 'up' | 'down' } => {
        const spaceBelow = window.innerHeight - rect.bottom;
        const direction = spaceBelow < height ? 'up' : 'down';
        const y = direction === 'down' ? rect.bottom + 10 : rect.top - 10;
        return { y, direction };
    };

    // --- ACTIONS ---
    const transformBlock = (type: BlockType) => {
        if (!commandMenu?.blockId) return;
        const block = blocks.find(b => b.id === commandMenu.blockId);
        if (!block) return;

        if (type === 'math') {
            preloadMath();
            updateBlock(commandMenu.blockId, "", "math", true);
            setCommandMenu(null);
            setFocusedId(commandMenu.blockId);
            return;
        }

        const cleanContent = block.content.slice(0, commandMenu.triggerIdx);
        updateBlock(commandMenu.blockId, cleanContent, type, true);
        setCommandMenu(null);
        setFocusedId(commandMenu.blockId);
    };

    const insertMathSymbol = (symbol: MathSymbol) => {
        if (!mathMenu?.blockId) return;
        const block = blocks.find(b => b.id === mathMenu.blockId);
        if (!block) return;
        const lastBackslash = block.content.lastIndexOf('\\');
        const newContent = block.content.substring(0, lastBackslash) + symbol.cmd + " ";
        updateBlock(mathMenu.blockId, newContent);
        setMathMenu(null);
        setFocusedId(mathMenu.blockId);
    };

    // --- BLOCK UPDATE HANDLER ---
    const handleBlockUpdate = (id: string, content: string, type?: BlockType, pushHistory?: boolean) => {
        let finalContent = content;
        const block = blocks.find(b => b.id === id);

        if (block && block.type === 'math' && (!type || type === 'math')) {
            finalContent = processSmartInput(content);
        }

        updateBlock(id, finalContent, type, pushHistory);

        if (!block) return;

        // MENU CLOSING LOGIC
        if (mathMenu && mathMenu.blockId === id) {
            if (!content.includes('\\', mathMenu.triggerIdx)) {
                setMathMenu(null);
            }
        }
        if (commandMenu && commandMenu.blockId === id) {
            if (!content.includes('/', commandMenu.triggerIdx)) {
                setCommandMenu(null);
            }
        }

        // COMMAND TRIGGER (/)
        if (content.endsWith('/') && !commandMenu && block.type !== 'math') {
            const el = document.activeElement;
            const rect = el?.getBoundingClientRect();
            if (rect) {
                const { y, direction } = getMenuPosition(rect);
                setCommandMenu({ isOpen: true, x: rect.left, y, blockId: id, selectedIndex: 0, filterText: "", direction, triggerIdx: content.length - 1 });
                setMathMenu(null);
            }
        }
        // MATH TRIGGER (\)
        else if (content.endsWith('\\') && !mathMenu && block.type === 'math') {
            const el = document.activeElement;
            const rect = el?.getBoundingClientRect();
            if (rect) {
                const { y, direction } = getMenuPosition(rect);
                setMathMenu({ isOpen: true, x: rect.left, y, blockId: id, selectedIndex: 0, filterText: "", direction, triggerIdx: content.length - 1 });
                setCommandMenu(null);
            }
        }
        // FILTER MENUS
        else if (commandMenu && commandMenu.blockId === id) {
            const newFilter = content.slice(commandMenu.triggerIdx + 1);
            if (newFilter.includes(" ")) setCommandMenu(null);
            else setCommandMenu(prev => prev ? { ...prev, filterText: newFilter } : null);
        }
        else if (mathMenu && mathMenu.blockId === id) {
            const newFilter = content.slice(mathMenu.triggerIdx + 1);
            if (newFilter.includes(" ")) setMathMenu(null);
            else setMathMenu(prev => prev ? { ...prev, filterText: newFilter } : null);
        }

        // AUTO-TRANSFORMS
        if (content.endsWith(' ')) {
            if (content === '# ') { updateBlock(id, '', 'h1', true); setFocusedId(id); }
            if (content === '## ') { updateBlock(id, '', 'h2', true); setFocusedId(id); }
            if (content === '### ') { updateBlock(id, '', 'h3', true); setFocusedId(id); }
            if (content === '$$ ') { preloadMath(); updateBlock(id, '', 'math', true); setFocusedId(id); }
        }
        // Preload math early when user types $$
        if (content === '$$') preloadMath();
    };

    const handleKeyDown = (e: React.KeyboardEvent, id: string, index: number) => {
        // Auto-Brackets for Math
        if (blocks[index].type === 'math' && e.key === '(') {
            e.preventDefault();
            const target = e.currentTarget as HTMLTextAreaElement;
            const start = target.selectionStart;
            const val = target.value;
            const newVal = val.slice(0, start) + "()" + val.slice(start);
            const processed = processSmartInput(newVal);
            updateBlock(id, processed);
            setTimeout(() => {
                target.setSelectionRange(start + 1, start + 1);
            }, 0);
            return;
        }

        // MENU NAVIGATION
        if (commandMenu?.isOpen) {
            if (e.key === "ArrowDown") { e.preventDefault(); setCommandMenu({ ...commandMenu, selectedIndex: (commandMenu.selectedIndex + 1) % filteredCommands.length }); return; }
            if (e.key === "ArrowUp") { e.preventDefault(); setCommandMenu({ ...commandMenu, selectedIndex: (commandMenu.selectedIndex - 1 + filteredCommands.length) % filteredCommands.length }); return; }
            if (e.key === "Enter") { e.preventDefault(); const selected = filteredCommands[commandMenu.selectedIndex]; if (selected) transformBlock(selected.type); return; }
            if (e.key === "Escape") { setCommandMenu(null); return; }
        }
        if (mathMenu?.isOpen) {
            if (e.key === "ArrowDown") { e.preventDefault(); setMathMenu({ ...mathMenu, selectedIndex: (mathMenu.selectedIndex + 1) % filteredMath.length }); return; }
            if (e.key === "ArrowUp") { e.preventDefault(); setMathMenu({ ...mathMenu, selectedIndex: (mathMenu.selectedIndex - 1 + filteredMath.length) % filteredMath.length }); return; }
            if (e.key === "Enter" || e.key === "Tab") { e.preventDefault(); const selected = filteredMath[mathMenu.selectedIndex]; if (selected) insertMathSymbol(selected); return; }
            if (e.key === "Escape") { setMathMenu(null); return; }
        }

        // UNDO/REDO
        if ((e.ctrlKey || e.metaKey) && e.key === 'z') {
            e.preventDefault();
            if (e.shiftKey) redo(); else undo();
            return;
        }
        if ((e.ctrlKey || e.metaKey) && e.key === 'y') {
            e.preventDefault();
            redo();
            return;
        }

        // ZOOM SHORTCUTS
        if ((e.ctrlKey || e.metaKey) && (e.key === '=' || e.key === '+')) {
            e.preventDefault();
            setZoom(prev => Math.min(prev + 0.1, 2.0));
            return;
        }
        if ((e.ctrlKey || e.metaKey) && e.key === '-') {
            e.preventDefault();
            setZoom(prev => Math.max(prev - 0.1, 0.5));
            return;
        }
        if ((e.ctrlKey || e.metaKey) && e.key === '0') {
            e.preventDefault();
            setZoom(1.0);
            return;
        }

        const block = blocks.find(b => b.id === id);
        if (!block) return;

        // ENTER - SPLIT BLOCK
        if (e.key === 'Enter' && !e.shiftKey) {
            e.preventDefault();
            const target = e.currentTarget as HTMLTextAreaElement;
            const newId = splitBlock(id, target.selectionStart);
            setFocusedId(newId);
            pendingCursor.current = { id: newId, pos: 0 };
            return;
        }

        // BACKSPACE - MERGE UP
        if (e.key === 'Backspace') {
            const target = e.currentTarget as HTMLTextAreaElement;
            if (target.selectionStart === 0 && target.selectionEnd === 0 && index > 0) {
                e.preventDefault();
                const prevBlock = blocks[index - 1];
                const joinPos = prevBlock.content.length;
                const prevId = mergeBlock(id, 'up');
                setFocusedId(prevId);
                pendingCursor.current = { id: prevId, pos: joinPos };
            }
        }

        // DELETE - MERGE DOWN
        if (e.key === 'Delete') {
            const target = e.currentTarget as HTMLTextAreaElement;
            if (target.selectionStart === target.value.length && target.selectionEnd === target.value.length && index < blocks.length - 1) {
                e.preventDefault();
                const currentLen = block.content.length;
                mergeBlock(id, 'down');
                pendingCursor.current = { id: id, pos: currentLen };
            }
        }

        // ARROW UP
        if (e.key === "ArrowUp" && index > 0) {
            const target = e.currentTarget as HTMLTextAreaElement;
            if (target.selectionStart === 0) {
                e.preventDefault();
                const prevBlock = blocks[index - 1];
                setFocusedId(prevBlock.id);
                pendingCursor.current = { id: prevBlock.id, pos: prevBlock.content.length };
            }
        }

        // ARROW DOWN
        if (e.key === "ArrowDown" && index < blocks.length - 1) {
            const target = e.currentTarget as HTMLTextAreaElement;
            if (target.selectionStart === target.value.length) {
                e.preventDefault();
                setFocusedId(blocks[index + 1].id);
                pendingCursor.current = { id: blocks[index + 1].id, pos: 0 };
            }
        }
    };

    const onPaste = (e: React.ClipboardEvent, id: string) => {
        e.preventDefault();
        const text = e.clipboardData.getData('text/plain');
        const target = e.currentTarget as HTMLTextAreaElement;
        const success = handlePaste(id, text, target.selectionStart);
        if (!success) {
            const currentBlock = blocks.find(b => b.id === id);
            if (!currentBlock) return;
            const newContent = currentBlock.content.slice(0, target.selectionStart) + text + currentBlock.content.slice(target.selectionEnd);
            updateBlock(id, newContent);
        }
    };

    if (!activeNoteId) return <div className="flex-1 bg-zinc-950 flex items-center justify-center text-zinc-700 font-mono text-xs uppercase tracking-widest select-none">Awaiting Input...</div>;

    return (
        <main
            ref={scrollContainerRef}
            className="flex-1 h-screen bg-zinc-950 text-white p-16 pb-[50vh] overflow-y-auto relative custom-scrollbar w-full max-w-full"
            onClick={() => { }}
        >
            <div className="absolute top-4 right-8 text-[10px] uppercase tracking-widest font-black select-none">
                {isSaving ? <span className="text-purple-500 animate-pulse">Syncing...</span> : <span className="text-zinc-800">Synced</span>}
            </div>

            {/* ZOOM WRAPPER */}
            <div style={{ transform: `scale(${zoom})`, transformOrigin: 'top left', width: `${100 / zoom}%` }}>
                <input
                    ref={titleRef}
                    value={title}
                    onChange={(e) => setTitle(e.target.value)}
                    onKeyDown={(e) => {
                        if (e.key === 'Enter') {
                            e.preventDefault();
                            if (blocks.length > 0) {
                                setFocusedId(blocks[0].id);
                                pendingCursor.current = { id: blocks[0].id, pos: 0 };
                            } else {
                                addBlock(null);
                            }
                        }
                    }}
                    className="text-7xl font-black bg-transparent border-none outline-none text-zinc-100 mb-20 w-full tracking-tighter placeholder-zinc-900 break-words"
                    placeholder="Untitled"
                    style={{ fontSize: `${zoom * 5}rem` }}
                />

                {/* BLOCKS */}
                <div className="flex flex-col gap-2 w-full max-w-full">
                    {blocks.map((block, index) => (
                        <EditorBlock
                            key={block.id} block={block} index={index}
                            isFocused={focusedId === block.id}
                            updateBlock={handleBlockUpdate}
                            onFocus={setFocusedId}
                            onKeyDown={handleKeyDown}
                            onPaste={onPaste}
                            zoom={zoom}
                        />
                    ))}
                </div>

                <div className="h-40 cursor-text" onClick={() => {
                    if (blocks.length > 0) return;
                    addBlock(null);
                }} />
            </div>

            {/* COMMAND MENU */}
            {commandMenu?.isOpen && (
                <div id="onyx-context-menu" className={`fixed z-50 bg-zinc-900 border border-zinc-800 rounded-xl p-2 w-96 flex flex-col overflow-hidden shadow-2xl`} style={{
                    [commandMenu.direction === 'up' ? 'bottom' : 'top']: commandMenu.direction === 'up' ? (window.innerHeight - commandMenu.y + 20) : commandMenu.y,
                    left: commandMenu.x,
                    transform: `scale(${zoom})`,
                    transformOrigin: commandMenu.direction === 'up' ? 'bottom left' : 'top left'
                }}>
                    <div className="px-4 py-2 text-xs font-bold text-zinc-600 uppercase tracking-widest">Turn Into</div>
                    <div ref={menuScrollRef} className="max-h-96 overflow-y-auto no-scrollbar flex flex-col">
                        {filteredCommands.map((option, i) => (
                            <button key={option.type} onClick={() => transformBlock(option.type)} className={`flex items-center gap-4 w-full px-6 py-4 rounded-lg transition-colors duration-75 text-left ${commandMenu.selectedIndex === i ? 'bg-zinc-800 text-zinc-100' : 'text-zinc-500 hover:bg-zinc-800/40 hover:text-zinc-300'}`}>
                                <div className={`w-10 h-10 shrink-0 flex items-center justify-center text-sm font-black bg-black/40 rounded ${commandMenu.selectedIndex === i ? 'text-zinc-100' : 'text-zinc-600'}`}>{option.indicator}</div>
                                <span className={`text-lg font-bold tracking-tight ${commandMenu.selectedIndex === i ? 'text-zinc-100' : 'text-zinc-400'}`}>{option.label}</span>
                            </button>
                        ))}
                    </div>
                </div>
            )}

            {/* MATH MENU */}
            {mathMenu?.isOpen && (
                <div id="onyx-math-menu" className={`fixed z-50 bg-zinc-900 border border-zinc-800 rounded-xl p-2 w-96 flex flex-col overflow-hidden shadow-2xl`} style={{
                    [mathMenu.direction === 'up' ? 'bottom' : 'top']: mathMenu.direction === 'up' ? (window.innerHeight - mathMenu.y + 20) : mathMenu.y,
                    left: mathMenu.x,
                    transform: `scale(${zoom})`,
                    transformOrigin: mathMenu.direction === 'up' ? 'bottom left' : 'top left'
                }}>
                    <div className="px-4 py-2 text-xs font-bold text-zinc-500 uppercase tracking-widest">Math Symbols</div>
                    <div ref={mathMenuScrollRef} className="max-h-80 overflow-y-auto no-scrollbar flex flex-col">
                        {filteredMath.map((symbol, i) => (
                            <button key={symbol.cmd} onClick={() => insertMathSymbol(symbol)} className={`flex items-center gap-4 w-full px-6 py-3 rounded-lg transition-colors duration-75 text-left ${mathMenu.selectedIndex === i ? 'bg-zinc-800 text-zinc-100' : 'text-zinc-500 hover:bg-zinc-800/40 hover:text-zinc-300'}`}>
                                <div className={`w-10 h-10 shrink-0 flex items-center justify-center bg-black/40 rounded ${mathMenu.selectedIndex === i ? 'text-zinc-100' : 'text-zinc-500'}`}><LazyInlineMath math={symbol.cmd} /></div>
                                <div className="flex flex-col"><span className={`text-base font-bold ${mathMenu.selectedIndex === i ? 'text-zinc-100' : 'text-zinc-400'}`}>{symbol.name}</span><span className="text-xs font-mono opacity-40">{symbol.cmd}</span></div>
                            </button>
                        ))}
                    </div>
                </div>
            )}
        </main>
    );
}