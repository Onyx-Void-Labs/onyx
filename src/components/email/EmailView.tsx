import { Inbox, Send, Star, Trash2, Archive, Clock, Paperclip, Search, MailPlus, Shield } from 'lucide-react';

// ─── Email View ──────────────────────────────────────────────────────────────

const FOLDERS = [
    { icon: Inbox, label: 'Inbox', count: 3, active: true },
    { icon: Star, label: 'Starred', count: 0 },
    { icon: Send, label: 'Sent', count: 0 },
    { icon: Clock, label: 'Drafts', count: 1 },
    { icon: Archive, label: 'Archive', count: 0 },
    { icon: Trash2, label: 'Trash', count: 0 },
];

const SAMPLE_EMAILS = [
    {
        from: 'Security Team',
        subject: 'Your encryption keys have been rotated',
        preview: 'All your data remains fully encrypted with the new keys...',
        time: '2m ago',
        unread: true,
        hasAttachment: false,
    },
    {
        from: 'Onyx Cloud',
        subject: 'Storage upgrade confirmed',
        preview: 'Your plan has been upgraded to 200GB. Enjoy the extra space...',
        time: '1h ago',
        unread: true,
        hasAttachment: true,
    },
    {
        from: 'Calendar',
        subject: 'Reminder: Design Review at 2:00 PM',
        preview: 'You have an upcoming event in 30 minutes...',
        time: '3h ago',
        unread: false,
        hasAttachment: false,
    },
];

export default function EmailView() {
    return (
        <div className="flex h-full overflow-hidden">
            {/* Folder Sidebar */}
            <div className="w-56 bg-zinc-900/60 flex flex-col border-r border-zinc-800/30 shrink-0">
                <div className="p-3 shrink-0">
                    <button className="w-full flex items-center justify-center gap-2 px-3 py-2.5 rounded-xl bg-amber-500/15 text-amber-400 hover:bg-amber-500/20 transition-colors text-sm font-semibold">
                        <MailPlus size={16} />
                        Compose
                    </button>
                </div>

                <div className="flex-1 overflow-y-auto px-2 space-y-0.5">
                    {FOLDERS.map(folder => (
                        <button
                            key={folder.label}
                            className={`w-full flex items-center gap-3 px-3 py-2 rounded-lg text-sm transition-colors ${folder.active
                                    ? 'bg-zinc-800/60 text-zinc-100'
                                    : 'text-zinc-500 hover:text-zinc-300 hover:bg-zinc-800/30'
                                }`}
                        >
                            <folder.icon size={16} className={folder.active ? 'text-amber-400' : 'text-zinc-600'} />
                            <span className="flex-1 text-left truncate">{folder.label}</span>
                            {folder.count > 0 && (
                                <span className={`text-xs font-medium px-1.5 py-0.5 rounded-full ${folder.active ? 'bg-amber-500/20 text-amber-400' : 'text-zinc-600'
                                    }`}>
                                    {folder.count}
                                </span>
                            )}
                        </button>
                    ))}
                </div>
            </div>

            {/* Email List */}
            <div className="w-80 bg-zinc-900/30 flex flex-col border-r border-zinc-800/30 shrink-0">
                {/* Search */}
                <div className="p-3 border-b border-zinc-800/30 shrink-0">
                    <div className="flex items-center gap-2 bg-zinc-800/30 rounded-lg px-3 py-2 border border-zinc-700/20">
                        <Search size={14} className="text-zinc-600" />
                        <input
                            type="text"
                            placeholder="Search emails..."
                            className="flex-1 bg-transparent text-sm text-zinc-300 placeholder-zinc-600 outline-none"
                            disabled
                        />
                    </div>
                </div>

                {/* Email items */}
                <div className="flex-1 overflow-y-auto">
                    {SAMPLE_EMAILS.map((email, i) => (
                        <div
                            key={i}
                            className={`px-4 py-3 border-b border-zinc-800/20 cursor-pointer transition-colors ${i === 0
                                    ? 'bg-amber-500/5 border-l-2 border-l-amber-400'
                                    : 'hover:bg-zinc-800/20 border-l-2 border-l-transparent'
                                }`}
                        >
                            <div className="flex items-center justify-between mb-0.5">
                                <span className={`text-sm truncate ${email.unread ? 'font-semibold text-zinc-100' : 'text-zinc-400'}`}>
                                    {email.from}
                                </span>
                                <span className="text-xs text-zinc-600 shrink-0 ml-2">{email.time}</span>
                            </div>
                            <div className="flex items-center gap-1.5">
                                <span className={`text-sm truncate ${email.unread ? 'text-zinc-300' : 'text-zinc-500'}`}>
                                    {email.subject}
                                </span>
                                {email.hasAttachment && <Paperclip size={12} className="text-zinc-600 shrink-0" />}
                            </div>
                            <p className="text-xs text-zinc-600 truncate mt-0.5">{email.preview}</p>
                        </div>
                    ))}
                </div>
            </div>

            {/* Reading Pane — Coming Soon */}
            <div className="flex-1 flex items-center justify-center bg-zinc-950/50">
                <div className="text-center space-y-4 max-w-sm">
                    <div className="w-20 h-20 rounded-2xl bg-amber-500/10 border border-amber-500/20 flex items-center justify-center mx-auto">
                        <Shield size={32} className="text-amber-400" />
                    </div>
                    <div>
                        <h3 className="text-lg font-bold text-zinc-100 mb-1">Private Email</h3>
                        <p className="text-sm text-zinc-500 leading-relaxed">
                            IMAP & SMTP with client-side encryption via Rust backend. Your inbox, truly yours.
                        </p>
                    </div>
                    <div className="inline-flex items-center gap-2 px-3 py-1.5 rounded-full bg-amber-500/10 border border-amber-500/20 text-xs font-medium text-amber-400">
                        <div className="w-1.5 h-1.5 rounded-full bg-amber-400 animate-pulse" />
                        Coming Soon
                    </div>
                </div>
            </div>
        </div>
    );
}
