import React, { useEffect, useState, useRef } from 'react';
import { EditorView } from '@codemirror/view';

interface ContextMenuProps {
    view: EditorView | null;
    parentRef: React.RefObject<HTMLDivElement | null>;
}

export const ContextMenu: React.FC<ContextMenuProps> = ({ view, parentRef }) => {
    const [visible, setVisible] = useState(false);
    const [pos, setPos] = useState({ x: 0, y: 0 });
    const [context, setContext] = useState<'math' | 'text' | null>(null);
    const [mathContent, setMathContent] = useState<string>("");

    const menuRef = useRef<HTMLDivElement>(null);

    useEffect(() => {
        const parent = parentRef.current;
        if (!parent || !view) return;

        const handleContextMenu = (e: MouseEvent) => {
            e.preventDefault();

            const pos = view.posAtCoords({ x: e.clientX, y: e.clientY });
            if (pos === null) return;

            const doc = view.state.doc.toString();
            const before = doc.slice(0, pos);
            const after = doc.slice(pos);
            const lastDollar = before.lastIndexOf('$');
            const nextDollar = after.indexOf('$');

            let isMath = false;
            let content = "";

            if (lastDollar !== -1 && nextDollar !== -1) {
                const dollarsBefore = (before.slice(0, lastDollar).match(/\$/g) || []).length;
                if (dollarsBefore % 2 === 0) {
                    isMath = true;
                    content = doc.slice(lastDollar + 1, pos + nextDollar);
                    if (content.startsWith('$') && after.slice(nextDollar + 1).startsWith('$')) {
                        content = doc.slice(lastDollar + 2, pos + nextDollar);
                    }
                }
            }

            if (isMath) {
                setContext('math');
                setMathContent(content);
            } else {
                setContext('text');
            }

            setPos({ x: e.clientX, y: e.clientY });
            setVisible(true);
        };

        const handleClick = (e: MouseEvent) => {
            if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
                setVisible(false);
            }
        };

        parent.addEventListener('contextmenu', handleContextMenu);
        window.addEventListener('click', handleClick);

        return () => {
            parent.removeEventListener('contextmenu', handleContextMenu);
            window.removeEventListener('click', handleClick);
        };
    }, [view, parentRef]);

    if (!visible) return null;

    const copyToClipboard = (text: string) => {
        navigator.clipboard.writeText(text);
        setVisible(false);
    };

    const applyFormat = (mod: string) => {
        if (!view) return;
        const { from, to } = view.state.selection.main;
        const selected = view.state.sliceDoc(from, to);
        view.dispatch({
            changes: { from, to, insert: `${mod}${selected}${mod}` },
            selection: { anchor: from + mod.length, head: to + mod.length }
        });
        setVisible(false);
    };

    return (
        <div
            ref={menuRef}
            className="fixed z-50 min-w-[180px] overflow-hidden rounded-xl border border-zinc-700/50 bg-zinc-900/95 backdrop-blur-xl shadow-2xl animate-in fade-in zoom-in-95 duration-100 ring-1 ring-white/10"
            style={{
                top: pos.y,
                left: pos.x,
                boxShadow: '0 0 20px -5px rgba(0, 0, 0, 0.5), 0 0 10px -2px rgba(168, 85, 247, 0.2)' // Subtle purple glow
            }}
        >
            {context === 'math' && (
                <div className="p-1">
                    <div className="px-3 py-2 text-[10px] font-bold uppercase tracking-widest text-zinc-500/80 select-none">Math Actions</div>
                    <MenuItem label="Copy LaTeX" onClick={() => copyToClipboard(mathContent)} shortcut="Ctrl+C" icon="📋" />
                </div>
            )}

            {context === 'text' && (
                <div className="p-1">
                    <div className="px-3 py-2 text-[10px] font-bold uppercase tracking-widest text-zinc-500/80 select-none">Format</div>
                    {/* Horizontal Format Bar */}
                    <div className="flex gap-1 px-1 pb-1">
                        <IconButton label="B" onClick={() => applyFormat('**')} active />
                        <IconButton label="I" onClick={() => applyFormat('*')} />
                        <IconButton label="Code" onClick={() => applyFormat('`')} />
                    </div>
                </div>
            )}
        </div>
    );
};

const MenuItem: React.FC<{ label: string, onClick: () => void, shortcut?: string, icon?: string }> = ({ label, onClick, shortcut, icon }) => (
    <button
        onClick={onClick}
        className="group relative flex w-full items-center justify-between rounded-md px-3 py-2 text-sm text-zinc-300 transition-all hover:bg-zinc-800 hover:text-white"
    >
        <div className="flex items-center gap-2">
            {icon && <span className="opacity-70 group-hover:opacity-100">{icon}</span>}
            <span>{label}</span>
        </div>
        {shortcut && <span className="text-xs text-zinc-600 group-hover:text-zinc-500">{shortcut}</span>}
    </button>
);

const IconButton: React.FC<{ label: string, onClick: () => void, active?: boolean }> = ({ label, onClick }) => (
    <button
        onClick={onClick}
        className="flex h-8 w-full items-center justify-center rounded-md bg-zinc-800/50 text-sm font-medium text-zinc-400 hover:bg-zinc-700 hover:text-white transition-colors border border-transparent hover:border-zinc-600"
    >
        {label}
    </button>
);
