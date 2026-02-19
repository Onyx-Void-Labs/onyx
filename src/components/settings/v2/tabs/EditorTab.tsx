import { Terminal, ToggleLeft, ToggleRight, Keyboard } from 'lucide-react';
import { useSettings } from '../../../../contexts/SettingsContext';

export default function EditorTab() {
    const {
        editorFontFamily,
        vimMode,
        lineNumbers,
        updateSettings
    } = useSettings();

    const codeFonts = [
        { id: '"JetBrains Mono", monospace', name: 'JetBrains Mono' },
        { id: '"Fira Code", monospace', name: 'Fira Code' },
        { id: 'Consolas, monospace', name: 'Consolas' },
    ];

    return (
        <div className="space-y-8 animate-in fade-in slide-in-from-bottom-4 duration-500">

            <div className="flex items-center justify-between">
                <div>
                    <h2 className="text-2xl font-bold text-white mb-1">Editor Settings</h2>
                    <p className="text-zinc-400 text-sm">Fine-tune your writing and coding experience.</p>
                </div>
            </div>

            {/* Editor Font */}
            <div className="space-y-4">
                <h3 className="text-sm font-bold text-zinc-300 uppercase tracking-wider flex items-center gap-2">
                    <TypeIcon size={16} /> Font Configuration
                </h3>
                <div className="p-6 bg-zinc-900/50 border border-white/5 rounded-3xl space-y-6">
                    <div className="grid grid-cols-3 gap-3">
                        {codeFonts.map((font) => (
                            <button
                                key={font.id}
                                onClick={() => updateSettings({ editorFontFamily: font.id as any })}
                                className={`px-4 py-3 rounded-xl text-sm border transition-all ${editorFontFamily === font.id
                                    ? 'bg-blue-500/10 border-blue-500/50 text-blue-400 font-bold'
                                    : 'bg-black/20 border-white/5 text-zinc-400 hover:bg-white/5'
                                    }`}
                                style={{ fontFamily: font.id }}
                            >
                                {font.name}
                            </button>
                        ))}
                    </div>
                </div>
            </div>

            {/* Visual Behavior Toggles (Mini Windows) */}
            <div className="space-y-4">
                <h3 className="text-sm font-bold text-zinc-300 uppercase tracking-wider flex items-center gap-2">
                    <Terminal size={16} /> Interface & Behavior
                </h3>

                <div className="grid grid-cols-1 md:grid-cols-2 gap-6">

                    {/* Line Numbers Preview Card */}
                    <button
                        onClick={() => updateSettings({ lineNumbers: !lineNumbers })}
                        className={`relative overflow-hidden p-1 rounded-3xl border transition-all duration-300 text-left group ${lineNumbers
                            ? 'bg-blue-500/5 border-blue-500/30'
                            : 'bg-zinc-900/40 border-zinc-800 hover:border-zinc-700'
                            }`}
                    >
                        <div className="absolute inset-x-0 top-0 h-32 bg-gradient-to-b from-black/50 to-transparent z-10" />

                        {/* Mini Window Visual */}
                        <div className="relative h-32 bg-[#1e1e2e] rounded-t-2xl border-b border-white/5 overflow-hidden flex font-mono text-[10px] leading-relaxed p-4 pt-6 mx-1 mt-1">
                            {/* Gutter */}
                            <div className={`flex flex-col text-zinc-600 pr-3 border-r border-white/5 mr-3 transition-all duration-300 ${lineNumbers ? 'opacity-100 translate-x-0 w-6' : 'opacity-0 -translate-x-4 w-0 border-none'}`}>
                                <span>1</span>
                                <span>2</span>
                                <span>3</span>
                            </div>
                            {/* Code */}
                            <div className="text-zinc-400">
                                <div className="text-blue-400">function <span className="text-yellow-400">add</span>(a, b) {'{'}</div>
                                <div className="pl-4">return a + b;</div>
                                <div>{'}'}</div>
                            </div>
                        </div>

                        <div className="p-5 relative z-20 bg-gradient-to-t from-zinc-900/90 via-zinc-900/50 to-transparent -mt-10 pt-10">
                            <div className="flex items-center justify-between">
                                <div>
                                    <div className={`text-sm font-bold transition-colors ${lineNumbers ? 'text-blue-400' : 'text-zinc-300'}`}>Line Numbers</div>
                                    <div className="text-[10px] text-zinc-500 mt-0.5">Show gutter indicators</div>
                                </div>
                                {lineNumbers
                                    ? <ToggleRight size={28} className="text-blue-400" />
                                    : <ToggleLeft size={28} className="text-zinc-600" />
                                }
                            </div>
                        </div>
                    </button>

                    {/* Vim Mode Preview Card */}
                    <button
                        onClick={() => updateSettings({ vimMode: !vimMode })}
                        className={`relative overflow-hidden p-1 rounded-3xl border transition-all duration-300 text-left group ${vimMode
                            ? 'bg-green-500/5 border-green-500/30'
                            : 'bg-zinc-900/40 border-zinc-800 hover:border-zinc-700'
                            }`}
                    >
                        {/* Mini Window Visual */}
                        <div className="relative h-32 bg-[#1e1e2e] rounded-t-2xl border-b border-white/5 overflow-hidden flex flex-col justify-end p-2 mx-1 mt-1">
                            {/* Vim Status Bar */}
                            <div className={`flex items-center justify-between px-3 py-1 text-[9px] font-mono font-bold transition-all duration-300 ${vimMode ? 'bg-green-500 text-black translate-y-0' : 'bg-zinc-800 text-zinc-500 translate-y-8'}`}>
                                <div className="flex gap-4">
                                    <span>NORMAL</span>
                                    <span>main.tsx</span>
                                </div>
                                <span>100%</span>
                            </div>
                            <div className={`absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 transition-opacity duration-300 ${!vimMode ? 'opacity-100' : 'opacity-0'}`}>
                                <Keyboard size={32} className="text-zinc-700" />
                            </div>
                        </div>

                        <div className="p-5 relative z-20 bg-gradient-to-t from-zinc-900/90 via-zinc-900/50 to-transparent -mt-10 pt-10">
                            <div className="flex items-center justify-between">
                                <div>
                                    <div className={`text-sm font-bold transition-colors ${vimMode ? 'text-green-400' : 'text-zinc-300'}`}>Vim Mode</div>
                                    <div className="text-[10px] text-zinc-500 mt-0.5">Vim keybindings active</div>
                                </div>
                                {vimMode
                                    ? <ToggleRight size={28} className="text-green-400" />
                                    : <ToggleLeft size={28} className="text-zinc-600" />
                                }
                            </div>
                        </div>
                    </button>

                </div>
            </div>
        </div>
    );
}

function TypeIcon({ size }: { size: number }) {
    return (
        <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <polyline points="4 7 4 4 20 4 20 7" />
            <line x1="9" y1="20" x2="15" y2="20" />
            <line x1="12" y1="4" x2="12" y2="20" />
        </svg>
    )
}
