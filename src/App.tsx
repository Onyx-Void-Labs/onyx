import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import Sidebar from "./components/ui/Sidebar";
import Editor from "./components/editor/Editor";
import TabBar from "./components/ui/TabBar";
import Titlebar from "./components/ui/TitleBar";
import SearchModal from "./components/ui/SearchModal";

export default function App() {
  const [notes, setNotes] = useState<any[]>([]);
  const [tabs, setTabs] = useState<number[]>([]);
  const [activeTabId, setActiveTabId] = useState<number | null>(null);
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false);
  const [searchOpen, setSearchOpen] = useState(false);

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
      // Ctrl+B - Toggle sidebar
      if ((e.ctrlKey || e.metaKey) && e.key === 'b') {
        e.preventDefault();
        setSidebarCollapsed(prev => !prev);
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
            refreshNotes={fetchNotes}
            openTabs={tabs}
            onDeleteNote={handleDeleteNote}
            onOpenSearch={() => setSearchOpen(true)}
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