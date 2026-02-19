
import React, { createContext, useContext, useEffect, useState, useRef } from 'react';
import * as Y from 'yjs';
import { HocuspocusProvider } from '@hocuspocus/provider';
import { IndexeddbPersistence } from 'y-indexeddb';
import { FileMeta, SyncStatus } from '../types/sync';
// import { v4 as uuidv4 } from 'uuid';

const generateId = () => {
    if (typeof crypto !== 'undefined' && crypto.randomUUID) {
        return crypto.randomUUID();
    }
    return Math.random().toString(36).substring(2, 15) + Math.random().toString(36).substring(2, 15);
};

interface SyncContextType {
    files: FileMeta[];
    status: SyncStatus;
    createFile: (title?: string) => string; // Returns ID
    deleteFile: (id: string) => void;
    updateFile: (id: string, updates: Partial<FileMeta>) => void;
    provider: HocuspocusProvider | null; // Exposed for debugging or advanced use
}

const SyncContext = createContext<SyncContextType | null>(null);

export const useSync = () => {
    const context = useContext(SyncContext);
    if (!context) throw new Error('useSync must be used within a SyncProvider');
    return context;
};

export const SyncProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
    const [files, setFiles] = useState<FileMeta[]>([]);
    const [status, setStatus] = useState<SyncStatus>('connecting');

    // Refs for Yjs instances to prevent re-creation
    const docRef = useRef<Y.Doc>(new Y.Doc());
    const providerRef = useRef<HocuspocusProvider | null>(null);
    const persistenceRef = useRef<IndexeddbPersistence | null>(null);

    useEffect(() => {
        const doc = docRef.current;

        // 1. Persistence (Offline First)
        // We sync the 'filesystem' room which contains the list of files
        persistenceRef.current = new IndexeddbPersistence('onyx-filesystem', doc);

        persistenceRef.current.on('synced', () => {
            console.log('[Sync] Filesystem loaded from local DB');

            // DEMO MODE: Create Welcome Note if empty
            if (import.meta.env.VITE_DEMO_MODE) {
                // Check if files map is empty
                const filesMap = doc.getMap<FileMeta>('files');
                if (filesMap.size === 0) {
                    console.log('[Demo] Creating Welcome Note');
                    const id = generateId(); // Use local helper
                    const newFile: FileMeta = {
                        id,
                        title: 'Welcome to Onyx',
                        createdAt: Date.now(),
                        updatedAt: Date.now()
                    };
                    filesMap.set(id, newFile);

                    // Add content to the note
                    // Note: Content is stored in a separate Y.Doc per note, not the filesystem doc.
                    const noteDoc = new Y.Doc();
                    const notePersistence = new IndexeddbPersistence(`onyx-note-${id}`, noteDoc);

                    notePersistence.on('synced', () => {
                        console.log('[Demo] Seeding Welcome Note content');
                        const contentText = noteDoc.getText('codemirror');
                        contentText.insert(0, '# Welcome to Onyx\n\nThis is a **live demo** of the Onyx encrypted workspace.\n\n- **Private**: Your data is stored locally in your browser.\n- **Secure**: No server sees your data in this demo.\n- **Transformed**: Experience the UI/UX of the desktop app right here.\n\nTry creating a new note or exploring the interface!');

                        // Cleanup after a short delay to ensure save
                        setTimeout(() => {
                            notePersistence.destroy();
                            noteDoc.destroy();
                        }, 1000);
                    });
                }
            }
        });

        // 2. WebSocket Provider (Server Sync)
        const wsUrl = import.meta.env.VITE_WS_URL || 'ws://localhost:1234';

        // Get Auth Token for Hocuspocus
        import('../lib/pocketbase').then(async ({ pb }) => {
            // Check if we are logged in
            if (!pb.authStore.isValid) {
                console.warn('[Sync] No auth token, operating in offline mode');
                setStatus('offline');
                return;
            }

            const token = pb.authStore.token;
            const userId = pb.authStore.model?.id;

            if (!token || !userId) {
                console.warn('[Sync] No auth token, operating in offline mode');
                setStatus('offline');
                return;
            }

            // Room name is scoped to user ID for multi-tenancy
            const roomName = `user-${userId}-filesystem`;

            const provider = new HocuspocusProvider({
                url: wsUrl,
                name: roomName,
                document: doc,
                token: token,
                onStatus: ({ status }) => {
                    setStatus(status as SyncStatus);
                }
            });

            providerRef.current = provider;
        });

        // 3. Observe the Files Map
        const filesMap = doc.getMap<FileMeta>('files');

        const updateFilesState = () => {
            // Convert Y.Map to Array and Sort by createdAt (semantically stable)
            const fileList = Array.from(filesMap.values()).sort((a, b) => b.createdAt - a.createdAt);
            setFiles(fileList);
        };

        filesMap.observe(() => {
            updateFilesState();
        });

        // Initial state update (in case satisfied from IndexedDB before observation)
        updateFilesState();

        return () => {
            if (providerRef.current) providerRef.current.destroy();
            if (persistenceRef.current) persistenceRef.current.destroy();
            // We generally don't destroy the doc as it might be shared, but here it's scoped to provider
            doc.destroy();
        };
    }, []);

    const createFile = (title: string = 'Untitled Note') => {
        const id = generateId();
        const newFile: FileMeta = {
            id,
            title,
            createdAt: Date.now(),
            updatedAt: Date.now()
        };

        const filesMap = docRef.current.getMap<FileMeta>('files');
        filesMap.set(id, newFile);

        return id;
    };

    const deleteFile = (id: string) => {
        const filesMap = docRef.current.getMap<FileMeta>('files');
        filesMap.delete(id);
    };

    const updateFile = (id: string, updates: Partial<FileMeta>) => {
        const filesMap = docRef.current.getMap<FileMeta>('files');
        const current = filesMap.get(id);
        if (current) {
            filesMap.set(id, { ...current, ...updates, updatedAt: Date.now() });
        }
    };

    return (
        <SyncContext.Provider value={{ files, status, createFile, deleteFile, updateFile, provider: providerRef.current }}>
            {children}
        </SyncContext.Provider>
    );
};
