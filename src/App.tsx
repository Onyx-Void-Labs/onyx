
import { useState, useEffect } from "react";

import Sidebar from "./components/ui/Sidebar";
import Editor from "./components/editor/Editor";
import TabBar from "./components/ui/TabBar";
import Titlebar from "./components/ui/Titlebar";
import SearchModal from "./components/ui/SearchModal";
import { pb } from "./lib/pocketbase";

import { remove } from "@tauri-apps/plugin-fs";
import { documentDir, join } from "@tauri-apps/api/path";

import { useSync } from "./contexts/SyncContext";
import { SettingsProvider, useSettings } from "./contexts/SettingsContext";
import { WorkspaceProvider, useWorkspace } from "./contexts/WorkspaceContext";
import SettingsModal from "./components/settings/v2/SettingsModal";
import AuthModal from "./components/auth/AuthModal";

// Module Views
import MessagesView from "./components/messages/MessagesView";
import CalendarView from "./components/calendar/CalendarView";
import EmailView from "./components/email/EmailView";
import PhotosView from "./components/photos/PhotosView";
import PasswordsView from "./components/passwords/PasswordsView";
import CloudView from "./components/cloud/CloudView";

function AppContent() {
  // Use Sync Context for File List (Single Source of Truth)
  const { files, deleteFile } = useSync();
  const { toggleSettings, settings } = useSettings();
  const { activeWorkspace } = useWorkspace();

  // Local UI State
  const [tabs, setTabs] = useState<string[]>([]);
  const [activeTabId, setActiveTabId] = useState<string | null>(null);
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false);
  const [searchOpen, setSearchOpen] = useState(false);
  const [authOpen, setAuthOpen] = useState(false);
  const [user, setUser] = useState<any>(pb.authStore.model);

  useEffect(() => {
    // Subscribe to auth changes
    const unsubscribe = pb.authStore.onChange((_token, model) => {
      setUser(model);
    });

    return () => {
      unsubscribe();
    };
  }, []);

  const handleLogout = () => {
    pb.authStore.clear();
  };


  // Cleanup tabs when files are deleted remotely
  useEffect(() => {
    const loadedNoteIds = new Set(files.map(n => n.id));
    const validTabs = tabs.filter(tabId => loadedNoteIds.has(tabId));

    if (validTabs.length !== tabs.length) {
      console.log("Reconciling tabs: Closing deleted notes");
      setTabs(validTabs);

      if (activeTabId !== null && !loadedNoteIds.has(activeTabId)) {
        setActiveTabId(validTabs.length > 0 ? validTabs[validTabs.length - 1] : null);
      }
    }
  }, [files, tabs, activeTabId]);

  // Global keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && (e.key === 'p' || e.key === 't')) {
        e.preventDefault();
        setSearchOpen(true);
      }
      if ((e.ctrlKey || e.metaKey) && e.key === ',') {
        e.preventDefault();
        if (!import.meta.env.VITE_DEMO_MODE) {
          toggleSettings(true);
        }
      }
      if ((e.ctrlKey || e.metaKey) && e.key === '\\') {
        e.preventDefault();
        setSidebarCollapsed(prev => !prev);
      }
      if ((e.ctrlKey || e.metaKey) && (e.key === 'f' || e.key === 'F')) {
        e.preventDefault();
      }
      if ((e.ctrlKey || e.metaKey) && e.key === 'w') {
        e.preventDefault();
        if (activeTabId !== null) {
          closeTab(activeTabId);
        }
      }
      if (e.ctrlKey && e.key === 'Tab' && !e.shiftKey) {
        e.preventDefault();
        if (tabs.length > 1 && activeTabId !== null) {
          const currentIndex = tabs.indexOf(activeTabId);
          const nextIndex = (currentIndex + 1) % tabs.length;
          setActiveTabId(tabs[nextIndex]);
        }
      }
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

  const openTab = (id: string, forceNew: boolean = false) => {
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

  const closeTab = (id: string) => {
    const currentIndex = tabs.indexOf(id);
    const newTabs = tabs.filter((t) => t !== id);
    setTabs(newTabs);
    if (activeTabId === id && newTabs.length > 0) {
      const nextIndex = Math.min(currentIndex, newTabs.length - 1);
      setActiveTabId(newTabs[nextIndex]);
    } else if (newTabs.length === 0) {
      setActiveTabId(null);
    }
  };

  // Import at top level needed (handled by next tool or assume globals? No, imports needed)
  // But for this block:
  const handleDeleteNote = async (id: string) => {
    try {
      // 1. Delete from Sync Context (Database)
      deleteFile(id);

      // 2. Delete Mirror File (if enabled)
      if (settings.mirrorEnabled) {
        try {
          let basePath = settings.mirrorPath;
          // Resolve default path if not set
          if (!basePath) {
            const docs = await documentDir();
            basePath = await join(docs, 'Onyx Notes');
          }

          // PERSISTENT FILE MAPPING: Use localStorage to get the actual filename
          const storedFilename = localStorage.getItem(`mirror-filename-${id}`);
          if (storedFilename) {
            const fileName = `${storedFilename}.md`;
            const fullPath = await join(basePath, fileName);

            // Delete file (to bin or permanent based on setting)
            if (settings.mirrorDeleteToBin) {
              const { invoke } = await import('@tauri-apps/api/core');
              await invoke('move_to_trash', { path: fullPath });
              console.log('[App] Moved mirror file to Recycle Bin:', fullPath);
            } else {
              await remove(fullPath);
              console.log('[App] Deleted mirror file permanently:', fullPath);
            }

            // Clean up localStorage entry
            localStorage.removeItem(`mirror-filename-${id}`);
          }
        } catch (err) {
          console.error('[App] Failed to delete mirror file:', err);
          // Don't block UI for this failure
        }
      }

      closeTab(id);
    } catch (e) {
      console.error("Delete failed", e);
    }
  };

  const handleSearchSelect = (id: string) => {
    openTab(id, true);
  };

  const reorderTabs = (fromIndex: number, toIndex: number) => {
    const reorderedTabs = [...tabs];
    const [moved] = reorderedTabs.splice(fromIndex, 1);
    reorderedTabs.splice(toIndex, 0, moved);
    setTabs(reorderedTabs);
  };

  const handleLockNote = async (_id: string, _password: string) => {
    console.warn("Locking not yet implemented for Pure Yjs");
  };

  // ─── Determine sidebar visibility per module ────────────────────────────
  // Some modules have their own built-in sidebars (messages, email, photos, cloud)
  // Notes uses the existing sidebar. Calendar has its own agenda sidebar.
  const showNoteSidebar = activeWorkspace === 'notes';

  // ─── Render the active module content ───────────────────────────────────
  const renderModuleContent = () => {
    switch (activeWorkspace) {
      case 'notes':
        return (
          <div className="flex flex-col flex-1 overflow-hidden relative">
            <TabBar
              tabs={tabs}
              activeTabId={activeTabId}
              onSelectTab={setActiveTabId}
              onCloseTab={closeTab}
              onReorderTabs={reorderTabs}
              notes={files}
            />
            <Editor
              activeNoteId={activeTabId}
            />
          </div>
        );
      case 'messages':
        return <MessagesView />;
      case 'calendar':
        return <CalendarView />;
      case 'email':
        return <EmailView />;
      case 'photos':
        return <PhotosView />;
      case 'passwords':
        return <PasswordsView />;
      case 'cloud':
        return <CloudView />;
      default:
        return null;
    }
  };

  return (
    <div className="flex flex-col h-screen w-screen bg-zinc-950 overflow-hidden select-none rounded-lg relative">
      <Titlebar
        sidebarCollapsed={sidebarCollapsed}
        onToggleSidebar={() => setSidebarCollapsed(!sidebarCollapsed)}
      />

      <div className="flex flex-1 overflow-hidden">
        {/* Notes sidebar — only shown for notes workspace */}
        {showNoteSidebar && (
          <div
            className={`shrink-0 transition-all duration-300 ease-in-out overflow-hidden ${sidebarCollapsed ? 'w-0' : 'w-64'}`}
          >
            <Sidebar
              onSelectNote={openTab}
              activeNoteId={activeTabId}
              notes={files}
              openTabs={tabs}
              onDeleteNote={handleDeleteNote}
              onOpenSearch={() => setSearchOpen(true)}
              onLockNote={handleLockNote}
              onOpenAuth={() => toggleSettings(true)}
            />
          </div>
        )}

        {/* Main content area — switches based on active workspace */}
        <div className="flex-1 flex flex-col overflow-hidden">
          {renderModuleContent()}
        </div>
      </div>

      <SearchModal
        isOpen={searchOpen}
        onClose={() => setSearchOpen(false)}
        notes={files}
        onSelectNote={handleSearchSelect}
      />

      <SettingsModal
        user={user}
        onLogout={handleLogout}
      />

      <AuthModal
        isOpen={authOpen}
        onClose={() => setAuthOpen(false)}
      />


    </div>
  );
}

export default function App() {
  return (
    <SettingsProvider>
      <WorkspaceProvider>
        <AppContent />
      </WorkspaceProvider>
    </SettingsProvider>
  );
}
