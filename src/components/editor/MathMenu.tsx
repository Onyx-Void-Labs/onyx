import React, { useEffect, useRef } from 'react';
import { InlineMath } from 'react-katex';

export interface MathOption {
    cmd: string;
    name: string;
}

interface MathMenuProps {
    visible: boolean;
    position: { x: number, y: number };
    options: MathOption[];
    selectedIndex: number;
    onSelect: (cmd: string) => void;
}

export const MathMenu: React.FC<MathMenuProps> = ({ visible, position, options, selectedIndex, onSelect }) => {
    const listRef = useRef<HTMLDivElement>(null);

    // Auto-scroll to selected item
    useEffect(() => {
        if (visible && listRef.current) {
            const selectedEl = listRef.current.children[selectedIndex] as HTMLElement;
            if (selectedEl) {
                selectedEl.scrollIntoView({ block: 'nearest' });
            }
        }
    }, [selectedIndex, visible]);

    if (!visible || options.length === 0) return null;

    return (
        <div
            className="fixed z-50 flex flex-col overflow-hidden bg-[#121212] border border-zinc-800 rounded-xl shadow-2xl animate-in fade-in zoom-in-95 duration-75"
            style={{
                top: position.y + 6,
                left: position.x,
                width: '300px',
                maxHeight: '360px',
                boxShadow: '0 12px 30px rgba(0,0,0,0.5)'
            }}
            onMouseDown={(e) => e.preventDefault()} // Prevent focus loss
        >
            <div className="px-3 py-2 text-[10px] font-bold text-zinc-500 uppercase tracking-widest border-b border-zinc-900/50 select-none">
                Math Symbols
            </div>

            <div
                ref={listRef}
                className="flex-1 overflow-y-auto p-1 custom-scrollbar"
            >
                {options.map((sym, i) => (
                    <div
                        key={sym.cmd}
                        className={`flex items-center gap-3 px-3 py-2 rounded-lg cursor-pointer transition-colors ${i === selectedIndex ? 'bg-zinc-800' : 'hover:bg-zinc-800/50'
                            }`}
                        onClick={() => onSelect(sym.cmd)}
                    >
                        {/* Icon Box */}
                        <div className={`
                            flex items-center justify-center w-10 h-10 rounded-lg border 
                            ${i === selectedIndex ? 'bg-[#1e1e20] border-zinc-700 text-white' : 'bg-[#18181b] border-zinc-800 text-zinc-500'}
                            text-lg select-none
                        `}>
                            <InlineMath math={sym.cmd} />
                        </div>

                        {/* Text Info */}
                        <div className="flex flex-col">
                            <span className={`text-sm font-semibold ${i === selectedIndex ? 'text-white' : 'text-zinc-500'
                                }`}>
                                {sym.name}
                            </span>
                            <span className="text-xs font-mono text-zinc-600">
                                {sym.cmd}
                            </span>
                        </div>
                    </div>
                ))}
            </div>
        </div>
    );
};
