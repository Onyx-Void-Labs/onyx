

export default function KeybindsTab() {
    return (
        <div className="space-y-6 animate-in fade-in slide-in-from-bottom-4 duration-500">
            <h2 className="text-2xl font-bold text-white mb-1">Keyboard Shortcuts</h2>
            <p className="text-zinc-400 text-sm">Customize your workflow efficiency.</p>

            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                {/* Editor Shortcuts */}
                <div className="p-4 bg-zinc-900/50 border border-white/5 rounded-2xl space-y-3">
                    <h3 className="text-zinc-400 text-sm font-semibold uppercase tracking-wider mb-2">Editor</h3>
                    <Shortcut label="Bold" keys={['Mod', 'B']} />
                    <Shortcut label="Italic" keys={['Mod', 'I']} />
                    <Shortcut label="Underline" keys={['Mod', 'U']} />
                    <Shortcut label="Inline Code" keys={['Mod', '`']} />
                    <Shortcut label="Inline Math" keys={['Mod', 'M']} />
                    <Shortcut label="Block Math" keys={['Mod', 'Shift', 'M']} />
                    <Shortcut label="Find" keys={['Mod', 'F']} />
                </div>

                {/* Navigation & General */}
                <div className="p-4 bg-zinc-900/50 border border-white/5 rounded-2xl space-y-3">
                    <h3 className="text-zinc-400 text-sm font-semibold uppercase tracking-wider mb-2">General</h3>
                    <Shortcut label="Quick Search" keys={['Mod', 'P']} />
                    <Shortcut label="Settings" keys={['Mod', ',']} />
                    <Shortcut label="Toggle Sidebar" keys={['Mod', '\\']} />
                    <Shortcut label="New Note" keys={['Mod', 'N']} />
                    <Shortcut label="Close Tab" keys={['Mod', 'W']} />
                    <Shortcut label="Next Tab" keys={['Ctrl', 'Tab']} />
                    <Shortcut label="Prev Tab" keys={['Ctrl', 'Shift', 'Tab']} />
                </div>
            </div>
        </div>
    );
}

function Shortcut({ label, keys }: { label: string, keys: string[] }) {
    return (
        <div className="flex items-center justify-between">
            <span className="text-zinc-300 text-sm">{label}</span>
            <div className="flex items-center gap-1">
                {keys.map((k, i) => (
                    <kbd key={i} className="px-2 py-1 bg-zinc-800 rounded-md text-[10px] text-zinc-400 font-mono border border-white/5 min-w-[24px] text-center">
                        {k === 'Mod' ? 'Ctrl' : k}
                    </kbd>
                ))}
            </div>
        </div>
    );
}
