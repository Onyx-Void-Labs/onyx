import { getCurrentWindow } from '@tauri-apps/api/window';
import { Minus, Square, X, PanelLeftClose, PanelLeft, PenLine, MessageCircle, CalendarDays, Mail, Image, Cloud, KeyRound, Settings, ChevronDown, Info, Maximize2, Minimize2 } from 'lucide-react';
import { useState, useRef, useEffect } from 'react';
import { useWorkspace, MODULE_ORDER, MODULES, type WorkspaceModule } from '../../contexts/WorkspaceContext';
import { useSettings } from '../../contexts/SettingsContext';

// Map module IDs → Lucide icon components
const MODULE_ICONS: Record<WorkspaceModule, React.ComponentType<{ size?: number; className?: string }>> = {
    notes: PenLine,
    messages: MessageCircle,
    calendar: CalendarDays,
    email: Mail,
    photos: Image,
    passwords: KeyRound,
    cloud: Cloud,
};

// Accent color mappings
const ACCENT_CLASSES: Record<string, { active: string; indicator: string; text: string; hover: string }> = {
    purple: {
        active: 'bg-purple-500/15 text-purple-300',
        indicator: 'bg-gradient-to-r from-purple-500/50 via-purple-400 to-purple-500/50',
        text: 'text-purple-400',
        hover: 'hover:bg-purple-500/8 hover:text-purple-300',
    },
    blue: {
        active: 'bg-blue-500/15 text-blue-300',
        indicator: 'bg-gradient-to-r from-blue-500/50 via-blue-400 to-blue-500/50',
        text: 'text-blue-400',
        hover: 'hover:bg-blue-500/8 hover:text-blue-300',
    },
    emerald: {
        active: 'bg-emerald-500/15 text-emerald-300',
        indicator: 'bg-gradient-to-r from-emerald-500/50 via-emerald-400 to-emerald-500/50',
        text: 'text-emerald-400',
        hover: 'hover:bg-emerald-500/8 hover:text-emerald-300',
    },
    amber: {
        active: 'bg-amber-500/15 text-amber-300',
        indicator: 'bg-gradient-to-r from-amber-500/50 via-amber-400 to-amber-500/50',
        text: 'text-amber-400',
        hover: 'hover:bg-amber-500/8 hover:text-amber-300',
    },
    rose: {
        active: 'bg-rose-500/15 text-rose-300',
        indicator: 'bg-gradient-to-r from-rose-500/50 via-pink-400 to-rose-500/50',
        text: 'text-rose-400',
        hover: 'hover:bg-rose-500/8 hover:text-rose-300',
    },
    indigo: {
        active: 'bg-indigo-500/15 text-indigo-300',
        indicator: 'bg-gradient-to-r from-indigo-500/50 via-indigo-400 to-indigo-500/50',
        text: 'text-indigo-400',
        hover: 'hover:bg-indigo-500/8 hover:text-indigo-300',
    },
    sky: {
        active: 'bg-sky-500/15 text-sky-300',
        indicator: 'bg-gradient-to-r from-sky-500/50 via-sky-400 to-sky-500/50',
        text: 'text-sky-400',
        hover: 'hover:bg-sky-500/8 hover:text-sky-300',
    },
};

interface TitlebarProps {
    sidebarCollapsed: boolean;
    onToggleSidebar: () => void;
}

export default function Titlebar({ sidebarCollapsed, onToggleSidebar }: TitlebarProps) {
    // Robust detection for Tauri v2 window
    let appWindow: any = null;
    // @ts-ignore
    const isTauri = typeof window !== 'undefined' && !!window.__TAURI_INTERNALS__;

    if (isTauri) {
        try {
            // @ts-ignore
            appWindow = getCurrentWindow();
        } catch {
            // Silent catch - avoiding console spam in browser mode
        }
    }
    const { activeWorkspace, setActiveWorkspace, enabledModules } = useWorkspace();
    const { toggleSettings } = useSettings();
    const activeConfig = MODULES[activeWorkspace];

    const [menuOpen, setMenuOpen] = useState(false);
    const menuRef = useRef<HTMLDivElement>(null);

    const handleMinimize = () => appWindow?.minimize();
    const handleMaximize = () => appWindow?.toggleMaximize();
    const handleClose = () => appWindow?.close();

    // Close menu on outside click
    useEffect(() => {
        if (!menuOpen) return;
        const handleClick = (e: MouseEvent) => {
            if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
                setMenuOpen(false);
            }
        };
        window.addEventListener('mousedown', handleClick);
        return () => window.removeEventListener('mousedown', handleClick);
    }, [menuOpen]);

    // Close menu on Escape
    useEffect(() => {
        if (!menuOpen) return;
        const handleKey = (e: KeyboardEvent) => {
            if (e.key === 'Escape') setMenuOpen(false);
        };
        window.addEventListener('keydown', handleKey);
        return () => window.removeEventListener('keydown', handleKey);
    }, [menuOpen]);

    const visibleModules = MODULE_ORDER.filter(m => enabledModules.includes(m));
    const accent = ACCENT_CLASSES[activeConfig.accentColor];

    return (
        <header
            className="h-11 bg-zinc-900/80 backdrop-blur-md flex items-center justify-between select-none shrink-0 border-b border-zinc-800/50 z-50 relative"
            data-tauri-drag-region
            onDoubleClick={(e) => {
                // Only maximize if clicking the drag region itself (not buttons)
                if ((e.target as HTMLElement).dataset.tauriDragRegion !== undefined) {
                    handleMaximize();
                }
            }}
        >
            {/* LEFT: Sidebar Toggle + ONYX Menu */}
            <div className="flex items-center gap-1 px-3 min-w-[180px]">
                <button
                    onClick={onToggleSidebar}
                    className="p-1.5 rounded-md hover:bg-zinc-800 text-zinc-500 hover:text-zinc-300 transition-colors"
                    title={sidebarCollapsed ? "Show Sidebar" : "Hide Sidebar"}
                >
                    {sidebarCollapsed ? <PanelLeft size={16} /> : <PanelLeftClose size={16} />}
                </button>

                {/* ONYX clickable menu - HIDDEN IN DEMO MODE */}
                {!import.meta.env.VITE_DEMO_MODE && (
                    <div className="relative" ref={menuRef}>
                        <button
                            onClick={() => setMenuOpen(!menuOpen)}
                            className={`flex items-center gap-1 ml-1 px-2 py-1 rounded-lg transition-all duration-150 ${menuOpen
                                ? 'bg-zinc-800 shadow-sm'
                                : 'hover:bg-zinc-800/60'
                                }`}
                        >
                            <span className="text-zinc-100 font-black tracking-widest text-sm">ONYX</span>
                            <span className={`font-bold text-sm transition-colors duration-300 ${accent?.text || 'text-purple-400'}`}>
                                {activeConfig.label.toLowerCase()}
                            </span>
                            <ChevronDown size={12} className={`text-zinc-500 transition-transform duration-200 ${menuOpen ? 'rotate-180' : ''}`} />
                        </button>

                        {/* Dropdown Menu */}
                        {menuOpen && (
                            <div className="absolute top-full left-0 mt-1 w-56 bg-zinc-900 border border-zinc-800/80 rounded-xl shadow-2xl shadow-black/50 overflow-hidden z-[9999] animate-in fade-in slide-in-from-top-1 duration-150">
                                {/* Settings */}
                                <div className="p-1.5 border-b border-zinc-800/50">
                                    <button
                                        onClick={() => { toggleSettings(true); setMenuOpen(false); }}
                                        className="w-full flex items-center gap-3 px-3 py-2 rounded-lg text-sm text-zinc-300 hover:bg-zinc-800/70 transition-colors"
                                    >
                                        <Settings size={15} className="text-zinc-500" />
                                        <span>Settings</span>
                                        <span className="ml-auto text-[10px] text-zinc-600 font-medium">Ctrl+,</span>
                                    </button>
                                </div>

                                {/* Window controls */}
                                <div className="p-1.5 border-b border-zinc-800/50">
                                    <button
                                        onClick={() => { handleMaximize(); setMenuOpen(false); }}
                                        className="w-full flex items-center gap-3 px-3 py-2 rounded-lg text-sm text-zinc-300 hover:bg-zinc-800/70 transition-colors"
                                    >
                                        <Maximize2 size={15} className="text-zinc-500" />
                                        <span>Maximize</span>
                                    </button>
                                    <button
                                        onClick={() => { handleMinimize(); setMenuOpen(false); }}
                                        className="w-full flex items-center gap-3 px-3 py-2 rounded-lg text-sm text-zinc-300 hover:bg-zinc-800/70 transition-colors"
                                    >
                                        <Minimize2 size={15} className="text-zinc-500" />
                                        <span>Minimize</span>
                                    </button>
                                </div>

                                {/* About */}
                                <div className="p-1.5">
                                    <button
                                        onClick={() => { toggleSettings(true); setMenuOpen(false); }}
                                        className="w-full flex items-center gap-3 px-3 py-2 rounded-lg text-sm text-zinc-300 hover:bg-zinc-800/70 transition-colors"
                                    >
                                        <Info size={15} className="text-zinc-500" />
                                        <span>About ONYX</span>
                                    </button>
                                </div>
                            </div>
                        )}
                    </div>
                )}
            </div>

            {/* CENTER: Workspace Tabs */}
            <div className="flex items-center gap-1 flex-1 justify-center">
                {visibleModules
                    .filter(m => !(import.meta.env.VITE_DEMO_MODE && m === 'photos'))
                    .map(moduleId => {
                        const config = MODULES[moduleId];
                        const isActive = activeWorkspace === moduleId;
                        const Icon = MODULE_ICONS[moduleId];
                        const modAccent = ACCENT_CLASSES[config.accentColor];

                        return (
                            <button
                                key={moduleId}
                                onClick={() => setActiveWorkspace(moduleId)}
                                className={`
                                relative flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg text-xs font-semibold
                                transition-all duration-200 ease-out
                                ${isActive
                                        ? `${modAccent.active}`
                                        : `text-zinc-500 ${modAccent.hover}`
                                    }
                            `}
                                title={config.label}
                            >
                                {isActive && (
                                    <div className={`absolute bottom-0 left-2 right-2 h-[2px] ${modAccent.indicator} rounded-full`} />
                                )}
                                <Icon size={14} className={isActive ? modAccent.text : ''} />
                                <span className="tracking-tight hidden sm:inline">{config.label}</span>
                            </button>
                        );
                    })}
            </div>

            {/* RIGHT: Window Controls only — clean, hidden in demo mode */}
            {!import.meta.env.VITE_DEMO_MODE && (
                <div className="flex items-center h-full min-w-[180px] justify-end">
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
            )}
        </header>
    );
}
