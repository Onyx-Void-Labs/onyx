import { createContext, useContext, useState, useCallback, ReactNode } from 'react';

// ─── Module Definitions ─────────────────────────────────────────────────────
export type WorkspaceModule = 'notes' | 'messages' | 'calendar' | 'email' | 'photos' | 'passwords' | 'cloud';

export interface ModuleConfig {
    id: WorkspaceModule;
    label: string;
    icon: string; // lucide icon name reference
    accentColor: string; // tailwind color class
    accentGradient: string; // gradient for active indicator
    description: string;
}

export const MODULES: Record<WorkspaceModule, ModuleConfig> = {
    notes: {
        id: 'notes',
        label: 'Notes',
        icon: 'pen-line',
        accentColor: 'purple',
        accentGradient: 'from-purple-500 via-purple-400 to-violet-500',
        description: 'Write, organize, and encrypt your notes',
    },
    messages: {
        id: 'messages',
        label: 'Messages',
        icon: 'message-circle',
        accentColor: 'blue',
        accentGradient: 'from-blue-500 via-blue-400 to-cyan-500',
        description: 'End-to-end encrypted messaging',
    },
    calendar: {
        id: 'calendar',
        label: 'Calendar',
        icon: 'calendar-days',
        accentColor: 'emerald',
        accentGradient: 'from-emerald-500 via-emerald-400 to-teal-500',
        description: 'Schedule and manage your events',
    },
    email: {
        id: 'email',
        label: 'Email',
        icon: 'mail',
        accentColor: 'amber',
        accentGradient: 'from-amber-500 via-amber-400 to-orange-500',
        description: 'Send and receive encrypted email',
    },
    photos: {
        id: 'photos',
        label: 'Photos',
        icon: 'image',
        accentColor: 'rose',
        accentGradient: 'from-rose-500 via-pink-400 to-rose-500',
        description: 'Store and browse your photos securely',
    },
    passwords: {
        id: 'passwords',
        label: 'Vault',
        icon: 'key-round',
        accentColor: 'indigo',
        accentGradient: 'from-indigo-500 via-indigo-400 to-violet-500',
        description: 'Encrypted password manager and secure vault',
    },
    cloud: {
        id: 'cloud',
        label: 'Cloud',
        icon: 'cloud',
        accentColor: 'sky',
        accentGradient: 'from-sky-500 via-sky-400 to-blue-500',
        description: 'Encrypted cloud storage up to 200GB',
    },
};

export const MODULE_ORDER: WorkspaceModule[] = ['notes', 'messages', 'calendar', 'email', 'photos', 'passwords', 'cloud'];

// ─── Context ─────────────────────────────────────────────────────────────────

interface WorkspaceContextValue {
    activeWorkspace: WorkspaceModule;
    setActiveWorkspace: (module: WorkspaceModule) => void;
    enabledModules: WorkspaceModule[];
    toggleModule: (module: WorkspaceModule) => void;
    isModuleEnabled: (module: WorkspaceModule) => boolean;
    getActiveConfig: () => ModuleConfig;
}

const WorkspaceContext = createContext<WorkspaceContextValue | null>(null);

// ─── Provider ────────────────────────────────────────────────────────────────

export function WorkspaceProvider({ children }: { children: ReactNode }) {
    const [activeWorkspace, setActiveWorkspaceState] = useState<WorkspaceModule>(() => {
        const saved = localStorage.getItem('onyx-active-workspace');
        return (saved as WorkspaceModule) || 'notes';
    });

    const [enabledModules, setEnabledModules] = useState<WorkspaceModule[]>(() => {
        const saved = localStorage.getItem('onyx-enabled-modules');
        if (saved) {
            try {
                return JSON.parse(saved);
            } catch {
                return [...MODULE_ORDER]; // All enabled by default
            }
        }
        return [...MODULE_ORDER]; // All enabled by default
    });

    const setActiveWorkspace = useCallback((module: WorkspaceModule) => {
        setActiveWorkspaceState(module);
        localStorage.setItem('onyx-active-workspace', module);
    }, []);

    const toggleModule = useCallback((module: WorkspaceModule) => {
        // Notes can never be disabled
        if (module === 'notes') return;

        setEnabledModules(prev => {
            const next = prev.includes(module)
                ? prev.filter(m => m !== module)
                : [...prev, module];
            localStorage.setItem('onyx-enabled-modules', JSON.stringify(next));
            return next;
        });
    }, []);

    const isModuleEnabled = useCallback((module: WorkspaceModule) => {
        return enabledModules.includes(module);
    }, [enabledModules]);

    const getActiveConfig = useCallback(() => {
        return MODULES[activeWorkspace];
    }, [activeWorkspace]);

    return (
        <WorkspaceContext.Provider value={{
            activeWorkspace,
            setActiveWorkspace,
            enabledModules,
            toggleModule,
            isModuleEnabled,
            getActiveConfig,
        }}>
            {children}
        </WorkspaceContext.Provider>
    );
}

// ─── Hook ────────────────────────────────────────────────────────────────────

export function useWorkspace() {
    const ctx = useContext(WorkspaceContext);
    if (!ctx) throw new Error('useWorkspace must be used within WorkspaceProvider');
    return ctx;
}
