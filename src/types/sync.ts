
export interface FileMeta {
    id: string; // UUID
    title: string;
    createdAt: number;
    updatedAt: number;
}

export type SyncStatus = 'connecting' | 'connected' | 'disconnected' | 'offline';
