import { KeyRound, Plus, Search, Shield, Copy, Eye, EyeOff, Globe, Clock, Star, AlertTriangle } from 'lucide-react';
import { useState } from 'react';

// ─── Password Manager / Vault View — Clean, no category sidebar ─────────────

type VaultFilter = 'all' | 'favorites' | 'recent';

const SAMPLE_ENTRIES = [
    { name: 'GitHub', username: 'omar@example.com', url: 'github.com', icon: '🐙', strength: 'strong' as const, fav: true, lastUsed: '2m ago' },
    { name: 'Google Account', username: 'omar@gmail.com', url: 'google.com', icon: '🔍', strength: 'strong' as const, fav: true, lastUsed: '1h ago' },
    { name: 'Discord', username: 'omar#1234', url: 'discord.com', icon: '🎮', strength: 'medium' as const, fav: false, lastUsed: '3h ago' },
    { name: 'Netflix', username: 'omar@example.com', url: 'netflix.com', icon: '🎬', strength: 'weak' as const, fav: false, lastUsed: '2d ago' },
    { name: 'AWS Console', username: 'admin@company.com', url: 'aws.amazon.com', icon: '☁️', strength: 'strong' as const, fav: true, lastUsed: '5h ago' },
    { name: 'Spotify', username: 'omar@example.com', url: 'spotify.com', icon: '🎵', strength: 'strong' as const, fav: false, lastUsed: '1d ago' },
    { name: 'Twitter / X', username: 'omar@example.com', url: 'x.com', icon: '𝕏', strength: 'medium' as const, fav: false, lastUsed: '4d ago' },
];

const strengthColors = {
    strong: 'bg-emerald-400',
    medium: 'bg-amber-400',
    weak: 'bg-red-400',
};

const strengthLabels = {
    strong: 'text-emerald-400',
    medium: 'text-amber-400',
    weak: 'text-red-400',
};

export default function PasswordsView() {
    const [selectedEntry, setSelectedEntry] = useState<number | null>(null);
    const [showPassword, setShowPassword] = useState(false);
    const [filter, setFilter] = useState<VaultFilter>('all');
    const [searchQuery, setSearchQuery] = useState('');

    // Filter entries
    const filteredEntries = SAMPLE_ENTRIES.filter(entry => {
        if (filter === 'favorites' && !entry.fav) return false;
        if (searchQuery) {
            const q = searchQuery.toLowerCase();
            return entry.name.toLowerCase().includes(q) || entry.username.toLowerCase().includes(q) || entry.url.toLowerCase().includes(q);
        }
        return true;
    });

    const entry = selectedEntry !== null ? SAMPLE_ENTRIES[selectedEntry] : null;

    return (
        <div className="flex h-full overflow-hidden">
            {/* Left: Search + Entry List (no category sidebar) */}
            <div className="w-80 bg-zinc-900/40 flex flex-col border-r border-zinc-800/30 shrink-0">
                {/* Search + New */}
                <div className="p-3 space-y-2 border-b border-zinc-800/30 shrink-0">
                    <div className="flex items-center gap-2">
                        <div className="flex-1 flex items-center gap-2 bg-zinc-800/40 rounded-lg px-3 py-2 border border-zinc-700/20 focus-within:border-indigo-500/30 transition-colors">
                            <Search size={14} className="text-zinc-600 shrink-0" />
                            <input
                                type="text"
                                placeholder="Search vault..."
                                value={searchQuery}
                                onChange={e => setSearchQuery(e.target.value)}
                                className="flex-1 bg-transparent text-sm text-zinc-300 placeholder-zinc-600 outline-none"
                            />
                        </div>
                        <button className="p-2 rounded-lg bg-indigo-500/15 text-indigo-400 hover:bg-indigo-500/25 transition-colors shrink-0" title="Add new">
                            <Plus size={16} />
                        </button>
                    </div>

                    {/* Filter pills */}
                    <div className="flex items-center gap-1">
                        {([
                            { key: 'all' as VaultFilter, label: 'All', count: SAMPLE_ENTRIES.length },
                            { key: 'favorites' as VaultFilter, label: 'Favorites', count: SAMPLE_ENTRIES.filter(e => e.fav).length },
                            { key: 'recent' as VaultFilter, label: 'Recent' },
                        ]).map(f => (
                            <button
                                key={f.key}
                                onClick={() => setFilter(f.key)}
                                className={`px-2.5 py-1 rounded-md text-xs font-medium transition-colors ${filter === f.key
                                        ? 'bg-indigo-500/15 text-indigo-300'
                                        : 'text-zinc-500 hover:text-zinc-300 hover:bg-zinc-800/40'
                                    }`}
                            >
                                {f.label}{f.count !== undefined ? ` (${f.count})` : ''}
                            </button>
                        ))}
                    </div>
                </div>

                {/* Entry list */}
                <div className="flex-1 overflow-y-auto">
                    {filteredEntries.length === 0 ? (
                        <div className="flex items-center justify-center h-32 text-sm text-zinc-600">
                            No items found
                        </div>
                    ) : (
                        filteredEntries.map((item, i) => {
                            const realIndex = SAMPLE_ENTRIES.indexOf(item);
                            return (
                                <button
                                    key={i}
                                    onClick={() => { setSelectedEntry(realIndex); setShowPassword(false); }}
                                    className={`w-full px-4 py-3 flex items-center gap-3 border-b border-zinc-800/15 transition-colors text-left ${realIndex === selectedEntry
                                            ? 'bg-indigo-500/8 border-l-2 border-l-indigo-400'
                                            : 'hover:bg-zinc-800/20 border-l-2 border-l-transparent'
                                        }`}
                                >
                                    <div className="w-9 h-9 rounded-lg bg-zinc-800/60 flex items-center justify-center text-base shrink-0">
                                        {item.icon}
                                    </div>
                                    <div className="flex-1 min-w-0">
                                        <div className="flex items-center gap-1.5">
                                            <span className="text-sm font-medium text-zinc-200 truncate">{item.name}</span>
                                            {item.fav && <Star size={10} className="text-amber-400 fill-amber-400 shrink-0" />}
                                        </div>
                                        <div className="text-xs text-zinc-500 truncate">{item.username}</div>
                                    </div>
                                    <div className="flex flex-col items-end gap-1 shrink-0">
                                        <div className={`w-2 h-2 rounded-full ${strengthColors[item.strength]}`} title={`${item.strength} password`} />
                                        <span className="text-[10px] text-zinc-600">{item.lastUsed}</span>
                                    </div>
                                </button>
                            );
                        })
                    )}
                </div>

                {/* Vault Health */}
                <div className="border-t border-zinc-800/30 p-3 shrink-0">
                    <div className="flex items-center gap-2 text-xs">
                        <Shield size={12} className="text-indigo-400" />
                        <span className="text-zinc-500 font-medium">Vault Health</span>
                        <div className="flex-1" />
                        <div className="flex items-center gap-1">
                            <AlertTriangle size={10} className="text-amber-400" />
                            <span className="text-amber-400 font-medium">1 weak</span>
                        </div>
                    </div>
                </div>
            </div>

            {/* Right: Detail Pane */}
            <div className="flex-1 flex flex-col bg-zinc-950/50 overflow-hidden relative">
                {entry ? (
                    <>
                        {/* Entry Header */}
                        <div className="px-8 pt-8 pb-6 border-b border-zinc-800/30 shrink-0">
                            <div className="flex items-center gap-4">
                                <div className="w-14 h-14 rounded-2xl bg-zinc-800/60 flex items-center justify-center text-2xl">
                                    {entry.icon}
                                </div>
                                <div className="flex-1 min-w-0">
                                    <div className="flex items-center gap-2">
                                        <h2 className="text-xl font-bold text-zinc-100">{entry.name}</h2>
                                        {entry.fav && <Star size={14} className="text-amber-400 fill-amber-400" />}
                                    </div>
                                    <span className="text-sm text-zinc-500">{entry.url}</span>
                                </div>
                            </div>
                        </div>

                        {/* Fields */}
                        <div className="flex-1 overflow-y-auto px-8 py-6 space-y-5">
                            {/* Username */}
                            <div className="space-y-1.5">
                                <label className="text-xs font-semibold uppercase tracking-wider text-zinc-600">Username</label>
                                <div className="flex items-center gap-2 bg-zinc-800/30 rounded-xl px-4 py-3 border border-zinc-700/20 group">
                                    <span className="flex-1 text-sm text-zinc-300 font-mono">{entry.username}</span>
                                    <button className="p-1 rounded-md opacity-0 group-hover:opacity-100 hover:bg-zinc-700 text-zinc-500 hover:text-zinc-300 transition-all">
                                        <Copy size={14} />
                                    </button>
                                </div>
                            </div>

                            {/* Password */}
                            <div className="space-y-1.5">
                                <label className="text-xs font-semibold uppercase tracking-wider text-zinc-600">Password</label>
                                <div className="flex items-center gap-2 bg-zinc-800/30 rounded-xl px-4 py-3 border border-zinc-700/20 group">
                                    <span className="flex-1 text-sm text-zinc-300 font-mono tracking-wider">
                                        {showPassword ? 'P@$$w0rd!2026' : '••••••••••••'}
                                    </span>
                                    <button
                                        onClick={() => setShowPassword(!showPassword)}
                                        className="p-1 rounded-md hover:bg-zinc-700 text-zinc-500 hover:text-zinc-300 transition-all"
                                    >
                                        {showPassword ? <EyeOff size={14} /> : <Eye size={14} />}
                                    </button>
                                    <button className="p-1 rounded-md opacity-0 group-hover:opacity-100 hover:bg-zinc-700 text-zinc-500 hover:text-zinc-300 transition-all">
                                        <Copy size={14} />
                                    </button>
                                </div>
                                {/* Strength bar */}
                                <div className="flex items-center gap-2 mt-1">
                                    <div className="flex gap-0.5 flex-1">
                                        {[...Array(4)].map((_, j) => (
                                            <div
                                                key={j}
                                                className={`h-1 flex-1 rounded-full ${j < (entry.strength === 'strong' ? 4 : entry.strength === 'medium' ? 2 : 1)
                                                        ? strengthColors[entry.strength]
                                                        : 'bg-zinc-800'
                                                    }`}
                                            />
                                        ))}
                                    </div>
                                    <span className={`text-[10px] font-semibold uppercase tracking-wider ${strengthLabels[entry.strength]}`}>
                                        {entry.strength}
                                    </span>
                                </div>
                            </div>

                            {/* Website */}
                            <div className="space-y-1.5">
                                <label className="text-xs font-semibold uppercase tracking-wider text-zinc-600">Website</label>
                                <div className="flex items-center gap-2 bg-zinc-800/30 rounded-xl px-4 py-3 border border-zinc-700/20 group">
                                    <Globe size={14} className="text-zinc-600 shrink-0" />
                                    <span className="flex-1 text-sm text-indigo-400 font-mono">{entry.url}</span>
                                    <button className="p-1 rounded-md opacity-0 group-hover:opacity-100 hover:bg-zinc-700 text-zinc-500 hover:text-zinc-300 transition-all">
                                        <Copy size={14} />
                                    </button>
                                </div>
                            </div>

                            {/* Notes */}
                            <div className="space-y-1.5">
                                <label className="text-xs font-semibold uppercase tracking-wider text-zinc-600">Notes</label>
                                <div className="bg-zinc-800/30 rounded-xl px-4 py-3 border border-zinc-700/20 min-h-[80px]">
                                    <span className="text-sm text-zinc-600 italic">No notes added</span>
                                </div>
                            </div>

                            {/* Last used */}
                            <div className="flex items-center gap-2 text-xs text-zinc-600 pt-2">
                                <Clock size={12} />
                                <span>Last used: {entry.lastUsed}</span>
                            </div>
                        </div>
                    </>
                ) : (
                    /* No selection state */
                    <div className="flex-1 flex items-center justify-center">
                        <div className="text-center space-y-4 max-w-xs">
                            <div className="w-16 h-16 rounded-2xl bg-indigo-500/10 border border-indigo-500/20 flex items-center justify-center mx-auto">
                                <KeyRound size={28} className="text-indigo-400" />
                            </div>
                            <div>
                                <h3 className="text-lg font-bold text-zinc-100 mb-1">ONYX Vault</h3>
                                <p className="text-sm text-zinc-500 leading-relaxed">
                                    Select an item or search your vault. Everything is zero-knowledge encrypted.
                                </p>
                            </div>
                        </div>
                    </div>
                )}

                {/* E2EE Badge */}
                <div className="absolute bottom-6 right-6 flex items-center gap-2 pointer-events-none">
                    <div className="flex items-center gap-2 px-3 py-1.5 rounded-full bg-indigo-500/10 border border-indigo-500/20 text-xs font-medium text-indigo-400">
                        <Shield size={12} />
                        <span>Zero-knowledge encrypted</span>
                    </div>
                </div>
            </div>
        </div>
    );
}
