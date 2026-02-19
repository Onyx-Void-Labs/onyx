import { Image, Grid3X3, LayoutGrid, Search, Upload, Shield, Heart, FolderOpen } from 'lucide-react';
import { useState } from 'react';

// ─── Photos View ─────────────────────────────────────────────────────────────

const SAMPLE_ALBUMS = [
    { name: 'All Photos', count: 0, icon: Image },
    { name: 'Favorites', count: 0, icon: Heart },
    { name: 'Screenshots', count: 0, icon: Grid3X3 },
    { name: 'Downloads', count: 0, icon: FolderOpen },
];

export default function PhotosView() {
    const [viewMode, setViewMode] = useState<'grid' | 'list'>('grid');

    return (
        <div className="flex h-full overflow-hidden">
            {/* Albums Sidebar */}
            <div className="w-56 bg-zinc-900/60 flex flex-col border-r border-zinc-800/30 shrink-0">
                <div className="h-12 px-4 flex items-center gap-2 border-b border-zinc-800/30 shrink-0">
                    <Image size={16} className="text-rose-400" />
                    <span className="text-sm font-semibold text-zinc-200">Albums</span>
                </div>

                <div className="flex-1 overflow-y-auto p-2 space-y-0.5">
                    {SAMPLE_ALBUMS.map((album, i) => (
                        <button
                            key={album.name}
                            className={`w-full flex items-center gap-3 px-3 py-2 rounded-lg text-sm transition-colors ${i === 0
                                    ? 'bg-zinc-800/60 text-zinc-100'
                                    : 'text-zinc-500 hover:text-zinc-300 hover:bg-zinc-800/30'
                                }`}
                        >
                            <album.icon size={16} className={i === 0 ? 'text-rose-400' : 'text-zinc-600'} />
                            <span className="flex-1 text-left truncate">{album.name}</span>
                            <span className="text-xs text-zinc-600">{album.count}</span>
                        </button>
                    ))}

                    <div className="pt-2">
                        <button className="w-full flex items-center justify-center gap-2 px-3 py-2 rounded-lg text-sm text-zinc-500 hover:text-rose-400 hover:bg-rose-500/8 border border-dashed border-zinc-800 hover:border-rose-500/30 transition-all">
                            <FolderOpen size={14} />
                            <span>New Album</span>
                        </button>
                    </div>
                </div>

                {/* Storage info */}
                <div className="border-t border-zinc-800/30 p-4 shrink-0 space-y-2">
                    <div className="flex items-center justify-between text-xs">
                        <span className="text-zinc-500">Storage Used</span>
                        <span className="text-zinc-400">0 / 200 GB</span>
                    </div>
                    <div className="h-1.5 bg-zinc-800 rounded-full overflow-hidden">
                        <div className="h-full w-0 bg-gradient-to-r from-rose-500 to-pink-500 rounded-full" />
                    </div>
                </div>
            </div>

            {/* Gallery Area */}
            <div className="flex-1 flex flex-col bg-zinc-950/50 overflow-hidden">
                {/* Toolbar */}
                <div className="h-12 px-4 flex items-center justify-between border-b border-zinc-800/30 shrink-0">
                    <span className="text-sm font-semibold text-zinc-200">All Photos</span>
                    <div className="flex items-center gap-2">
                        <div className="flex items-center bg-zinc-800/30 rounded-lg overflow-hidden border border-zinc-700/20">
                            <button
                                onClick={() => setViewMode('grid')}
                                className={`p-1.5 transition-colors ${viewMode === 'grid' ? 'bg-zinc-700/50 text-zinc-200' : 'text-zinc-500'}`}
                            >
                                <Grid3X3 size={14} />
                            </button>
                            <button
                                onClick={() => setViewMode('list')}
                                className={`p-1.5 transition-colors ${viewMode === 'list' ? 'bg-zinc-700/50 text-zinc-200' : 'text-zinc-500'}`}
                            >
                                <LayoutGrid size={14} />
                            </button>
                        </div>
                        <button className="p-1.5 rounded-md hover:bg-zinc-800 text-zinc-500 transition-colors">
                            <Search size={16} />
                        </button>
                    </div>
                </div>

                {/* Empty State / Coming Soon */}
                <div className="flex-1 flex items-center justify-center">
                    <div className="text-center space-y-4 max-w-sm">
                        <div className="w-20 h-20 rounded-2xl bg-rose-500/10 border border-rose-500/20 flex items-center justify-center mx-auto">
                            <Shield size={32} className="text-rose-400" />
                        </div>
                        <div>
                            <h3 className="text-lg font-bold text-zinc-100 mb-1">Encrypted Photos</h3>
                            <p className="text-sm text-zinc-500 leading-relaxed">
                                Your photos, encrypted before they leave your device. Up to 200GB of private cloud storage.
                            </p>
                        </div>
                        <button className="inline-flex items-center gap-2 px-4 py-2 rounded-xl bg-rose-500/15 text-rose-400 hover:bg-rose-500/20 transition-colors text-sm font-semibold">
                            <Upload size={16} />
                            Upload Photos
                        </button>
                        <div className="inline-flex items-center gap-2 px-3 py-1.5 rounded-full bg-rose-500/10 border border-rose-500/20 text-xs font-medium text-rose-400 ml-3">
                            <div className="w-1.5 h-1.5 rounded-full bg-rose-400 animate-pulse" />
                            Coming Soon
                        </div>
                    </div>
                </div>
            </div>
        </div>
    );
}
