import { X, FileText } from "lucide-react";
import { useEffect, useRef, useState, useCallback, useLayoutEffect } from "react";

interface TabBarProps {
    tabs: number[];
    activeTabId: number | null;
    onSelectTab: (id: number) => void;
    onCloseTab: (id: number) => void;
    onReorderTabs: (fromIndex: number, toIndex: number) => void;
    notes: any[];
}

export default function TabBar({ tabs, activeTabId, onSelectTab, onCloseTab, onReorderTabs, notes }: TabBarProps) {
    const containerRef = useRef<HTMLDivElement>(null);
    const activeTabRef = useRef<HTMLDivElement>(null);
    const [dragState, setDragState] = useState<{
        dragIndex: number;
        startX: number;
        currentX: number;
        originalTabId: number; // Track ID to identify settling tab
    } | null>(null);

    // Settling state for the dropped tab
    const [settlingState, setSettlingState] = useState<{
        tabId: number;
        startOffset: number;
    } | null>(null);

    const wasDragging = useRef(false);
    const tabRefs = useRef<(HTMLDivElement | null)[]>([]);

    useEffect(() => {
        if (activeTabRef.current && containerRef.current && !dragState) {
            activeTabRef.current.scrollIntoView({ behavior: 'smooth', block: 'nearest', inline: 'nearest' });
        }
    }, [activeTabId, dragState]);

    const getTabWidth = useCallback(() => {
        const firstTab = tabRefs.current[0];
        return firstTab ? firstTab.offsetWidth : 150;
    }, []);

    const getHoverIndex = useCallback((dragOffset: number, dragIndex: number, tabWidth: number): number => {
        const gap = 6;
        const threshold = (tabWidth + gap) * 0.6;

        if (dragOffset > threshold) {
            const steps = Math.floor((dragOffset - threshold) / (tabWidth + gap)) + 1;
            return Math.min(tabs.length - 1, dragIndex + steps);
        } else if (dragOffset < -threshold) {
            const steps = Math.floor((-dragOffset - threshold) / (tabWidth + gap)) + 1;
            return Math.max(0, dragIndex - steps);
        }
        return dragIndex;
    }, [tabs.length]);

    // Use layout effect to trigger the settle animation
    useLayoutEffect(() => {
        if (settlingState) {
            // Force reflow to ensure start position is applied? 
            // React batching might handle this, but let's clear it in the next tick just to be safe
            // Actually, simply clearing it in a timeout works best for transitions
            requestAnimationFrame(() => {
                setSettlingState(null);
            });
        }
    }, [settlingState]);

    useEffect(() => {
        if (!dragState) return;

        const handleMouseMove = (e: MouseEvent) => {
            setDragState(prev => prev ? { ...prev, currentX: e.clientX } : null);
        };

        const handleMouseUp = (e: MouseEvent) => {
            if (dragState) {
                const offset = e.clientX - dragState.startX;
                if (Math.abs(offset) > 5) {
                    wasDragging.current = true;
                }
                const tabWidth = getTabWidth();
                const dropIndex = getHoverIndex(offset, dragState.dragIndex, tabWidth);

                if (dropIndex !== dragState.dragIndex) {
                    // Calculate where the tab is VISUALLY relative to its new slot
                    // Visual Pos = OldSlot + Offset
                    // New Slot = OldSlot + (dropIndex - dragIndex) * (Width + Gap)
                    // Relative Start = Visual Pos - New Slot
                    //                = Offset - Shift
                    const gap = 6;
                    const shift = (dropIndex - dragState.dragIndex) * (tabWidth + gap);
                    const settleOffset = offset - shift;

                    setSettlingState({
                        tabId: dragState.originalTabId,
                        startOffset: settleOffset
                    });

                    onReorderTabs(dragState.dragIndex, dropIndex);
                } else {
                    // Dropped in same place, just settle from current offset
                    setSettlingState({
                        tabId: dragState.originalTabId,
                        startOffset: offset
                    });
                }
            }
            setDragState(null);
        };

        window.addEventListener('mousemove', handleMouseMove);
        window.addEventListener('mouseup', handleMouseUp);
        return () => {
            window.removeEventListener('mousemove', handleMouseMove);
            window.removeEventListener('mouseup', handleMouseUp);
        };
    }, [dragState, getHoverIndex, onReorderTabs, getTabWidth]);

    if (tabs.length === 0) return null;

    const manyTabs = tabs.length > 6;

    const handleMouseDown = (e: React.MouseEvent, index: number, tabId: number) => {
        wasDragging.current = false; // Reset drag state on new interaction
        if (e.button === 0 && tabs.length > 1) {
            e.preventDefault();
            setDragState({
                dragIndex: index,
                startX: e.clientX,
                currentX: e.clientX,
                originalTabId: tabId
            });
        }
        if (e.button === 1) {
            e.preventDefault();
            onCloseTab(tabs[index]);
        }
    };

    const dragOffset = dragState ? dragState.currentX - dragState.startX : 0;
    const tabWidth = getTabWidth();
    const gap = 6;
    const hoverIndex = dragState ? getHoverIndex(dragOffset, dragState.dragIndex, tabWidth) : -1;

    return (
        <div
            ref={containerRef}
            className="flex bg-gradient-to-r from-zinc-900/50 to-zinc-950/50 h-11 items-center px-3 gap-1.5 overflow-x-auto no-scrollbar border-b border-zinc-800/30 relative"
        >
            {tabs.map((tabId, index) => {
                const note = notes.find((n) => n.id === tabId);
                const isActive = activeTabId === tabId;
                const isDragging = dragState?.dragIndex === index;
                const isSettling = settlingState?.tabId === tabId;

                let translateX = 0;

                if (isDragging) {
                    translateX = dragOffset;
                } else if (isSettling) {
                    translateX = settlingState.startOffset;
                } else if (dragState) {
                    // Shift other tabs
                    const { dragIndex } = dragState;
                    const shift = tabWidth + gap;
                    if (hoverIndex < dragIndex && index >= hoverIndex && index < dragIndex) {
                        translateX = shift;
                    } else if (hoverIndex > dragIndex && index > dragIndex && index <= hoverIndex) {
                        translateX = -shift;
                    }
                }

                return (
                    <div
                        key={tabId}
                        ref={(el) => {
                            tabRefs.current[index] = el;
                            if (isActive && !isDragging) (activeTabRef as any).current = el;
                        }}
                        onMouseDown={(e) => handleMouseDown(e, index, tabId)}
                        onAuxClick={(e) => e.preventDefault()}
                        onClick={() => {
                            if (wasDragging.current) {
                                wasDragging.current = false;
                                return;
                            }
                            onSelectTab(tabId);
                        }}
                        style={{
                            transform: `translateX(${translateX}px)`,
                            zIndex: isDragging ? 100 : 1,
                            // If settling, we want to animate from startOffset to 0 (which happens when settlingState is cleared)
                            // But wait, if settleState is Set, translateX IS the offset.
                            // When we clear it to null, translateX becomes 0.
                            // So we need transition enabled when (isSettling is FALSE but WAS true)?? 
                            // Actually, simpler: Always enable transition, disable ONLY when dragging.
                            // When settlingState is active, it holds the pos.
                            // When cleared, it goes to 0 with transition.
                            // BUT react renders are discrete.
                            // Render 1: isSettling=true, x=offset. Transition=none (to set start pos)?
                            // Render 2 (next frame): isSettling=false, x=0. Transition=active.
                            transition: (isDragging || settlingState) ? 'none' : 'transform 200ms cubic-bezier(0.2, 0, 0, 1)',
                        }}
                        className={`
                            flex items-center gap-2 px-3 py-2
                            rounded-lg shrink-0 select-none
                            ${manyTabs ? 'min-w-28 max-w-40' : 'min-w-36 max-w-56'}
                            group relative cursor-pointer
                            ${isActive
                                ? "bg-zinc-800/80 text-zinc-100 shadow-lg shadow-black/20"
                                : "text-zinc-500 hover:bg-zinc-800/40 hover:text-zinc-300"}
                        `}
                    >
                        {isActive && (
                            <div className="absolute bottom-0 left-3 right-3 h-0.5 bg-gradient-to-r from-purple-500/50 via-purple-400 to-purple-500/50 rounded-full" />
                        )}

                        <FileText size={14} className={`shrink-0 ${isActive ? 'text-purple-400' : 'text-zinc-600'}`} />

                        <span className="text-sm font-medium truncate flex-1 tracking-tight">
                            {note?.title || "Untitled"}
                        </span>

                        <button
                            onClick={(e) => { e.stopPropagation(); onCloseTab(tabId); }}
                            onMouseDown={(e) => e.stopPropagation()}
                            className={`p-0.5 rounded-md transition-all shrink-0 ${isActive ? "hover:bg-zinc-700" : "hover:bg-zinc-700/50"} opacity-0 group-hover:opacity-100`}
                        >
                            <X size={12} />
                        </button>
                    </div>
                );
            })}
        </div>
    );
}
