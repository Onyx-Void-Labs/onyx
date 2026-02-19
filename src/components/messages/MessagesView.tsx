import { Hash, Volume2, Plus, Search, ChevronDown, Smile, Paperclip, Send, Shield } from 'lucide-react';

// ─── Messages View (Discord-like layout placeholder) ─────────────────────────

export default function MessagesView() {
    return (
        <div className="flex h-full overflow-hidden">
            {/* Server / DM Sidebar */}
            <div className="w-[72px] bg-zinc-950 flex flex-col items-center py-3 gap-2 border-r border-zinc-800/30 shrink-0">
                {/* Home / DMs button */}
                <button className="w-12 h-12 rounded-2xl bg-blue-500/20 text-blue-400 flex items-center justify-center hover:rounded-xl hover:bg-blue-500/30 transition-all duration-200">
                    <svg width="24" height="24" viewBox="0 0 24 24" fill="none"><path d="M19.73 4.87l-15.46 6.27a.5.5 0 00.01.95l5.15 1.72 1.72 5.15a.5.5 0 00.95.01l6.27-15.46a.5.5 0 00-.64-.64z" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" /></svg>
                </button>

                <div className="w-8 border-t border-zinc-800/50 my-1" />

                {/* Sample servers */}
                {['🎮', '💻', '🎵', '📚'].map((emoji, i) => (
                    <button
                        key={i}
                        className="w-12 h-12 rounded-3xl bg-zinc-800/60 text-lg flex items-center justify-center hover:rounded-xl hover:bg-blue-500/20 transition-all duration-200 group"
                    >
                        <span className="group-hover:scale-110 transition-transform">{emoji}</span>
                    </button>
                ))}

                {/* Add Server */}
                <button className="w-12 h-12 rounded-3xl bg-zinc-800/40 text-emerald-400 flex items-center justify-center hover:rounded-xl hover:bg-emerald-500/20 transition-all duration-200">
                    <Plus size={20} />
                </button>
            </div>

            {/* Channel Sidebar */}
            <div className="w-60 bg-zinc-900/60 flex flex-col border-r border-zinc-800/30 shrink-0">
                {/* Server Header */}
                <div className="h-12 px-4 flex items-center justify-between border-b border-zinc-800/30 shrink-0">
                    <span className="text-sm font-semibold text-zinc-200 truncate">Onyx Community</span>
                    <ChevronDown size={16} className="text-zinc-500" />
                </div>

                {/* Channels */}
                <div className="flex-1 overflow-y-auto p-2 space-y-4">
                    {/* Text Channels */}
                    <div>
                        <div className="flex items-center gap-1 px-2 py-1">
                            <ChevronDown size={10} className="text-zinc-500" />
                            <span className="text-[11px] font-bold uppercase tracking-wider text-zinc-500">Text Channels</span>
                        </div>
                        {['general', 'development', 'design', 'off-topic'].map((ch, i) => (
                            <button
                                key={ch}
                                className={`w-full flex items-center gap-2 px-2 py-1.5 rounded-md text-sm transition-colors ${i === 0
                                        ? 'bg-zinc-800/60 text-zinc-100'
                                        : 'text-zinc-500 hover:text-zinc-300 hover:bg-zinc-800/30'
                                    }`}
                            >
                                <Hash size={16} className="shrink-0 text-zinc-600" />
                                <span className="truncate">{ch}</span>
                            </button>
                        ))}
                    </div>

                    {/* Voice Channels */}
                    <div>
                        <div className="flex items-center gap-1 px-2 py-1">
                            <ChevronDown size={10} className="text-zinc-500" />
                            <span className="text-[11px] font-bold uppercase tracking-wider text-zinc-500">Voice Channels</span>
                        </div>
                        {['Lounge', 'Gaming', 'Music'].map(ch => (
                            <button
                                key={ch}
                                className="w-full flex items-center gap-2 px-2 py-1.5 rounded-md text-sm text-zinc-500 hover:text-zinc-300 hover:bg-zinc-800/30 transition-colors"
                            >
                                <Volume2 size={16} className="shrink-0 text-zinc-600" />
                                <span className="truncate">{ch}</span>
                            </button>
                        ))}
                    </div>
                </div>
            </div>

            {/* Main Chat Area */}
            <div className="flex-1 flex flex-col bg-zinc-950/50">
                {/* Channel Header */}
                <div className="h-12 px-4 flex items-center gap-3 border-b border-zinc-800/30 shrink-0">
                    <Hash size={18} className="text-zinc-600" />
                    <span className="text-sm font-semibold text-zinc-200">general</span>
                    <div className="w-px h-5 bg-zinc-800 mx-1" />
                    <span className="text-xs text-zinc-600 truncate">The place to start</span>
                    <div className="flex-1" />
                    <button className="p-1.5 rounded-md hover:bg-zinc-800 text-zinc-500 transition-colors">
                        <Search size={16} />
                    </button>
                </div>

                {/* Message Area — Coming Soon */}
                <div className="flex-1 flex items-center justify-center">
                    <div className="text-center space-y-4 max-w-sm">
                        <div className="w-20 h-20 rounded-2xl bg-blue-500/10 border border-blue-500/20 flex items-center justify-center mx-auto">
                            <Shield size={32} className="text-blue-400" />
                        </div>
                        <div>
                            <h3 className="text-lg font-bold text-zinc-100 mb-1">End-to-End Encrypted</h3>
                            <p className="text-sm text-zinc-500 leading-relaxed">
                                Messages with Signal-level encryption. Servers, channels, voice — all private by default.
                            </p>
                        </div>
                        <div className="inline-flex items-center gap-2 px-3 py-1.5 rounded-full bg-blue-500/10 border border-blue-500/20 text-xs font-medium text-blue-400">
                            <div className="w-1.5 h-1.5 rounded-full bg-blue-400 animate-pulse" />
                            Coming Soon
                        </div>
                    </div>
                </div>

                {/* Message Input */}
                <div className="p-4 shrink-0">
                    <div className="flex items-center gap-2 bg-zinc-800/40 rounded-xl px-4 py-2.5 border border-zinc-700/30">
                        <button className="text-zinc-500 hover:text-zinc-300 transition-colors">
                            <Paperclip size={18} />
                        </button>
                        <input
                            type="text"
                            placeholder="Message #general"
                            className="flex-1 bg-transparent text-sm text-zinc-300 placeholder-zinc-600 outline-none"
                            disabled
                        />
                        <button className="text-zinc-500 hover:text-zinc-300 transition-colors">
                            <Smile size={18} />
                        </button>
                        <button className="text-zinc-500 hover:text-blue-400 transition-colors">
                            <Send size={18} />
                        </button>
                    </div>
                </div>
            </div>
        </div>
    );
}
