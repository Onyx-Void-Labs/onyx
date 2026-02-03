import { getCurrentWindow } from '@tauri-apps/api/window';
import { Minus, Square, X, PanelLeftClose, PanelLeft } from 'lucide-react';
import { useState, useEffect } from 'react';

interface TitlebarProps {
    sidebarCollapsed: boolean;
    onToggleSidebar: () => void;
}

export default function Titlebar({ sidebarCollapsed, onToggleSidebar }: TitlebarProps) {
    const [isMaximized, setIsMaximized] = useState(false);
    const appWindow = getCurrentWindow();

    useEffect(() => {
        const checkMaximized = async () => {
            setIsMaximized(await appWindow.isMaximized());
        };
        checkMaximized();
    }, [appWindow]);

    const handleMinimize = () => appWindow.minimize();
    const handleMaximize = async () => {
        await appWindow.toggleMaximize();
        setIsMaximized(await appWindow.isMaximized());
    };
    const handleClose = () => appWindow.close();

    const startDrag = (e: React.MouseEvent) => {
        if (e.button === 0 && (e.target as HTMLElement).dataset.tauriDragRegion) {
            appWindow.startDragging();
        }
    };

    const handleDoubleClick = (e: React.MouseEvent) => {
        // Double-click on drag region to maximize
        if ((e.target as HTMLElement).dataset.tauriDragRegion) {
            handleMaximize();
        }
    };

    return (
        <header
            className="h-10 bg-zinc-900/80 backdrop-blur-md flex items-center justify-between select-none shrink-0 border-b border-zinc-800/50"
            onMouseDown={startDrag}
            onDoubleClick={handleDoubleClick}
            data-tauri-drag-region
        >
            {/* LEFT: Sidebar Toggle + Branding */}
            <div className="flex items-center gap-2 px-3">
                <button
                    onClick={onToggleSidebar}
                    className="p-1.5 rounded-md hover:bg-zinc-800 text-zinc-500 hover:text-zinc-300 transition-colors"
                    title={sidebarCollapsed ? "Show Sidebar" : "Hide Sidebar"}
                >
                    {sidebarCollapsed ? <PanelLeft size={16} /> : <PanelLeftClose size={16} />}
                </button>

                <div className="flex items-center gap-1 ml-2" data-tauri-drag-region>
                    <span className="text-zinc-100 font-black tracking-widest text-sm">ONYX</span>
                    <span className="text-purple-400 font-bold text-sm">notes</span>
                </div>
            </div>

            {/* RIGHT: Window Controls - Edge-aligned for easy mouse flicking */}
            <div className="flex items-center h-full">
                <button
                    onClick={handleMinimize}
                    className="h-full w-12 flex items-center justify-center hover:bg-zinc-700/60 text-zinc-400 hover:text-zinc-100 transition-colors"
                >
                    <Minus size={14} />
                </button>
                <button
                    onClick={handleMaximize}
                    className="h-full w-12 flex items-center justify-center hover:bg-zinc-700/60 text-zinc-400 hover:text-zinc-100 transition-colors"
                >
                    <Square size={12} />
                </button>
                <button
                    onClick={handleClose}
                    className="h-full w-12 flex items-center justify-center hover:bg-red-500 text-zinc-400 hover:text-white transition-colors"
                >
                    <X size={16} />
                </button>
            </div>
        </header>
    );
}
