import { Settings, Mic, MicOff, Headphones } from 'lucide-react';
import { useState } from 'react';
import { pb } from '../../lib/pocketbase';
import { useSettings } from '../../contexts/SettingsContext';
import { useWorkspace, MODULES } from '../../contexts/WorkspaceContext';

// Accent color for the status dot
const STATUS_COLORS: Record<string, string> = {
    purple: 'bg-purple-400',
    blue: 'bg-blue-400',
    emerald: 'bg-emerald-400',
    amber: 'bg-amber-400',
    rose: 'bg-rose-400',
    indigo: 'bg-indigo-400',
    sky: 'bg-sky-400',
};

interface AccountPanelProps {
    user: any;
    onOpenAuth: () => void;
}

export default function AccountPanel({ user, onOpenAuth }: AccountPanelProps) {
    const { toggleSettings } = useSettings();
    const { activeWorkspace } = useWorkspace();
    const activeConfig = MODULES[activeWorkspace];
    const [muted, setMuted] = useState(false);

    const statusColor = STATUS_COLORS[activeConfig.accentColor] || 'bg-purple-400';

    // Get display name and email
    const displayName = user?.name || user?.username || 'Guest';
    const email = user?.email || '';
    const avatarUrl = user?.avatar
        ? pb.files.getURL(user, user.avatar, { thumb: '64x64' })
        : null;

    // Truncate email for display
    const shortEmail = email.length > 20 ? email.slice(0, 18) + '…' : email;

    if (!user) {
        return (
            <div className="h-[52px] bg-zinc-900/90 border-t border-zinc-800/50 px-3 flex items-center">
                <button
                    onClick={onOpenAuth}
                    className="w-full flex items-center justify-center gap-2 px-3 py-1.5 rounded-lg bg-purple-500/15 text-purple-400 hover:bg-purple-500/20 transition-colors text-xs font-semibold"
                >
                    Sign in to sync
                </button>
            </div>
        );
    }

    return (
        <div className="h-[52px] bg-zinc-900/90 border-t border-zinc-800/50 px-2 flex items-center gap-1.5 shrink-0">
            {/* Avatar + Info */}
            <button
                onClick={() => toggleSettings(true)}
                className="flex items-center gap-2 flex-1 min-w-0 px-1.5 py-1 rounded-lg hover:bg-zinc-800/60 transition-colors group"
            >
                {/* Avatar */}
                <div className="relative shrink-0">
                    {avatarUrl ? (
                        <img
                            src={avatarUrl}
                            alt=""
                            className="w-8 h-8 rounded-full object-cover"
                        />
                    ) : (
                        <div className="w-8 h-8 rounded-full bg-zinc-800 flex items-center justify-center text-xs font-bold text-zinc-400">
                            {displayName.charAt(0).toUpperCase()}
                        </div>
                    )}
                    {/* Status dot */}
                    <div className={`absolute -bottom-0.5 -right-0.5 w-3 h-3 rounded-full ${statusColor} border-2 border-zinc-900`} />
                </div>

                {/* Name + Status */}
                <div className="flex flex-col items-start min-w-0">
                    <span className="text-xs font-semibold text-zinc-200 truncate max-w-[100px]">
                        {displayName}
                    </span>
                    <span className="text-[10px] text-zinc-500 truncate max-w-[100px]">
                        {shortEmail || 'Online'}
                    </span>
                </div>
            </button>

            {/* Quick Actions */}
            <div className="flex items-center gap-0.5 shrink-0">
                <button
                    onClick={() => setMuted(!muted)}
                    className={`p-1.5 rounded-md transition-colors ${muted
                        ? 'text-red-400 hover:bg-red-500/15'
                        : 'text-zinc-500 hover:bg-zinc-800 hover:text-zinc-300'
                        }`}
                    title={muted ? 'Unmute' : 'Mute'}
                >
                    {muted ? <MicOff size={15} /> : <Mic size={15} />}
                </button>
                <button
                    className="p-1.5 rounded-md text-zinc-500 hover:bg-zinc-800 hover:text-zinc-300 transition-colors"
                    title="Deafen"
                >
                    <Headphones size={15} />
                </button>
                <button
                    onClick={() => toggleSettings(true)}
                    className="p-1.5 rounded-md text-zinc-500 hover:bg-zinc-800 hover:text-zinc-300 transition-colors"
                    title="Settings"
                >
                    <Settings size={15} />
                </button>
            </div>
        </div>
    );
}
