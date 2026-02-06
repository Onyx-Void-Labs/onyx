
import { useEffect, useRef } from 'react';
import PocketBase from 'pocketbase';
import { invoke } from '@tauri-apps/api/core';

interface Note {
    id: number;
    title: string;
    updated_at: string;
    pb_id?: string | null;
}

export function usePocketBaseSync(
    pb: PocketBase,
    isConnected: boolean,
    user: any,
    refreshNotes: () => void
) {
    const isSyncingRef = useRef(false);

    useEffect(() => {
        if (!isConnected || !user) return;

        const syncList = async () => {
            if (isSyncingRef.current) return;
            isSyncingRef.current = true;

            console.log("Starting Full Sync...");

            try {
                // 1. Fetch Local Notes
                const localNotes = await invoke<Note[]>('get_notes');

                // 2. Fetch Remote Notes (All)
                const remoteNotes = await pb.collection('notes').getFullList();

                // 3. Compare & Sync

                // A. Upload Local -> Cloud (If missing pb_id)
                for (const note of localNotes) {
                    if (!note.pb_id) {
                        try {
                            console.log(`Uploading new local note: ${note.title}`);

                            // Parse content to unwrap JSON wrapper if present
                            let rawContent = (await invoke<any>('get_note_content', { id: note.id }))?.content || '';
                            try {
                                if (rawContent.trim().startsWith('[')) {
                                    const parsed = JSON.parse(rawContent);
                                    if (Array.isArray(parsed) && parsed.length > 0 && parsed[0].content) {
                                        rawContent = parsed[0].content;
                                    }
                                }
                            } catch (e) {
                                // Keep raw content if parse fails
                            }

                            // Create in PB
                            const created = await pb.collection('notes').create({
                                title: note.title,
                                content: rawContent,
                                owner: user.id
                            });

                            // Save PB ID locally
                            await invoke('update_note_pb_id', {
                                id: note.id,
                                pbId: created.id
                            });
                            // CRITICAL FIX: Update in-memory object so we don't re-import it below
                            note.pb_id = created.id;
                            console.log(`Synced ${note.title} -> ${created.id}`);
                        } catch (e) {
                            console.error(`Failed to upload note ${note.id}`, e);
                        }
                    }
                }

                // B. Download Cloud -> Local (If missing locally or newer)
                for (const remote of remoteNotes) {
                    const localMatch = localNotes.find(n => n.pb_id === remote.id);

                    if (!localMatch) {
                        // New from Cloud
                        console.log(`Downloading new cloud note: ${remote.title}`);
                        await invoke('import_note_from_pb', {
                            pbId: remote.id,
                            title: remote.title || 'Untitled',
                            content: remote.content || '',
                            updatedAt: remote.updated // PB uses 'updated'
                        });
                    } else {
                        // Conflict / Update Check
                        // PB 'updated' is ISO UTC. Local 'updated_at' is SQLite text (UTC? or Local?).
                        // SQLite CURRENT_TIMESTAMP is UTC.
                        const localTime = new Date(localMatch.updated_at + "Z").getTime(); // Append Z if SQLite stores without timezone but is UTC
                        const remoteTime = new Date(remote.updated).getTime();

                        // If Remote is significantly newer (> 2s to avoid clock drift issues on same sync)
                        // Actually, simplified: If Remote > Local.
                        if (remoteTime > localTime + 2000) {
                            console.log(`Updating local note from cloud: ${remote.title}`);
                            await invoke('update_note', {
                                id: localMatch.id,
                                title: remote.title,
                                content: remote.content
                            });
                            // Also update timestamp? update_note updates it to NOW.
                        }
                    }
                }

                refreshNotes();

            } catch (e) {
                console.error("Sync Error:", e);
            } finally {
                isSyncingRef.current = false;
            }
        };

        syncList();

        // A. Listen for Local Deletes (App -> Cloud)
        const handleLocalDelete = async (e: CustomEvent) => {
            const { pbId } = e.detail;
            if (pbId) {
                console.log(`Syncing Delete to Cloud: ${pbId}`);
                try {
                    await pb.collection('notes').delete(pbId);
                } catch (err) {
                    console.error("Failed to delete cloud note:", err);
                }
            }
        };
        window.addEventListener('onyx:deleted-note' as any, handleLocalDelete);

        // B. Listen for Local Updates (Creation/Edit)
        const handleLocalTrigger = () => {
            console.log("Sync Triggered by Local Event");
            syncList();
        };
        window.addEventListener('onyx:refresh-notes', handleLocalTrigger);

        // C. Realtime Subscription (Cloud -> Local)
        pb.collection('notes').subscribe('*', async (e) => {
            if (e.action === 'create' || e.action === 'update') {
                console.log("Realtime: List Update");
                syncList();
            } else if (e.action === 'delete') {
                console.log("Realtime: Delete Received", e.record.id);
                try {
                    await invoke('delete_note_by_pb_id', { pbId: e.record.id });
                    // Refresh UI
                    refreshNotes();
                } catch (err) {
                    console.error("Failed to delete local note:", err);
                }
            }
        });

        return () => {
            pb.collection('notes').unsubscribe('*').catch(() => { });
            window.removeEventListener('onyx:refresh-notes', handleLocalTrigger);
            window.removeEventListener('onyx:deleted-note' as any, handleLocalDelete);
        };

    }, [isConnected, user, pb]);
}
