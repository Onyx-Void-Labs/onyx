import { User, Palette, Code, Database, Zap, Info, Cpu, Key, Shield } from 'lucide-react';

interface SettingsSidebarProps {
    activeTab: string;
    setActiveTab: (tab: string) => void;
    user: any; // Add user prop
}

export default function SettingsSidebar({ activeTab, setActiveTab, user }: SettingsSidebarProps) {

    // Define sections dynamically based on user state
    const sections = [
        {
            label: 'General',
            items: [
                {
                    id: 'account',
                    label: user ? 'Cloud & Plan' : 'Account',
                    icon: User
                },
                // Only show Security if authenticated
                ...(user ? [{ id: 'security', label: 'Security & Privacy', icon: Shield }] : []),
            ]
        },
        {
            label: 'Look & Feel',
            items: [
                { id: 'appearance', label: 'Appearance', icon: Palette },
            ]
        },
        {
            label: 'Workspace',
            items: [
                { id: 'editor', label: 'Editor', icon: Code },
                { id: 'shortcuts', label: 'Keybinds', icon: Key },
                { id: 'features', label: 'Features', icon: Zap },
                { id: 'storage', label: 'Data', icon: Database },
            ]
        },
    ];

    // Bottom section for Info/About
    const bottomItems = [
        { id: 'about', label: 'About', icon: Info }
    ];

    return (
        <div className="w-64 bg-zinc-900/30 border-r border-white/5 flex flex-col p-2 h-full">
            <div className="flex items-center gap-3 px-4 mb-6 mt-4">
                <div className="w-8 h-8 bg-purple-500/10 rounded-xl flex items-center justify-center text-purple-400">
                    <Cpu size={18} />
                </div>
                <div>
                    <div className="font-bold text-zinc-200 tracking-tight">Onyx Config</div>
                    <div className="text-[10px] uppercase font-bold text-zinc-600">v0.0.3-alpha</div>
                </div>
            </div>

            <div className="flex-1 overflow-y-auto space-y-6 custom-scrollbar px-2">
                {sections.map((cat, i) => (
                    <div key={i} className="space-y-1">
                        <div className="px-4 text-[10px] uppercase tracking-widest font-bold text-zinc-600 mb-2">{cat.label}</div>
                        {cat.items.map((item) => (
                            <button
                                key={item.id}
                                onClick={() => setActiveTab(item.id)}
                                className={`w-full flex items-center gap-3 px-4 py-2.5 rounded-xl text-sm font-medium transition-all duration-200 group active:scale-95 ${activeTab === item.id
                                    ? 'bg-purple-500/10 text-purple-400 shadow-sm'
                                    : 'text-zinc-400 hover:text-zinc-200 hover:bg-white/5'
                                    }`}
                            >
                                <item.icon size={16} className={`${activeTab === item.id ? "text-purple-400" : "opacity-70"} transition-transform group-hover:scale-110`} />
                                <span>{item.label}</span>
                            </button>
                        ))}
                    </div>
                ))}
            </div>

            <div className="mt-4 pt-4 border-t border-white/5 space-y-2">
                {/* Bottom Items (About) */}
                {bottomItems.map((item) => (
                    <button
                        key={item.id}
                        onClick={() => setActiveTab(item.id)}
                        className={`w-full flex items-center gap-3 px-4 py-2.5 rounded-xl text-sm font-medium transition-all duration-200 active:scale-95 ${activeTab === item.id
                            ? 'bg-purple-500/10 text-purple-400 shadow-sm'
                            : 'text-zinc-400 hover:text-zinc-200 hover:bg-white/5'
                            }`}
                    >
                        <item.icon size={16} className={activeTab === item.id ? "text-purple-400" : "opacity-70"} />
                        <span>{item.label}</span>
                    </button>
                ))}

                <div className="px-2 pt-2">
                    <div className="p-3 rounded-xl bg-gradient-to-br from-indigo-500/10 to-purple-500/10 border border-white/5">
                        <div className="text-xs font-semibold text-zinc-300">Onyx Pro</div>
                        <div className="text-[10px] text-zinc-500 mt-0.5">v0.0.3-alpha</div>
                    </div>
                </div>
            </div>
        </div >
    );
}
