import { useEffect, useRef, useState } from 'react';
import { Search, FileText, Plus } from 'lucide-react';
import { useSync } from '../../contexts/SyncContext';

type Note = {
    id: string;
    title: string;
};

interface SearchModalProps {
    isOpen: boolean;
    onClose: () => void;
    notes: Note[];
    onSelectNote: (id: string) => void;
}

export default function SearchModal({ isOpen, onClose, notes, onSelectNote }: SearchModalProps) {
    const { createFile, updateFile } = useSync();

    const [query, setQuery] = useState('');
    const [selectedIndex, setSelectedIndex] = useState(0);
    const inputRef = useRef<HTMLInputElement>(null);

    const filtered = notes.filter(n =>
        (n.title || 'Untitled').toLowerCase().includes(query.toLowerCase())
    );

    // Add "Create new" option when query exists but no exact match
    const hasExactMatch = notes.some(n => (n.title || '').toLowerCase() === query.toLowerCase());
    const showCreateOption = query.trim().length > 0 && !hasExactMatch;
    const totalItems = filtered.length + (showCreateOption ? 1 : 0);

    useEffect(() => {
        if (isOpen) {
            setQuery('');
            setSelectedIndex(0);
            setTimeout(() => inputRef.current?.focus(), 50);
        }
    }, [isOpen]);

    useEffect(() => {
        setSelectedIndex(0);
    }, [query]);

    const handleCreateNew = () => {
        try {
            const newId = createFile();
            if (query.trim()) {
                updateFile(newId, { title: query.trim() });
            }
            onSelectNote(newId);
            onClose();
        } catch (error) {
            console.error("Failed to create note:", error);
        }
    };

    const handleKeyDown = (e: React.KeyboardEvent) => {
        if (e.key === 'ArrowDown') {
            e.preventDefault();
            setSelectedIndex(prev => (prev + 1) % totalItems);
        } else if (e.key === 'ArrowUp') {
            e.preventDefault();
            setSelectedIndex(prev => (prev - 1 + totalItems) % totalItems);
        } else if (e.key === 'Enter') {
            if (showCreateOption && selectedIndex === filtered.length) {
                handleCreateNew();
            } else if (filtered[selectedIndex]) {
                onSelectNote(filtered[selectedIndex].id);
                onClose();
            }
        } else if (e.key === 'Escape') {
            onClose();
        }
    };

    if (!isOpen) return null;

    return (
        <div
            className="fixed inset-0 z-[100] flex items-start justify-center pt-[15vh] bg-black/60 backdrop-blur-sm"
            onClick={onClose}
        >
            <div
                className="w-[500px] bg-zinc-900 rounded-xl border border-zinc-800 shadow-2xl overflow-hidden"
                onClick={e => e.stopPropagation()}
            >
                {/* Search Input */}
                <div className="flex items-center gap-3 px-4 py-3 border-b border-zinc-800">
                    <Search size={18} className="text-zinc-500" />
                    <input
                        ref={inputRef}
                        type="text"
                        value={query}
                        onChange={e => setQuery(e.target.value)}
                        onKeyDown={handleKeyDown}
                        placeholder="Search or create note..."
                        className="flex-1 bg-transparent text-zinc-100 placeholder-zinc-600 outline-none text-sm"
                    />
                </div>

                {/* Results */}
                <div className="max-h-80 overflow-y-auto">
                    {filtered.map((note, i) => (
                        <div
                            key={note.id}
                            onClick={() => {
                                onSelectNote(note.id);
                                onClose();
                            }}
                            className={`flex items-center gap-3 px-4 py-3 cursor-pointer transition-colors ${i === selectedIndex
                                ? 'bg-purple-500/10 text-zinc-100'
                                : 'text-zinc-400 hover:bg-white/5'
                                }`}
                        >
                            <FileText size={16} className={i === selectedIndex ? 'text-purple-400' : 'text-zinc-600'} />
                            <span className="text-sm truncate">{note.title || 'Untitled'}</span>
                        </div>
                    ))}

                    {/* Create New Option */}
                    {showCreateOption && (
                        <div
                            onClick={handleCreateNew}
                            className={`flex items-center gap-3 px-4 py-3 cursor-pointer transition-colors border-t border-zinc-800/50 ${selectedIndex === filtered.length
                                ? 'bg-purple-500/10 text-zinc-100'
                                : 'text-zinc-400 hover:bg-white/5'
                                }`}
                        >
                            <Plus size={16} className={selectedIndex === filtered.length ? 'text-purple-400' : 'text-zinc-600'} />
                            <span className="text-sm">Create "<span className="text-purple-400 font-medium">{query}</span>"</span>
                        </div>
                    )}

                    {/* No Results */}
                    {filtered.length === 0 && !showCreateOption && (
                        <div className="px-4 py-8 text-center text-zinc-600 text-sm">
                            No notes found
                        </div>
                    )}
                </div>
            </div>
        </div>
    );
}
