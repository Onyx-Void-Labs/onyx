import { invoke } from "@tauri-apps/api/core";
import { Search, PlusSquare, FileText, Trash2 } from "lucide-react";

type Note = {
    id: number;
    title: string;
};

interface SidebarProps {
    onSelectNote: (id: number, forceNew: boolean) => void;
    activeNoteId: number | null;
    notes: Note[];
    refreshNotes: () => Promise<void>;
    openTabs: number[];
    onDeleteNote: (id: number) => void;
    onOpenSearch: () => void;
}

export default function Sidebar({
    onSelectNote,
    activeNoteId,
    notes,
    refreshNotes,
    openTabs,
    onDeleteNote,
    onOpenSearch
}: SidebarProps) {

    const handleNewPage = async () => {
        try {
            const newId = await invoke<number>("create_note", {
                title: "",
                content: ""
            });
            await refreshNotes(); // Wait for notes to refresh before selecting
            onSelectNote(newId, true);
        } catch (error) {
            console.error("Failed to create note:", error);
        }
    };

    return (
        <aside className="w-64 h-full bg-gradient-to-b from-zinc-900 to-zinc-950 text-zinc-400 flex flex-col">
            {/* ACTION MENU */}
            <div className="p-3 space-y-1">
                <div
                    onClick={onOpenSearch}
                    className="flex items-center gap-2 px-3 py-2.5 hover:bg-white/5 rounded-lg cursor-pointer transition-all duration-150 group"
                >
                    <Search size={16} className="text-zinc-500 group-hover:text-purple-400 transition-colors" />
                    <span className="text-sm font-medium text-zinc-400 group-hover:text-zinc-200">Search</span>
                </div>
                <div
                    onClick={handleNewPage}
                    className="flex items-center gap-2 px-3 py-2.5 hover:bg-white/5 rounded-lg cursor-pointer transition-all duration-150 group"
                >
                    <PlusSquare size={16} className="text-zinc-500 group-hover:text-purple-400 transition-colors" />
                    <span className="text-sm font-medium text-zinc-400 group-hover:text-zinc-200">New Page</span>
                </div>
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