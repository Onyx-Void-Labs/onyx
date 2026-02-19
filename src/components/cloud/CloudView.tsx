import { Cloud, FolderOpen, File, Upload, Search, Shield, HardDrive, FolderPlus, MoreHorizontal } from 'lucide-react';

// ─── Cloud View (File Manager) ──────────────────────────────────────────────

const FOLDER_TREE = [
    { name: 'My Files', icon: FolderOpen, active: true },
    { name: 'Documents', icon: FolderOpen },
    { name: 'Shared', icon: FolderOpen },
    { name: 'Recent', icon: File },
    { name: 'Trash', icon: File },
];

const SAMPLE_FILES = [
    { name: 'Documents', type: 'folder' as const, size: '—', modified: 'Jan 15, 2026' },
    { name: 'Projects', type: 'folder' as const, size: '—', modified: 'Feb 3, 2026' },
    { name: 'backup-2026.zip', type: 'file' as const, size: '2.4 GB', modified: 'Feb 10, 2026' },
    { name: 'presentation.pdf', type: 'file' as const, size: '18 MB', modified: 'Feb 8, 2026' },
    { name: 'notes-export.md', type: 'file' as const, size: '156 KB', modified: 'Feb 11, 2026' },
];

export default function CloudView() {
    return (
        <div className="flex h-full overflow-hidden">
            {/* Folder Sidebar */}
            <div className="w-56 bg-zinc-900/60 flex flex-col border-r border-zinc-800/30 shrink-0">
                <div className="h-12 px-4 flex items-center gap-2 border-b border-zinc-800/30 shrink-0">
                    <Cloud size={16} className="text-sky-400" />
                    <span className="text-sm font-semibold text-zinc-200">Cloud Drive</span>
                </div>

                <div className="flex-1 overflow-y-auto p-2 space-y-0.5">
                    {FOLDER_TREE.map(folder => (
                        <button
                            key={folder.name}
                            className={`w-full flex items-center gap-3 px-3 py-2 rounded-lg text-sm transition-colors ${folder.active
                                ? 'bg-zinc-800/60 text-zinc-100'
                                : 'text-zinc-500 hover:text-zinc-300 hover:bg-zinc-800/30'
                                }`}
                        >
                            <folder.icon size={16} className={folder.active ? 'text-sky-400' : 'text-zinc-600'} />
                            <span className="flex-1 text-left truncate">{folder.name}</span>
                        </button>
                    ))}

                    <div className="pt-2">
                        <button className="w-full flex items-center justify-center gap-2 px-3 py-2 rounded-lg text-sm text-zinc-500 hover:text-sky-400 hover:bg-sky-500/8 border border-dashed border-zinc-800 hover:border-sky-500/30 transition-all">
                            <FolderPlus size={14} />
                            <span>New Folder</span>
                        </button>
                    </div>
                </div>

                {/* Storage */}
                <div className="border-t border-zinc-800/30 p-4 shrink-0 space-y-2">
                    <div className="flex items-center gap-2 mb-1">
                        <HardDrive size={14} className="text-zinc-500" />
                        <span className="text-xs font-semibold uppercase tracking-wider text-zinc-500">Storage</span>
                    </div>
                    <div className="flex items-center justify-between text-xs">
                        <span className="text-zinc-400">0 GB used</span>
                        <span className="text-zinc-600">200 GB</span>
                    </div>
                    <div className="h-1.5 bg-zinc-800 rounded-full overflow-hidden">
                        <div className="h-full w-0 bg-gradient-to-r from-sky-500 to-blue-500 rounded-full" />
                    </div>
                </div>
            </div>

            {/* File Grid */}
            <div className="flex-1 flex flex-col bg-zinc-950/50 overflow-hidden">
                {/* Toolbar */}
                <div className="h-12 px-4 flex items-center justify-between border-b border-zinc-800/30 shrink-0">
                    <div className="flex items-center gap-2">
                        <span className="text-sm font-semibold text-zinc-200">My Files</span>
                        <span className="text-xs text-zinc-600">/ root</span>
                    </div>
                    <div className="flex items-center gap-2">
                        <button className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-sky-500/15 text-sky-400 hover:bg-sky-500/20 transition-colors text-xs font-semibold">
                            <Upload size={14} />
                            Upload
                        </button>
                        <button className="p-1.5 rounded-md hover:bg-zinc-800 text-zinc-500 transition-colors">
                            <Search size={16} />
                        </button>
                    </div>
                </div>

                {/* File Table */}
                <div className="flex-1 overflow-y-auto">
                    {/* Table Header */}
                    <div className="grid grid-cols-[1fr_100px_140px_40px] px-4 py-2 text-xs font-semibold text-zinc-600 uppercase tracking-wider border-b border-zinc-800/20 sticky top-0 bg-zinc-950/80 backdrop-blur-sm">
                        <span>Name</span>
                        <span>Size</span>
                        <span>Modified</span>
                        <span></span>
                    </div>

                    {/* File Rows */}
                    {SAMPLE_FILES.map(file => (
                        <div
                            key={file.name}
                            className="grid grid-cols-[1fr_100px_140px_40px] px-4 py-2.5 hover:bg-zinc-800/20 cursor-pointer transition-colors border-b border-zinc-800/10 group"
                        >
                            <div className="flex items-center gap-3">
                                {file.type === 'folder' ? (
                                    <FolderOpen size={16} className="text-sky-400 shrink-0" />
                                ) : (
                                    <File size={16} className="text-zinc-500 shrink-0" />
                                )}
                                <span className={`text-sm truncate ${file.type === 'folder' ? 'text-zinc-200 font-medium' : 'text-zinc-400'}`}>
                                    {file.name}
                                </span>
                            </div>
                            <span className="text-sm text-zinc-600 flex items-center">{file.size}</span>
                            <span className="text-sm text-zinc-600 flex items-center">{file.modified}</span>
                            <div className="flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity">
                                <button className="p-1 rounded-md hover:bg-zinc-700 text-zinc-500 transition-colors">
                                    <MoreHorizontal size={14} />
                                </button>
                            </div>
                        </div>
                    ))}
                </div>

                {/* E2EE Badge */}
                <div className="absolute bottom-6 right-6 flex items-center gap-2 pointer-events-none">
                    <div className="flex items-center gap-2 px-3 py-1.5 rounded-full bg-sky-500/10 border border-sky-500/20 text-xs font-medium text-sky-400">
                        <Shield size={12} />
                        <span>End-to-end encrypted</span>
                    </div>
                </div>
            </div>
        </div>
    );
}
