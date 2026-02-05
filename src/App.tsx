import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import Sidebar from "./components/ui/Sidebar";
import Editor from "./components/editor/Editor"; // Using CodeMirror 6 editor
import TabBar from "./components/ui/TabBar";
import Titlebar from "./components/ui/Titlebar";
import SearchModal from "./components/ui/SearchModal";
import { encryptNote } from "./services/SecurityService";

export default function App() {
  const [notes, setNotes] = useState<any[]>([]);
  const [tabs, setTabs] = useState<number[]>([]);
  const [activeTabId, setActiveTabId] = useState<number | null>(null);
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false);
  const [searchOpen, setSearchOpen] = useState(false);
  const [refreshTrigger, setRefreshTrigger] = useState(0);

  const fetchNotes = async () => {
    try {
      const fetched = await invoke<any[]>("get_notes");
      setNotes(fetched);
    } catch (e) {
      console.error("Fetch failed", e);
    }
  };

  useEffect(() => { fetchNotes(); }, []);

  // Global keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Ctrl+P or Ctrl+T - Open search
      if ((e.ctrlKey || e.metaKey) && (e.key === 'p' || e.key === 't')) {
        e.preventDefault();
        setSearchOpen(true);
      }
      // Ctrl+\ - Toggle sidebar (VS Code style)
      if ((e.ctrlKey || e.metaKey) && e.key === '\\') {
        e.preventDefault();
        setSidebarCollapsed(prev => !prev);
      }

      // Ctrl+F - Prevent default (Tauri/Browser Find) globally
      // We handle this inside EditorV2 locally, but we need to stop it elsewhere too
      if ((e.ctrlKey || e.metaKey) && (e.key === 'f' || e.key === 'F')) {
        e.preventDefault();
      }

      // Ctrl+W - Close current tab
      if ((e.ctrlKey || e.metaKey) && e.key === 'w') {
        e.preventDefault();
        if (activeTabId !== null) {
          closeTab(activeTabId);
        }
      }
      // Ctrl+Tab - Next tab
      if (e.ctrlKey && e.key === 'Tab' && !e.shiftKey) {
        e.preventDefault();
        if (tabs.length > 1 && activeTabId !== null) {
          const currentIndex = tabs.indexOf(activeTabId);
          const nextIndex = (currentIndex + 1) % tabs.length;
          setActiveTabId(tabs[nextIndex]);
        }
      }
      // Ctrl+Shift+Tab - Previous tab
      if (e.ctrlKey && e.key === 'Tab' && e.shiftKey) {
        e.preventDefault();
        if (tabs.length > 1 && activeTabId !== null) {
          const currentIndex = tabs.indexOf(activeTabId);
          const prevIndex = (currentIndex - 1 + tabs.length) % tabs.length;
          setActiveTabId(tabs[prevIndex]);
        }
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [tabs, activeTabId]);

  const openTab = (id: number, forceNew: boolean = false) => {
    if (tabs.includes(id)) {
      setActiveTabId(id);
      return;
    }
    if (forceNew || tabs.length === 0 || activeTabId === null) {
      setTabs([...tabs, id]);
      setActiveTabId(id);
    } else {
      if (activeTabId !== null) {
        const newTabs = tabs.map(t => t === activeTabId ? id : t);
        setTabs(newTabs);
        setActiveTabId(id);
      } else {
        setTabs([id]);
        setActiveTabId(id);
      }
    }
  };

  const closeTab = (id: number) => {
    const currentIndex = tabs.indexOf(id);
    const newTabs = tabs.filter((t) => t !== id);
    setTabs(newTabs);
    if (activeTabId === id && newTabs.length > 0) {
      // Select the next tab (right), or previous if closing last tab
      const nextIndex = Math.min(currentIndex, newTabs.length - 1);
      setActiveTabId(newTabs[nextIndex]);
    } else if (newTabs.length === 0) {
      setActiveTabId(null);
    }
  };

  const handleDeleteNote = async (id: number) => {
    try {
      await invoke("delete_note", { id });
      setTabs(tabs.filter(t => t !== id));
      if (activeTabId === id) {
        const remaining = tabs.filter(t => t !== id);
        setActiveTabId(remaining.length > 0 ? remaining[remaining.length - 1] : null);
      }
      fetchNotes();
    } catch (e) {
      console.error("Delete failed", e);
    }
  };

  const handleSearchSelect = (id: number) => {
    openTab(id, true); // Always add to end
  };

  const reorderTabs = (fromIndex: number, toIndex: number) => {
    const newTabs = [...tabs];
    const [moved] = newTabs.splice(fromIndex, 1);
    newTabs.splice(toIndex, 0, moved);
    setTabs(newTabs);
  };

  const handleLockNote = async (id: number, password: string) => {
    try {
      // 1. Fetch current content
      // detailed note structure: { id, title, content, ... }
      const note: any = await invoke("get_note", { id });

      // 2. Encrypt
      const encryptedData = await encryptNote(note.content, password);
      const encryptedContentString = JSON.stringify(encryptedData);

      // 3. Save
      await invoke("update_note", {
        id,
        title: note.title,
        content: encryptedContentString
      });

      console.log(`Note ${id} locked successfully.`);

      // Force refresh to ensure UI reflects state if needed? 
      // Actually, EditorV2 needs to know.
      // If we locked the ACTIVE note, we should force it to reload.
      if (activeTabId === id) {
        setRefreshTrigger(prev => prev + 1);
      }
      fetchNotes();

    } catch (e) {
      console.error("Locking failed", e);
    }
  };

  return (
    <div className="flex flex-col h-screen w-screen bg-zinc-950 overflow-hidden select-none rounded-lg">
      {/* Custom Titlebar */}
      <Titlebar
        sidebarCollapsed={sidebarCollapsed}
        onToggleSidebar={() => setSidebarCollapsed(!sidebarCollapsed)}
      />

      {/* Main Content */}
      <div className="flex flex-1 overflow-hidden">
        {/* Sidebar with collapse animation */}
        <div
          className={`transition-all duration-300 ease-in-out overflow-hidden ${sidebarCollapsed ? 'w-0' : 'w-64'}`}
        >
          <Sidebar
            onSelectNote={openTab}
            activeNoteId={activeTabId}
            notes={notes}
            refreshNotes={async () => { await fetchNotes(); }}
            openTabs={tabs}
            onDeleteNote={handleDeleteNote}
            onOpenSearch={() => setSearchOpen(true)}
            onLockNote={handleLockNote}
          />
        </div>

        {/* Editor Area */}
        <div className="flex flex-col flex-1 overflow-hidden">
          <TabBar
            tabs={tabs}
            activeTabId={activeTabId}
            onSelectTab={setActiveTabId}
            onCloseTab={closeTab}
            onReorderTabs={reorderTabs}
            notes={notes}
          />
          <Editor
            activeNoteId={activeTabId}
            refreshTrigger={refreshTrigger}
            onSave={fetchNotes}
          />
        </div>
      </div>

      {/* Search Modal */}
      <SearchModal
        isOpen={searchOpen}
        onClose={() => setSearchOpen(false)}
        notes={notes}
        onSelectNote={handleSearchSelect}
        onRefreshNotes={fetchNotes}
      />
    </div>
  );
}