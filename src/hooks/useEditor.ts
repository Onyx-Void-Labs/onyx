import { useState, useCallback } from 'react';
import { Block, BlockType } from '../types';

export const useEditor = (initialBlocks: Block[]) => {
    const [blocks, setBlocks] = useState<Block[]>(initialBlocks);

    // HISTORY STACK (Undo/Redo)
    const [history, setHistory] = useState<Block[][]>([]);
    const [future, setFuture] = useState<Block[][]>([]);

    const undo = useCallback(() => {
        if (history.length === 0) return;
        const previous = history[history.length - 1];
        const newHistory = history.slice(0, -1);

        setFuture(prev => [blocks, ...prev]);
        setBlocks(previous);
        setHistory(newHistory);
    }, [blocks, history]);

    const redo = useCallback(() => {
        if (future.length === 0) return;
        const next = future[0];
        const newFuture = future.slice(1);

        setHistory(prev => [...prev, blocks]);
        setBlocks(next);
        setFuture(newFuture);
    }, [blocks, future]);

    // CORE OPERATIONS
    const updateBlock = useCallback((id: string, content: string, type?: BlockType, shouldPushHistory = false) => {
        if (shouldPushHistory) {
            setHistory(prev => [...prev, blocks]);
            setFuture([]);
        }

        setBlocks(prev => prev.map(b => b.id === id ? { ...b, content, type: type || b.type } : b));
    }, [blocks]);

    const addBlock = useCallback((afterId: string | null, type: BlockType = 'p', content: string = '') => {
        setHistory(prev => [...prev, blocks]);
        setFuture([]);

        const newBlock: Block = { id: crypto.randomUUID(), type, content };
        setBlocks(prev => {
            if (!afterId) return [...prev, newBlock];
            const index = prev.findIndex(b => b.id === afterId);
            if (index === -1) return prev;
            const newBlocks = [...prev];
            newBlocks.splice(index + 1, 0, newBlock);
            return newBlocks;
        });
        return newBlock.id;
    }, [blocks]);

    const removeBlock = useCallback((id: string) => {
        setHistory(prev => [...prev, blocks]);
        setFuture([]);
        setBlocks(prev => prev.filter(b => b.id !== id));
    }, [blocks]);

    // MERGE & SPLIT
    const splitBlock = useCallback((id: string, cursorIndex: number) => {
        setHistory(prev => [...prev, blocks]);
        setFuture([]);

        const newId = crypto.randomUUID();

        setBlocks(prev => {
            const index = prev.findIndex(b => b.id === id);
            if (index === -1) return prev;
            const block = prev[index];
            const contentBefore = block.content.slice(0, cursorIndex);
            const contentAfter = block.content.slice(cursorIndex);

            const newBlock: Block = { id: newId, type: 'p', content: contentAfter };

            const newBlocks = [...prev];
            newBlocks[index] = { ...block, content: contentBefore };
            newBlocks.splice(index + 1, 0, newBlock);
            return newBlocks;
        });
        return newId;
    }, [blocks]);

    const mergeBlock = useCallback((id: string, direction: 'up' | 'down') => {
        const index = blocks.findIndex(b => b.id === id);
        if (index === -1) return "";

        const targetIndex = direction === 'up' ? index - 1 : index + 1;
        if (targetIndex < 0 || targetIndex >= blocks.length) return "";

        const current = blocks[index];
        const target = blocks[targetIndex];

        setHistory(prev => [...prev, blocks]);
        setFuture([]);

        let focusId = "";
        const newBlocks = [...blocks];

        if (direction === 'up') {
            newBlocks[targetIndex] = { ...target, content: target.content + current.content };
            newBlocks.splice(index, 1);
            focusId = target.id;
        } else {
            newBlocks[index] = { ...current, content: current.content + target.content };
            newBlocks.splice(targetIndex, 1);
            focusId = current.id;
        }

        setBlocks(newBlocks);
        return focusId;
    }, [blocks]);

    // PASTE HANDLER
    const handlePaste = useCallback((id: string, text: string, cursorIndex: number) => {
        setHistory(prev => [...prev, blocks]);
        setFuture([]);

        const paragraphs = text.split(/\r?\n\r?\n/).filter(p => p.trim());
        if (paragraphs.length <= 1) return false;

        setBlocks(prev => {
            const index = prev.findIndex(b => b.id === id);
            if (index === -1) return prev;
            const block = prev[index];

            const before = block.content.slice(0, cursorIndex);
            const after = block.content.slice(cursorIndex);

            const newBlocks = [...prev];
            newBlocks[index] = { ...block, content: before + paragraphs[0] };

            for (let i = 1; i < paragraphs.length - 1; i++) {
                const newId = crypto.randomUUID();
                newBlocks.splice(index + i, 0, { id: newId, type: 'p', content: paragraphs[i] });
            }

            const finalId = crypto.randomUUID();
            newBlocks.splice(index + paragraphs.length - 1, 0, {
                id: finalId,
                type: 'p',
                content: paragraphs[paragraphs.length - 1] + after
            });

            return newBlocks;
        });
        return true;
    }, [blocks]);

    return {
        blocks,
        setBlocks,
        updateBlock,
        addBlock,
        removeBlock,
        splitBlock,
        mergeBlock,
        undo,
        redo,
        history,
        future,
        handlePaste
    };
};
