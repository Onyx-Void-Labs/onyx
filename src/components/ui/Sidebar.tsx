
import { Search, PlusSquare, FileText, Trash2, Lock, Settings } from "lucide-react";
import { useState, useEffect } from "react";
import LockModal from "./LockModal";
// import LoginModal from "./LoginModal";
// import { usePocketBase } from "../../contexts/PocketBaseContext";
import { useSync } from "../../contexts/SyncContext";
import { useSettings } from "../../contexts/SettingsContext";

type Note = {
    id: string;
    title: string;
};

interface SidebarProps {
    onSelectNote: (id: string, forceNew: boolean) => void;
    activeNoteId: string | null;
    notes: Note[];
    openTabs: string[];
    onDeleteNote: (id: string) => void;
    onOpenSearch: () => void;
    onLockNote: (id: string, password: string) => Promise<void>;
    onOpenAuth: () => void;
}

export default function Sidebar({
    onSelectNote,
    activeNoteId,
    notes,
    openTabs,
    onDeleteNote,
    onOpenSearch,
    onLockNote,
    onOpenAuth
}: SidebarProps) {

    const [lockingNoteId, setLockingNoteId] = useState<string | null>(null);
    const [lockingNoteTitle, setLockingNoteTitle] = useState("");

    // Auth Context - Removed for Pure Yjs Rewrite
    const { status, createFile } = useSync();
    const { offlineMode } = useSettings();

    const handleNewPage = async () => {
        try {
            // Yjs: Create Local File Instantly
            const newId = createFile();
            // onSelectNote will handle the UI update
            onSelectNote(newId, true);
        } catch (error) {
            console.error("Failed to create note:", error);
        }
    };

    const handleLockClick = (id: string, title: string) => {
        setLockingNoteId(id);
        setLockingNoteTitle(title);
    }

    // Debounce status to prevent flickering (RGB light effect)
    const [displayedStatus, setDisplayedStatus] = useState(status);

    useEffect(() => {
        // If connected, update instantly (responsiveness), otherwise debounce to hide micro-outages
        // Actually, debouncing everything is smoother for "flickering" issues.
        // If we go Connected -> Disconnected -> Connected, we want to ignore the middle part.
        const timer = setTimeout(() => {
            setDisplayedStatus(status);
        }, 1000); // 1s delay to stabilize UI

        return () => clearTimeout(timer);
    }, [status]);

    return (
        <aside className="w-64 h-full bg-gradient-to-b from-zinc-900 to-zinc-950 text-zinc-400 flex flex-col">
            <LockModal
                isOpen={!!lockingNoteId}
                onClose={() => setLockingNoteId(null)}
                onConfirm={async (password) => {
                    if (lockingNoteId) {
                        await onLockNote(lockingNoteId, password);
                        setLockingNoteId(null);
                    }
                }}
                noteTitle={lockingNoteTitle}
            />

            {/* ACTION MENU */}
            <div className="p-3 space-y-1">
                <div
                    onClick={onOpenSearch}
                    className="flex items-center gap-2 px-3 py-2.5 hover:bg-white/5 rounded-lg cursor-pointer transition-all duration-300 group relative overflow-hidden"
                >
                    <div className="absolute inset-0 bg-purple-500/0 group-hover:bg-purple-500/5 transition-colors duration-500" />
                    <Search size={16} className="text-zinc-500 group-hover:text-purple-400 group-hover:scale-125 group-hover:rotate-12 transition-all duration-500 ease-out z-10" />
                    <span className="text-sm font-medium text-zinc-400 group-hover:text-zinc-200 transition-colors duration-300 z-10">Search</span>
                </div>
                <div
                    onClick={handleNewPage}
                    className="flex items-center gap-2 px-3 py-2.5 hover:bg-white/5 rounded-lg cursor-pointer transition-all duration-300 group relative overflow-hidden"
                >
                    <div className="absolute inset-0 bg-emerald-500/0 group-hover:bg-emerald-500/5 transition-colors duration-500" />
                    <PlusSquare size={16} className="text-zinc-500 group-hover:text-emerald-400 group-hover:rotate-90 group-hover:scale-110 transition-all duration-500 ease-in-out z-10" />
                    <span className="text-sm font-medium text-zinc-400 group-hover:text-zinc-200 transition-colors duration-300 z-10">New Page</span>
                </div>

                {/* SETTINGS BUTTON - Hidden in Demo Mode */}
                {!import.meta.env.VITE_DEMO_MODE && (
                    <div
                        onClick={onOpenAuth}
                        className="flex items-center gap-2 px-3 py-2.5 hover:bg-white/5 rounded-lg cursor-pointer transition-all duration-150 group"
                    >
                        <Settings
                            size={16}
                            className="text-zinc-500 group-hover:text-purple-400 group-hover:rotate-90 transition-all duration-500 ease-in-out"
                        />
                        <div className="flex flex-1 items-center justify-between">
                            <span className="text-sm font-medium text-zinc-400 group-hover:text-zinc-200">Settings</span>

                            {/* SUBTLE STATUS DOT - Debounced */}
                            <div className="flex items-center gap-2">
                                {(!offlineMode) && (
                                    <div className={`w-1.5 h-1.5 rounded-full transition-colors duration-500 ${displayedStatus === 'connected' ? 'bg-emerald-500 shadow-[0_0_8px_rgba(16,185,129,0.5)]' :
                                        displayedStatus === 'connecting' ? 'bg-amber-500 animate-pulse' :
                                            'bg-red-500'
                                        }`} />
                                )}
                            </div>
                        </div>
                    </div>
                )}

            </div>

            {/* DIVIDER */}
            <div className="mx-3 h-px bg-gradient-to-r from-transparent via-zinc-800 to-transparent" />

            {/* WORKSPACE LIST */}
            <div className="flex-1 overflow-y-auto py-3 custom-scrollbar">
                <div className="px-4 pb-2 text-[10px] font-bold text-zinc-600 uppercase tracking-[0.15em]">Workspace</div>

                <div className="px-2 space-y-0.5">
                    {notes.map((note) => {
                        const isOpen = openTabs.includes(note.id);
                        const isActive = activeNoteId === note.id;

                        return (
                            <div
                                key={note.id}
                                onMouseDown={(e) => {
                                    if (e.button === 1) e.preventDefault(); // Prevent auto-scroll
                                    if (e.button === 0) onSelectNote(note.id, false);
                                    if (e.button === 1) onSelectNote(note.id, true);
                                }}
                                onAuxClick={(e) => e.preventDefault()} // Prevent auto-scroll on middle-click
                                className={`flex items-center justify-between px-3 py-2 cursor-pointer group transition-all duration-150 rounded-lg ${isActive
                                    ? 'bg-purple-500/10 text-zinc-100'
                                    : 'text-zinc-400 hover:bg-white/5 hover:text-zinc-200'
                                    }`}
                            >
                                <div className="flex items-center gap-2.5 truncate">
                                    {/* TAB INDICATOR DOT */}
                                    <div className={`w-1.5 h-1.5 rounded-full shrink-0 transition-all duration-300 ${isOpen
                                        ? 'bg-purple-400 shadow-[0_0_6px_rgba(168,85,247,0.5)]'
                                        : 'bg-zinc-700'
                                        }`} />

                                    <FileText size={14} className={`${isActive ? "text-purple-400" : "text-zinc-600 group-hover:text-zinc-400"} transition-colors`} />

                                    <span className={`text-sm truncate ${isActive ? "font-semibold" : "font-medium"}`}>
                                        {note.title || "Untitled"}
                                    </span>
                                </div>

                                {/* LOCK BUTTON */}
                                <button
                                    onClick={(e) => {
                                        e.stopPropagation();
                                        handleLockClick(note.id, note.title);
                                    }}
                                    className="opacity-0 group-hover:opacity-100 p-1 hover:bg-zinc-700 rounded text-zinc-500 hover:text-zinc-300 transition-all mr-1"
                                    title="Lock Note"
                                >
                                    <Lock size={12} />
                                </button>

                                {/* DELETE BUTTON */}
                                <button
                                    onClick={(e) => {
                                        e.stopPropagation();
                                        onDeleteNote(note.id);
                                    }}
                                    className="opacity-0 group-hover:opacity-100 p-1 hover:bg-red-500/20 rounded text-zinc-500 hover:text-red-400 transition-all"
                                >
                                    <Trash2 size={12} />
                                </button>
                            </div>
                        );
                    })}
                </div>
            </div>
        </aside>
    );
}