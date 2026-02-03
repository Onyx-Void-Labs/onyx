import { InlineMath } from 'react-katex';
import { BlockType } from '../../types';
import { MathSymbol } from '../../data/mathSymbols'; // Keep your existing data file

// --- COMMAND MENU ---
export const CommandMenu = ({ 
    position, 
    selectedIndex, 
    options,
    onSelect 
}: { 
    position: { x: number; y: number; dir: 'up' | 'down' };
    selectedIndex: number; 
    options: any[]; 
    onSelect: (type: BlockType) => void;
}) => {
    return (
        <div 
            className="fixed z-50 bg-zinc-900/95 backdrop-blur-xl border border-zinc-800 rounded-xl p-2 w-80 shadow-2xl flex flex-col overflow-hidden animate-in fade-in zoom-in-95 duration-100"
            style={{ 
                left: position.x, 
                [position.dir === 'up' ? 'bottom' : 'top']: position.dir === 'up' ? window.innerHeight - position.y + 10 : position.y + 10 
            }}
        >
            <div className="px-3 py-2 text-[10px] font-bold text-zinc-600 uppercase tracking-widest mb-1">Slash Commands</div>
            {options.map((opt, i) => (
                <button 
                    key={opt.type} 
                    onClick={() => onSelect(opt.type)}
                    className={`flex items-center gap-3 w-full px-3 py-2 rounded-lg text-left transition-all ${
                        i === selectedIndex ? 'bg-zinc-800 text-white translate-x-1' : 'text-zinc-500 hover:text-zinc-300'
                    }`}
                >
                    <div className="w-8 h-8 rounded flex items-center justify-center bg-black/40 text-xs font-mono border border-white/5">
                        {opt.indicator}
                    </div>
                    <span className="text-sm font-medium">{opt.label}</span>
                </button>
            ))}
        </div>
    );
};

// --- MATH MENU ---
export const MathMenu = ({ 
    position, 
    selectedIndex, 
    symbols,
    onSelect 
}: { 
    position: { x: number; y: number; dir: 'up' | 'down' };
    selectedIndex: number; 
    symbols: MathSymbol[]; 
    onSelect: (sym: MathSymbol) => void;
}) => {
    return (
        <div 
            className="fixed z-50 bg-zinc-900/95 backdrop-blur-xl border border-zinc-800 rounded-xl p-2 w-80 shadow-2xl flex flex-col overflow-hidden animate-in fade-in zoom-in-95 duration-100"
            style={{ 
                left: position.x, 
                [position.dir === 'up' ? 'bottom' : 'top']: position.dir === 'up' ? window.innerHeight - position.y + 10 : position.y + 10 
            }}
        >
             <div className="px-3 py-2 text-[10px] font-bold text-zinc-600 uppercase tracking-widest mb-1">Math Intelligence</div>
             <div className="max-h-64 overflow-y-auto custom-scrollbar">
                {symbols.map((sym, i) => (
                    <button 
                        key={sym.cmd} 
                        onClick={() => onSelect(sym)}
                        className={`flex items-center gap-3 w-full px-3 py-2 rounded-lg text-left transition-all ${
                            i === selectedIndex ? 'bg-zinc-800 text-white translate-x-1' : 'text-zinc-500 hover:text-zinc-300'
                        }`}
                    >
                         <div className="w-8 h-8 rounded flex items-center justify-center bg-black/40 text-lg border border-white/5">
                            <InlineMath math={sym.cmd} />
                         </div>
                         <div className="flex flex-col">
                            <span className="text-sm font-medium">{sym.name}</span>
                            <span className="text-[10px] font-mono opacity-50">{sym.cmd}</span>
                         </div>
                    </button>
                ))}
             </div>
        </div>
    );
};