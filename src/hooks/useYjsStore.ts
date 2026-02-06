import { useEffect, useState } from 'react';
import * as Y from 'yjs';
import { IndexeddbPersistence } from 'y-indexeddb';

export function useYjsStore(docId: string) {
    const [yDoc, setYDoc] = useState<Y.Doc | null>(null);
    const [synced, setSynced] = useState(false);

    useEffect(() => {
        if (!docId) return;

        // 1. Create the Yjs Doc
        const doc = new Y.Doc();

        // 2. Connect to IndexedDB (Offline Persistence)
        // This loads data from browser storage if it exists
        const provider = new IndexeddbPersistence(docId, doc);

        provider.on('synced', () => {
            console.log(`[Yjs] Loaded ${docId} from IndexedDB`);
            setSynced(true);
        });

        setYDoc(doc);

        return () => {
            provider.destroy();
            doc.destroy();
        };
    }, [docId]);

    return { yDoc, synced };
}
