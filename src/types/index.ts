// src/types/index.ts

export type BlockType = 'h1' | 'h2' | 'h3' | 'h4' | 'h5' | 'h6' |'p' | 'code' | 'math';

export interface Block {
    id: string;
    type: BlockType;
    content: string;
    properties?: Record<string, any>; // Future-proofing for metadata
}

export interface NoteContent {
    id: number;
    title: string;
    blocks: Block[];
    lastUpdated?: number;
}