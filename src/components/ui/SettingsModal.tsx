import { useState, useMemo } from 'react';
import {
    X, User, Palette, Loader2,
    Search, Shield, Zap, Code, Cpu, Database,
    Key, Settings, ChevronRight, Info
} from 'lucide-react';
import { useSettings } from '../../contexts/SettingsContext';
import { pb } from '../../lib/pocketbase';
import SubscriptionPanel from '../settings/SubscriptionPanel';

interface SettingsModalProps {
    user: any;
    onLogout: () => void;
}

export default function SettingsModal({ user, onLogout }: SettingsModalProps) {
    const settings = useSettings();
    const {
        isSettingsOpen,
        toggleSettings,
        updateSettings
    } = settings;

    const [activeTab, setActiveTab] = useState<string>('appearance');
    const [searchTerm, setSearchTerm] = useState('');

    // Auth State
    const [authLoading, setAuthLoading] = useState(false);
    const [authEmail, setAuthEmail] = useState('');
    const [authPassword, setAuthPassword] = useState('');
    const [authMessage, setAuthMessage] = useState<{ type: 'error' | 'success', text: string } | null>(null);
    const [isLogin, setIsLogin] = useState(true);

    const HELP_REGISTRY: Record<string, string> = {
        // ... (truncated for brevity, keep existing content if possible? No, replace tool replaces block)
        uiFontFamily: "The main font for menus and buttons.",
        editorFontFamily: "The primary font for writing. 'JetBrains Mono' is great for reading code.",
        accentColor: "The main highlight color for the app.",
        backgroundEffect: "Adds a subtle visual effect to your workspace background.",
        iconSet: "Changes the style of icons throughout the interface.",
        glassIntensity: "Controls how blurry the transparent panels look.",
        acrylicOpacity: "Controls how see-through the navigation panels are.",
        cornerRadius: "Controls how rounded the corners of menus and boxes look.",
        lineNumbers: "Shows numbers on the side of your notes.",
        indentationGuides: "Draws lines to help you see how your text is aligned.",
        bracketPairColorization: "Colors matching brackets to help you see where sections begin and end.",
        smoothScrolling: "Makes moving up and down the page feel fluid and natural.",
        cursorSmoothCaretAnimation: "Makes the typing cursor slide smoothly between characters.",
        fontLigatures: "Turns character sequences (like ->) into clean symbols (➔).",
        showWhitespace: "Shows hidden marks like spaces and tabs.",
        storageProvider: "Choose where your data is saved (e.g., Onyx Cloud or your own storage).",
        attachmentSyncMode: "Choose how your images and files are saved to the cloud.",
        compressionLevel: "Controls how much your files are shrunk to save space.",
        autoCleanupThreshold: "The maximum amount of space your local files can use.",
        encryptOnSave: "Locks your data with high-level encryption for maximum privacy.",
        zeroKnowledgeMode: "The highest privacy setting. Only you can unlock your data.",
        autoLockTimeout: "How long the app waits before locking itself for security.",
        privacyScreen: "Blurs the screen when you switch away from the app.",
        aiProvider: "Select the AI engine that helps you write your notes.",
        language: "Change the display language for the app.",
        cloudEnabled: "Turn on cloud sync to see your notes on all your devices.",
        latex: "Enables math formulas and scientific symbols using LaTeX.",
        math: "Enables standard math rendering for simple calculations.",
        dailyNotes: "Automatically creates a new note for you every day.",
        spellcheck: "Flags spelling mistakes as you type.",
        aiAutocomplete: "Predicts the next words as you type. Press Tab to accept.",
        vimMode: "Advanced keyboard controls for power users."
    };

    const SETTINGS_REGISTRY = useMemo(() => [
        { id: 'uiFontFamily', label: 'Display Font', desc: 'Main font for menus', tab: 'typography' },
        { id: 'editorFontFamily', label: 'Editor Font', desc: 'Font for writing notes', tab: 'typography' },
        { id: 'sidebarFontFamily', label: 'Sidebar Font', desc: 'Font for navigation', tab: 'typography' },
        { id: 'codeFontFamily', label: 'Code Font', desc: 'Font for code blocks', tab: 'typography' },
        { id: 'uiFontSize', label: 'Font Size', desc: 'Overall text size', tab: 'typography' },

        { id: 'accentColor', label: 'Theme Color', desc: 'Primary highlight color', tab: 'interface' },
        { id: 'backgroundEffect', label: 'Background Style', desc: 'Visual styles for your workspace', tab: 'interface' },
        { id: 'cogAnimationSpeed', label: 'Icon Animation', desc: 'Speed of menu animations', tab: 'interface' },
        { id: 'themeVariant', label: 'Base Theme', desc: 'Dark or light appearance', tab: 'interface' },
        { id: 'glassIntensity', label: 'Blur Intensity', desc: 'Glass blur amount', tab: 'interface' },

        { id: 'lineNumbers', label: 'Line Numbers', desc: 'Show margin numbers', tab: 'editor' },
        { id: 'indentationGuides', label: 'Alignment Guides', desc: 'Visual alignment lines', tab: 'editor' },
        { id: 'bracketPairColorization', label: 'Colorful Brackets', desc: 'Color matches for sections', tab: 'editor' },
        { id: 'smoothScrolling', label: 'Smooth Scroll', desc: 'Fluid page movement', tab: 'editor' },

        { id: 'attachmentSyncMode', label: 'Sync Mode', desc: 'How files upload to cloud', tab: 'storage' },
        { id: 'compressionLevel', label: 'File Compression', desc: 'Shrink file size', tab: 'storage' },
        { id: 'autoCleanupThreshold', label: 'Cleanup Limit', desc: 'Max space for local files', tab: 'storage' },

        { id: 'encryptOnSave', label: 'Secure Vault', desc: 'Encrypt all your notes', tab: 'security' },
        { id: 'zeroKnowledgeMode', label: 'Privacy Plus', desc: 'Maximum privacy mode', tab: 'security' },
        { id: 'maskSensitiveData', label: 'Hide Private Info', desc: 'Mask emails and keys', tab: 'security' },

        { id: 'aiProvider', label: 'AI Writing Helper', desc: 'Choose your AI assistant', tab: 'advanced' },
        { id: 'vimMode', label: 'Advanced Keys', desc: 'Power user keyboard controls', tab: 'advanced' },
    ], []);

    const tabs = useMemo(() => {
        const baseTabs = [
            { id: 'account', label: 'Cloud', icon: User, category: 'General' },
            { id: 'app-settings', label: 'App Settings', icon: Cpu, category: 'General' },
            { id: 'typography', label: 'Typography', icon: Code, category: 'Visuals' },
            { id: 'interface', label: 'Interface & FX', icon: Palette, category: 'Visuals' },
            { id: 'editor', label: 'Pro-Editor', icon: Code, category: 'Writing' },
            { id: 'features', label: 'Module Control', icon: Zap, category: 'Writing' },
            { id: 'storage', label: 'Data', icon: Database, category: 'Data' },
            { id: 'security', label: 'Encryption', icon: Shield, category: 'Privacy' },
            { id: 'shortcuts', label: 'Command Deck', icon: Key, category: 'General' },
            { id: 'advanced', label: 'Lab & AI', icon: Zap, category: 'Experimental' },
            { id: 'about', label: 'About Onyx', icon: Info, category: 'System' },
        ];
        return baseTabs;
    }, []);

    const filteredTabs = useMemo(() => {
        if (!searchTerm) return tabs;
        return tabs.filter(tab =>
            tab.label.toLowerCase().includes(searchTerm.toLowerCase()) ||
            tab.category.toLowerCase().includes(searchTerm.toLowerCase())
        );
    }, [searchTerm, tabs]);

    const searchResults = useMemo(() => {
        if (!searchTerm) return [];
        return SETTINGS_REGISTRY.filter(item =>
            item.label.toLowerCase().includes(searchTerm.toLowerCase()) ||
            item.desc.toLowerCase().includes(searchTerm.toLowerCase())
        );
    }, [searchTerm, SETTINGS_REGISTRY]);

    if (!isSettingsOpen) return null;

    const handleAuth = async (e: React.FormEvent) => {
        e.preventDefault();
        setAuthLoading(true);
        setAuthMessage(null);

        try {
            // ZERO-KNOWLEDGE: Hash email before sending
            const { hashEmail } = await import('../../services/SecurityService');
            const pseudonym = await hashEmail(authEmail);

            if (isLogin) {
                await pb.collection('users').authWithPassword(pseudonym, authPassword);
                setAuthMessage({ type: 'success', text: 'Logged in successfully!' });
            } else {
                await pb.collection('users').create({
                    email: pseudonym,
                    password: authPassword,
                    passwordConfirm: authPassword,
                    name: 'Onyx User'
                });

                await pb.collection('users').authWithPassword(pseudonym, authPassword);
                setAuthMessage({ type: 'success', text: 'Account created! You can now sign in.' });
            }
        } catch (err: any) {
            setAuthMessage({ type: 'error', text: err.message });
        } finally {
            setAuthLoading(false);
        }
    };

    return (
        <div className="fixed inset-0 z-[100] flex items-center justify-center p-4">
            {/* Backdrop */}
            <div
                className="absolute inset-0 bg-black/60 backdrop-blur-md animate-in fade-in duration-300"
                onClick={() => toggleSettings(false)}
            />

            {/* Modal Container */}
            <div className="relative w-full max-w-4xl h-[650px] bg-zinc-950/90 border border-zinc-800/50 rounded-2xl shadow-2xl overflow-hidden flex animate-in zoom-in-95 duration-200">
                {/* Top-Right Close Button */}
                <button
                    onClick={() => toggleSettings(false)}
                    className="absolute top-6 right-6 z-50 p-2 text-zinc-500 hover:text-red-500 hover:bg-red-500/10 rounded-xl transition-all group"
                >
                    <X size={20} className="group-hover:rotate-90 transition-transform duration-300" />
                </button>

                {/* Sidebar Navigation */}
                <div className="w-56 bg-zinc-900/40 border-r border-zinc-800/50 p-4 flex flex-col gap-1">
                    <div className="flex items-center gap-2 px-3 mb-6">
                        <div className="w-6 h-6 bg-purple-500 rounded-md flex items-center justify-center">
                            <Settings size={14} className="text-white" />
                        </div>
                        <span className="text-sm font-bold text-zinc-100 tracking-tight">Onyx Settings</span>
                    </div>

                    {/* Settings Search */}
                    <div className="relative mb-4 px-1">
                        <Search size={14} className="absolute left-3 top-1/2 -translate-y-1/2 text-zinc-500" />
                        <input
                            type="text"
                            placeholder="Search setting..."
                            value={searchTerm}
                            onChange={(e) => setSearchTerm(e.target.value)}
                            className="w-full bg-zinc-950/50 border border-zinc-800 rounded-lg pl-9 pr-3 py-1.5 text-xs text-zinc-300 focus:border-purple-500/50 outline-none transition-all placeholder:text-zinc-600"
                        />
                    </div>

                    <div className="flex-1 overflow-y-auto custom-scrollbar pr-1 space-y-4">
                        {['General', 'Visuals', 'Writing', 'Data', 'Privacy', 'Experimental', 'System'].map(category => {
                            const categoryTabs = filteredTabs.filter(t => t.category === category);
                            if (categoryTabs.length === 0) return null;

                            return (
                                <div key={category} className="space-y-1">
                                    <div className="text-[10px] font-bold text-zinc-600 uppercase tracking-widest px-3 py-1">{category}</div>
                                    {categoryTabs.map(tab => (
                                        <button
                                            key={tab.id}
                                            onClick={() => setActiveTab(tab.id)}
                                            className={`flex items-center justify-between w-full px-3 py-2 rounded-lg text-sm font-medium transition-all group ${activeTab === tab.id
                                                ? 'bg-purple-500/10 text-purple-400'
                                                : 'text-zinc-400 hover:text-zinc-200 hover:bg-white/5'
                                                }`}
                                        >
                                            <div className="flex items-center gap-3">
                                                <tab.icon size={16} className={activeTab === tab.id ? 'text-purple-400' : 'text-zinc-500 group-hover:text-zinc-300'} />
                                                {tab.label}
                                            </div>
                                            {activeTab === tab.id && <ChevronRight size={14} className="animate-in slide-in-from-left-2" />}
                                        </button>
                                    ))}
                                </div>
                            );
                        })}
                    </div>

                </div>

                {/* Content Area */}
                <div className="flex-1 flex flex-col overflow-hidden">
                    <div className="pt-20 px-8 pb-8 flex-1 overflow-y-auto custom-scrollbar bg-zinc-950/20">

                        {/* SEARCH RESULTS OVERLAY (If searching) */}
                        {searchTerm && (
                            <div className="animate-in fade-in slide-in-from-top-4 duration-300">
                                <h1 className="text-xl font-bold text-zinc-100 mb-6 flex items-center gap-2">
                                    <Search size={20} className="text-purple-400" />
                                    Results for "{searchTerm}"
                                </h1>
                                <div className="space-y-2">
                                    {searchResults.length > 0 ? (
                                        searchResults.map(item => (
                                            <button
                                                key={item.id}
                                                onClick={() => { setActiveTab(item.tab); setSearchTerm(''); }}
                                                className="w-full p-4 bg-zinc-900/40 hover:bg-zinc-900 border border-zinc-800 rounded-xl text-left transition-all group"
                                            >
                                                <div className="flex items-center justify-between">
                                                    <div>
                                                        <p className="text-sm font-bold text-zinc-200 group-hover:text-purple-400 transition-colors">{item.label}</p>
                                                        <p className="text-xs text-zinc-500 mt-1">{item.desc}</p>
                                                    </div>
                                                    <span className="text-[10px] bg-zinc-800 px-2 py-1 rounded text-zinc-500 uppercase font-mono tracking-tighter">{item.tab}</span>
                                                </div>
                                            </button>
                                        ))
                                    ) : (
                                        <div className="py-20 text-center">
                                            <p className="text-sm text-zinc-600">No settings matched your query.</p>
                                        </div>
                                    )}
                                </div>
                            </div>
                        )}

                        {!searchTerm && (
                            <div className="animate-in fade-in duration-500 relative z-10">

                                {/* ACCOUNT PROFILE SECTION */}
                                {activeTab === 'account' && (
                                    <div className="space-y-8">
                                        <div className="flex items-center justify-between">
                                            <h1 className="text-xl font-bold text-zinc-100 italic">Cloud</h1>
                                        </div>

                                        {!settings.cloudEnabled ? (
                                            <div className="p-12 text-center bg-zinc-900/20 border border-dashed border-zinc-800 rounded-3xl space-y-4">
                                                <Database size={40} className="mx-auto text-zinc-700" />
                                                <h3 className="text-lg font-bold text-zinc-400 uppercase tracking-tighter">Offline Mode</h3>
                                                <p className="text-xs text-zinc-600 max-w-xs mx-auto italic">Synchronisation is disabled. All vaults are stored strictly on this machine with AES-256 isolation.</p>
                                                <button onClick={() => updateSettings({ cloudEnabled: true })} className="mt-4 px-6 py-2 bg-purple-500 text-white text-[10px] font-black uppercase tracking-widest rounded-xl hover:bg-purple-400 transition-all shadow-xl shadow-purple-500/20">Enable Cloud</button>
                                            </div>
                                        ) : user ? (
                                            <div className="space-y-8">
                                                <div className="relative group p-6 bg-zinc-900/40 border border-zinc-800 rounded-3xl overflow-hidden ring-1 ring-white/5 shadow-2xl">
                                                    <div className="absolute top-0 right-0 p-8 opacity-10 group-hover:opacity-100 transition-opacity duration-1000">
                                                        <SecurityShield score={4} />
                                                    </div>

                                                    <div className="flex items-center gap-6 relative z-10">
                                                        <div className="w-20 h-20 bg-gradient-to-tr from-purple-600 to-indigo-600 rounded-3xl flex items-center justify-center text-white shadow-2xl rotate-3 group-hover:rotate-0 transition-transform duration-500">
                                                            <User size={40} />
                                                        </div>
                                                        <div className="flex-1">
                                                            <h2 className="text-2xl font-black text-white tracking-tight leading-none mb-2">ANONYMOUS USER</h2>
                                                            <p className="text-sm font-medium text-zinc-500 mb-1 font-mono">{user.email?.substring(0, 12)}... (Hash)</p>
                                                            <p className="text-[10px] text-emerald-500 flex items-center gap-1">
                                                                <Shield size={10} /> ZERO-KNOWLEDGE IDENTITY
                                                            </p>
                                                            <div className="flex gap-2 mt-4">
                                                                <span className="px-2 py-1 rounded-lg bg-emerald-500/10 text-emerald-400 text-[10px] font-bold uppercase tracking-widest border border-emerald-500/20">Pro Member</span>
                                                                <span className="px-2 py-1 rounded-lg bg-zinc-800 text-zinc-400 text-[10px] font-mono border border-white/5">ID: {user.id.slice(0, 8)}</span>
                                                            </div>
                                                        </div>
                                                        <div className="flex items-center gap-2">
                                                            <button
                                                                onClick={() => updateSettings({ cloudEnabled: false })}
                                                                className="px-4 py-2 bg-zinc-800 hover:bg-zinc-700 text-zinc-400 hover:text-white border border-white/5 rounded-xl transition-all duration-300 text-[10px] font-bold uppercase tracking-widest"
                                                            >
                                                                Offline Mode
                                                            </button>
                                                            <button onClick={onLogout} className="px-6 py-2.5 bg-red-500/10 hover:bg-red-500 text-red-500 hover:text-white border border-red-500/20 rounded-xl transition-all duration-300 text-xs font-black uppercase tracking-widest">Sign Out</button>
                                                        </div>
                                                    </div>
                                                </div>

                                                {/* STRIPE SUBSCRIPTION PANEL */}
                                                <div className="p-1">
                                                    <SubscriptionPanel />
                                                </div>

                                                <div className="p-6 bg-zinc-900/40 border border-zinc-800 rounded-3xl space-y-6">
                                                    <div className="flex justify-between items-center">
                                                        <h3 className="text-[10px] font-bold text-zinc-500 uppercase tracking-widest">Storage & Usage</h3>
                                                        <span className="text-[10px] font-mono text-purple-400 bg-purple-500/10 px-2 py-0.5 rounded border border-purple-500/20">PRO_ACC</span>
                                                    </div>
                                                    <div className="space-y-3">
                                                        <div className="flex justify-between text-[10px] font-mono text-zinc-500">
                                                            <span>1.22 GB / 10 GB</span>
                                                            <span className="text-zinc-300">12% used</span>
                                                        </div>
                                                        <div className="w-full h-2 bg-zinc-950 rounded-full border border-white/5 overflow-hidden">
                                                            <div className="h-full bg-gradient-to-r from-purple-600 to-emerald-500 w-[12.2%]" />
                                                        </div>
                                                    </div>
                                                    <div className="grid grid-cols-2 gap-4">
                                                        <div className="p-4 bg-zinc-950/40 border border-white/5 rounded-2xl">
                                                            <div className="text-[9px] text-zinc-600 font-bold uppercase mb-1">Last Sync</div>
                                                            <div className="text-xs font-black text-zinc-300">2 MINUTES AGO</div>
                                                        </div>
                                                        <div className="p-4 bg-zinc-950/40 border border-white/5 rounded-2xl">
                                                            <div className="text-[9px] text-zinc-600 font-bold uppercase mb-1">Connected Devices</div>
                                                            <div className="text-xs font-black text-emerald-400">4 DEVICES</div>
                                                        </div>
                                                    </div>
                                                </div>
                                            </div>
                                        ) : (
                                            <div className="max-w-md mx-auto py-12 text-center space-y-8">
                                                <div className="w-20 h-20 bg-zinc-900 border border-zinc-800 rounded-3xl flex items-center justify-center mx-auto shadow-2xl animate-bounce">
                                                    <Loader2 size={32} className="text-zinc-700 animate-spin" />
                                                </div>
                                                <div className="space-y-4">
                                                    <h2 className="text-2xl font-black text-white italic tracking-tighter uppercase">Cloud Sync</h2>
                                                    <p className="text-zinc-500 text-sm italic">Keep your notes safe and accessible on all devices.</p>

                                                    <form onSubmit={handleAuth} className="mt-8 space-y-3">
                                                        <input type="email" placeholder="Email Address" value={authEmail} onChange={e => setAuthEmail(e.target.value)} className="w-full bg-zinc-950 border border-zinc-800 rounded-xl px-4 py-3 text-xs text-zinc-300 outline-none focus:border-purple-500/50 transition-all" />
                                                        <input type="password" placeholder="Password" value={authPassword} onChange={e => setAuthPassword(e.target.value)} className="w-full bg-zinc-950 border border-zinc-800 rounded-xl px-4 py-3 text-xs text-zinc-300 outline-none focus:border-purple-500/50 transition-all" />

                                                        {authMessage && (
                                                            <div className={`p-3 rounded-xl text-[10px] font-bold uppercase tracking-widest ${authMessage.type === 'error' ? 'bg-red-500/10 text-red-500 border border-red-500/20' : 'bg-emerald-500/10 text-emerald-500 border border-emerald-500/20'}`}>
                                                                {authMessage.text}
                                                            </div>
                                                        )}

                                                        <button type="submit" disabled={authLoading} className="w-full py-3 bg-white text-black font-black uppercase tracking-widest rounded-xl hover:bg-zinc-200 transition-all shadow-xl shadow-white/5 disabled:opacity-50">
                                                            {authLoading ? 'Signing in...' : isLogin ? 'Sign In' : 'Create Account'}
                                                        </button>
                                                        <button type="button" onClick={() => setIsLogin(!isLogin)} className="text-[10px] text-zinc-500 hover:text-zinc-300 transition-colors uppercase font-bold tracking-widest">{isLogin ? "Don't have an account?" : 'Already have an account?'}</button>
                                                    </form>
                                                </div>
                                            </div>
                                        )}
                                    </div>
                                )}

                                {/* APP SETTINGS SECTION */}
                                {activeTab === 'app-settings' && (
                                    <div className="space-y-10">
                                        <h1 className="text-xl font-bold text-zinc-100">App Settings</h1>
                                        <div className="p-8 bg-zinc-900/40 border border-zinc-800 rounded-3xl space-y-8 animate-in slide-in-from-bottom-4 duration-500">
                                            <SelectItem label="Language" value={settings.language} options={['English', 'Spanish', 'French', 'German', 'Japanese', 'Arabic']} onChange={v => updateSettings({ language: v })} helpText={HELP_REGISTRY.language} />
                                            <ToggleItem label="Spell Checking" description="Checks for spelling mistakes in your notes." checked={settings.activeFeatures.spellcheck} onChange={v => updateSettings({ activeFeatures: { ...settings.activeFeatures, spellcheck: v } })} helpText={HELP_REGISTRY.spellcheck} />

                                            <div className="space-y-4 pt-4 border-t border-zinc-800/50">
                                                <h3 className="text-[10px] font-bold text-zinc-600 uppercase tracking-widest">System Paths</h3>
                                                <div className="space-y-3">
                                                    <label className="text-[10px] font-bold text-zinc-500 uppercase block pl-1 italic">Vault Root Directory</label>
                                                    <div className="flex gap-2">
                                                        <input readOnly value={settings.attachmentPath} className="flex-1 bg-zinc-950 border border-zinc-800 rounded-xl px-4 py-2 text-[10px] text-zinc-500 font-mono outline-none" />
                                                        <button className="px-4 py-2 bg-zinc-800 hover:bg-zinc-700 text-zinc-300 text-[10px] font-bold rounded-xl transition-all border border-white/5">Change</button>
                                                    </div>
                                                </div>
                                            </div>
                                        </div>
                                    </div>
                                )}

                                {/* MODULE CONTROL SECTION */}
                                {activeTab === 'features' && (
                                    <div className="space-y-10">
                                        <h1 className="text-xl font-bold text-zinc-100">Features</h1>
                                        <div className="p-8 bg-zinc-900/40 border border-zinc-800 rounded-3xl space-y-8 animate-in slide-in-from-bottom-4 duration-500">
                                            <div className="grid grid-cols-2 gap-x-12 gap-y-8">
                                                <ToggleItem label="Math Formulas (LaTeX)" description="Scientific typing and symbols." checked={settings.activeFeatures.latex} onChange={v => updateSettings({ activeFeatures: { ...settings.activeFeatures, latex: v } })} helpText={HELP_REGISTRY.latex} />
                                                <ToggleItem label="Simple Math" description="Calculations within your text." checked={settings.activeFeatures.math} onChange={v => updateSettings({ activeFeatures: { ...settings.activeFeatures, math: v } })} helpText={HELP_REGISTRY.math} />
                                                <ToggleItem label="Daily Notes" description="Automatically create daily journals." checked={settings.activeFeatures.dailyNotes} onChange={v => updateSettings({ activeFeatures: { ...settings.activeFeatures, dailyNotes: v } })} helpText={HELP_REGISTRY.dailyNotes} />
                                                <ToggleItem label="Graph View" description="Visual map of your linked notes." checked={settings.activeFeatures.graphView} onChange={v => updateSettings({ activeFeatures: { ...settings.activeFeatures, graphView: v } })} />
                                            </div>
                                            <div className="p-4 bg-purple-500/5 border border-purple-500/10 rounded-2xl italic">
                                                <p className="text-[10px] text-purple-400">"Turning off features you don't use can make the app run faster."</p>
                                            </div>
                                        </div>
                                    </div>
                                )}

                                {/* ABOUT SECTION */}
                                {activeTab === 'about' && (
                                    <div className="space-y-10 text-center py-10 animate-in fade-in duration-1000">
                                        <div className="w-24 h-24 bg-gradient-to-tr from-purple-600 to-indigo-600 rounded-[2.5rem] mx-auto flex items-center justify-center shadow-2xl shadow-purple-500/20 mb-6 group cursor-pointer hover:rotate-12 transition-transform duration-500">
                                            <Settings size={48} className="text-white" />
                                        </div>
                                        <div className="space-y-2">
                                            <h1 className="text-3xl font-black text-white tracking-tighter">ONYX</h1>
                                            <p className="text-[10px] text-zinc-500 font-mono uppercase tracking-[0.3em] font-black">{settings.appVersion}</p>
                                        </div>

                                        <div className="max-w-xs mx-auto space-y-4 pt-10">
                                            <button className="w-full py-4 bg-zinc-900 border border-white/5 text-zinc-100 text-[10px] font-black uppercase tracking-widest rounded-2xl hover:bg-zinc-800 transition-all active:scale-95">Verify Integrity & Updates</button>
                                            <div className="pt-8 border-t border-zinc-900">
                                                <p className="text-[9px] text-zinc-600 leading-relaxed uppercase tracking-tighter">Onyx Collective © 2026<br />All Rights Reserved</p>
                                            </div>
                                        </div>
                                    </div>
                                )}

                                {/* TYPOGRAPHY SECTION */}
                                {activeTab === 'typography' && (
                                    <div className="space-y-10 animate-in slide-in-from-bottom-4 duration-500">
                                        <div className="flex items-center justify-between">
                                            <h1 className="text-xl font-bold text-zinc-100">Typography</h1>
                                            <div className="flex items-center gap-2">
                                                <span className="px-2 py-0.5 rounded-md bg-zinc-800 text-[10px] font-bold text-zinc-400 border border-white/5 uppercase">Text System v4</span>
                                            </div>
                                        </div>

                                        <div className="space-y-10">
                                            <div className="grid grid-cols-2 gap-8">
                                                <div className="space-y-4">
                                                    <div className="text-[9px] font-black text-zinc-600 uppercase tracking-widest pl-1">Menu & Buttons Preview</div>
                                                    <div className="p-8 bg-zinc-900/60 border border-zinc-800 rounded-[2rem] min-h-[120px] flex items-center justify-center text-center shadow-inner" style={{ fontFamily: settings.uiFontFamily }}>
                                                        <p className="text-sm text-zinc-200 leading-relaxed italic">"The interface should be felt, but the type must be heard."</p>
                                                    </div>
                                                </div>
                                                <div className="space-y-4">
                                                    <div className="text-[9px] font-black text-zinc-600 uppercase tracking-widest pl-1">Writing Area Preview</div>
                                                    <div className="p-8 bg-zinc-900/60 border border-zinc-800 rounded-[2rem] min-h-[120px] flex items-center justify-center text-center shadow-inner" style={{ fontFamily: settings.editorFontFamily }}>
                                                        <p className="text-xs text-zinc-300">class Onyx {'{'} static experience = 'limitless'; {'}'}</p>
                                                    </div>
                                                </div>
                                            </div>

                                            <div className="grid grid-cols-2 gap-x-12 gap-y-10 pt-4">
                                                <div className="space-y-4">
                                                    <SelectItem label="Menu Font" value={settings.uiFontFamily} options={['Inter', 'System', 'Outfit', 'Montserrat']} onChange={v => updateSettings({ uiFontFamily: v as any })} helpText={HELP_REGISTRY.uiFontFamily} />
                                                </div>
                                                <div className="space-y-4">
                                                    <SelectItem label="Writing Font" value={settings.editorFontFamily} options={['Inter', 'JetBrains Mono', 'Georgia', 'Slab', 'Playfair']} onChange={v => updateSettings({ editorFontFamily: v as any })} helpText={HELP_REGISTRY.editorFontFamily} />
                                                </div>
                                                <div className="space-y-4">
                                                    <SelectItem label="Sidebar Font" value={settings.sidebarFontFamily} options={['Inter', 'System', 'Outfit']} onChange={v => updateSettings({ sidebarFontFamily: v as any })} helpText={HELP_REGISTRY.uiFontFamily} />
                                                    <div className="p-3 bg-zinc-950/40 border border-zinc-800/50 rounded-2xl flex items-center gap-3">
                                                        <span style={{ fontFamily: settings.sidebarFontFamily }} className="text-[10px] font-bold text-zinc-400 uppercase tracking-tight">DOCUMENT_ROOT / ASSETS</span>
                                                    </div>
                                                </div>
                                                <div className="space-y-4">
                                                    <SelectItem label="Code & Monospace" value={settings.codeFontFamily} options={['JetBrains Mono', 'System']} onChange={v => updateSettings({ codeFontFamily: v as any })} helpText={HELP_REGISTRY.editorFontFamily} />
                                                    <div className="p-3 bg-zinc-950/40 border border-zinc-800/50 rounded-2xl">
                                                        <span style={{ fontFamily: settings.codeFontFamily }} className="text-[10px] font-mono text-purple-400">async function vault_sync() {'{}'}</span>
                                                    </div>
                                                </div>
                                                <RangeItem label="Global UI Scale" value={settings.uiFontSize} min={12} max={18} unit="px" onChange={v => updateSettings({ uiFontSize: v })} />
                                                <RangeItem label="Sidebar Item Scale" value={settings.sidebarFontSize} min={10} max={16} unit="px" onChange={v => updateSettings({ sidebarFontSize: v })} />
                                            </div>
                                        </div>
                                    </div>
                                )}

                                {/* INTERFACE SECTION */}
                                {activeTab === 'interface' && (
                                    <div className="space-y-10">
                                        <div className="flex items-center justify-between">
                                            <h1 className="text-xl font-bold text-zinc-100">Visual Effects</h1>
                                            <div className="px-2 py-0.5 rounded-full bg-zinc-800 text-[10px] font-bold text-zinc-400">Design v4.0</div>
                                        </div>

                                        <div className="grid grid-cols-2 gap-8">
                                            <div className="space-y-8">
                                                <div className="p-6 bg-zinc-900/40 border border-zinc-800 rounded-3xl space-y-6 animate-in slide-in-from-bottom-4 duration-500 delay-75">
                                                    <SelectItem label="Background Style" value={settings.backgroundEffect} options={['none', 'mesh', 'particles', 'aurora', 'grain', 'pulse']} onChange={v => updateSettings({ backgroundEffect: v as any })} helpText={HELP_REGISTRY.backgroundEffect} />
                                                    <SelectItem label="Icon Style" value={settings.iconSet} options={['classic', 'neon', 'minimal', 'onyx-bold']} onChange={v => updateSettings({ iconSet: v as any })} helpText={HELP_REGISTRY.iconSet} />
                                                    <SelectItem label="Theme Color" value={settings.accentColor} options={['purple', 'emerald', 'blue', 'amber', 'rose', 'zinc', 'crimson', 'ocean']} onChange={v => updateSettings({ accentColor: v as any })} helpText={HELP_REGISTRY.accentColor} />
                                                </div>
                                                {/* Space for other visual settings */}
                                            </div>

                                            <div className="space-y-8">
                                                <div className="p-6 bg-zinc-900/40 border border-zinc-800 rounded-3xl space-y-6 animate-in slide-in-from-bottom-4 duration-500 delay-200">
                                                    <h3 className="text-[10px] font-bold text-zinc-500 uppercase tracking-widest">Transparency</h3>
                                                    <RangeItem label="Panel Opacity" value={settings.acrylicOpacity} min={0} max={100} unit="%" onChange={v => updateSettings({ acrylicOpacity: v })} helpText={HELP_REGISTRY.acrylicOpacity} />
                                                    <RangeItem label="Blur Strength" value={settings.glassIntensity} min={0} max={100} onChange={v => updateSettings({ glassIntensity: v })} helpText={HELP_REGISTRY.glassIntensity} />
                                                    <RangeItem label="Corner Roundness" value={settings.cornerRadius} min={0} max={32} unit="px" onChange={v => updateSettings({ cornerRadius: v })} helpText={HELP_REGISTRY.cornerRadius} />
                                                </div>
                                                <div className="p-6 bg-purple-500/10 border border-purple-500/20 rounded-3xl animate-in fade-in duration-1000 delay-300">
                                                    <h3 className="text-xs font-bold text-purple-400 mb-2">Premium Themes</h3>
                                                    <p className="text-[10px] text-purple-200/60 leading-relaxed mb-4">Unlock advanced backgrounds and styles that change while you move your cursor.</p>
                                                    <button className="w-full py-2 bg-purple-500 text-white text-[10px] font-black uppercase tracking-widest rounded-xl hover:bg-purple-400 transition-colors shadow-lg shadow-purple-500/20">Browse Themes</button>
                                                </div>
                                            </div>
                                        </div>
                                    </div>
                                )}

                                {/* EDITOR SECTION */}
                                {activeTab === 'editor' && (
                                    <div className="space-y-8">
                                        <div className="flex items-center justify-between">
                                            <h1 className="text-xl font-bold text-zinc-100">Editor Settings</h1>
                                            <div className="flex items-center gap-2">
                                                <span className="px-2 py-1 rounded-md bg-zinc-800 text-[9px] font-black text-zinc-400 uppercase border border-white/5">LLM Optimized</span>
                                            </div>
                                        </div>

                                        <div className="space-y-6">
                                            <div className="p-6 bg-zinc-900/40 border border-zinc-800 rounded-3xl animate-in slide-in-from-bottom-4 duration-500 delay-75">
                                                <h3 className="text-xs font-bold text-zinc-500 uppercase tracking-widest mb-6">Writing Aids</h3>
                                                <div className="grid grid-cols-2 gap-x-12 gap-y-6">
                                                    <div className="space-y-4">
                                                        <ToggleItem label="Indentation Guides" description="Visual scope alignment" checked={settings.indentationGuides} onChange={v => updateSettings({ indentationGuides: v })} helpText={HELP_REGISTRY.indentationGuides} />
                                                        <FeaturePreview type="guides" active={settings.indentationGuides} />
                                                    </div>
                                                    <div className="space-y-4">
                                                        <ToggleItem label="Bracket Colorization" description="Rainbow scoping" checked={settings.bracketPairColorization} onChange={v => updateSettings({ bracketPairColorization: v })} helpText={HELP_REGISTRY.bracketPairColorization} />
                                                        <FeaturePreview type="brackets" active={settings.bracketPairColorization} />
                                                    </div>
                                                </div>
                                            </div>

                                            <div className="grid grid-cols-2 gap-8">
                                                <div className="p-6 bg-zinc-900/40 border border-zinc-800 rounded-3xl space-y-4 animate-in slide-in-from-bottom-4 duration-500 delay-150">
                                                    <h3 className="text-[10px] font-bold text-zinc-600 uppercase">Input & Movement</h3>
                                                    <div className="space-y-4">
                                                        <ToggleItem label="Kinetic Scrolling" checked={settings.smoothScrolling} onChange={v => updateSettings({ smoothScrolling: v })} helpText={HELP_REGISTRY.smoothScrolling} />
                                                        <div className="pt-2">
                                                            <ToggleItem label="Smooth Cursor" checked={settings.cursorSmoothCaretAnimation} onChange={v => updateSettings({ cursorSmoothCaretAnimation: v })} helpText={HELP_REGISTRY.cursorSmoothCaretAnimation} />
                                                            <FeaturePreview type="caret" active={settings.cursorSmoothCaretAnimation} />
                                                        </div>
                                                    </div>
                                                    <SelectItem label="Cursor Animation" value={settings.cursorBlinking} options={['blink', 'smooth', 'phase', 'expand', 'solid']} onChange={v => updateSettings({ cursorBlinking: v as any })} />
                                                </div>
                                                <div className="p-6 bg-zinc-900/40 border border-zinc-800 rounded-3xl space-y-4 animate-in slide-in-from-bottom-4 duration-500 delay-200">
                                                    <h3 className="text-[10px] font-bold text-zinc-600 uppercase">Accessibility</h3>
                                                    <ToggleItem label="Line Numbers" checked={settings.lineNumbers} onChange={v => updateSettings({ lineNumbers: v })} helpText={HELP_REGISTRY.lineNumbers} />
                                                    <div className="pt-2">
                                                        <ToggleItem label="Font Ligatures" checked={settings.fontLigatures} onChange={v => updateSettings({ fontLigatures: v })} helpText={HELP_REGISTRY.fontLigatures} />
                                                        <FeaturePreview type="ligatures" active={settings.fontLigatures} />
                                                    </div>
                                                    <SelectItem label="Whitespaces" value={settings.showWhitespace} options={['none', 'selection', 'all']} onChange={v => updateSettings({ showWhitespace: v as any })} helpText={HELP_REGISTRY.showWhitespace} />
                                                </div>
                                            </div>
                                        </div>
                                    </div>
                                )}

                                {/* STORAGE SECTION */}
                                {activeTab === 'storage' && (
                                    <div className="space-y-8">
                                        <div className="flex items-center justify-between">
                                            <h1 className="text-xl font-bold text-zinc-100">Data & Storage</h1>
                                            <Database size={20} className="text-zinc-600" />
                                        </div>

                                        <div className="space-y-6 animate-in slide-in-from-bottom-4 duration-500 delay-75">
                                            <div className="p-6 bg-zinc-900/40 border border-zinc-800 rounded-3xl space-y-6">
                                                <h3 className="text-xs font-bold text-zinc-500 uppercase tracking-widest">File Handling</h3>
                                                <div className="grid grid-cols-2 gap-x-12 gap-y-4 pt-2">
                                                    <SelectItem label="Compression" value={settings.compressionLevel} options={['none', 'low', 'high', 'ultra']} onChange={v => updateSettings({ compressionLevel: v as any })} />
                                                    <SelectItem label="Sync Mode" value={settings.attachmentSyncMode} options={['all', 'manual', 'never']} onChange={v => updateSettings({ attachmentSyncMode: v as any })} helpText={HELP_REGISTRY.attachmentSyncMode} />
                                                    <RangeItem label="Storage Limit" value={settings.autoCleanupThreshold} min={100} max={2000} step={100} unit="MB" onChange={v => updateSettings({ autoCleanupThreshold: v })} />
                                                    <RangeItem label="Max File Size" value={settings.maxAttachmentSize} min={5} max={200} unit="MB" onChange={v => updateSettings({ maxAttachmentSize: v })} />
                                                </div>
                                            </div>

                                            <div className="p-6 bg-zinc-900/40 border border-zinc-800 rounded-3xl space-y-6">
                                                <h3 className="text-xs font-bold text-zinc-500 uppercase tracking-widest">Primary Cloud Endpoint</h3>
                                                <SelectItem label="Cloud Provider" value={settings.storageProvider} options={['onyx-cloud', 's3', 'webdav', 'gdrive', 'local-only']} onChange={v => updateSettings({ storageProvider: v as any })} helpText={HELP_REGISTRY.storageProvider} />
                                                {settings.storageProvider === 's3' && (
                                                    <div className="p-4 bg-zinc-950/50 border border-zinc-800 rounded-2xl animate-in slide-in-from-top-2">
                                                        <div className="space-y-4">
                                                            <div className="grid grid-cols-2 gap-4 text-[10px] font-mono">
                                                                <div className="space-y-1">
                                                                    <div className="text-zinc-600">S3_ENDPOINT</div>
                                                                    <input
                                                                        type="text"
                                                                        value={settings.s3Config.endpoint}
                                                                        onChange={(e) => updateSettings({ s3Config: { ...settings.s3Config, endpoint: e.target.value } })}
                                                                        className="w-full bg-transparent border-b border-zinc-800 focus:border-purple-500 outline-none text-zinc-300 py-1"
                                                                    />
                                                                </div>
                                                                <div className="space-y-1">
                                                                    <div className="text-zinc-600">BUCKET_NAME</div>
                                                                    <input
                                                                        type="text"
                                                                        value={settings.s3Config.bucket}
                                                                        onChange={(e) => updateSettings({ s3Config: { ...settings.s3Config, bucket: e.target.value } })}
                                                                        className="w-full bg-transparent border-b border-zinc-800 focus:border-purple-500 outline-none text-zinc-300 py-1"
                                                                    />
                                                                </div>
                                                            </div>
                                                        </div>
                                                    </div>
                                                )}
                                            </div>
                                        </div>
                                    </div>
                                )}

                                {/* SECURITY SECTION */}
                                {activeTab === 'security' && (
                                    <div className="space-y-8">
                                        <div className="flex items-center justify-between">
                                            <h1 className="text-xl font-bold text-zinc-100">Security</h1>
                                            <SecurityShield score={4} />
                                        </div>

                                        <div className="space-y-6">
                                            <div className="p-6 bg-zinc-900/40 border border-zinc-800 rounded-3xl space-y-4 animate-in slide-in-from-bottom-4 duration-500 delay-75">
                                                <ToggleItem label="Private Sync" description="Your keys staying only on your device." checked={settings.zeroKnowledgeMode} onChange={v => updateSettings({ zeroKnowledgeMode: v })} helpText={HELP_REGISTRY.zeroKnowledgeMode} />
                                                <ToggleItem label="Encryption" description="Lock all your local files automatically." checked={settings.encryptOnSave} onChange={v => updateSettings({ encryptOnSave: v })} helpText={HELP_REGISTRY.encryptOnSave} />
                                                <RangeItem label="Security Timeout" value={settings.autoLockTimeout} min={1} max={120} unit=" min" onChange={v => updateSettings({ autoLockTimeout: v })} helpText={HELP_REGISTRY.autoLockTimeout} />
                                            </div>
                                            <div className="grid grid-cols-2 gap-8">
                                                <div className="p-6 bg-zinc-900/40 border border-zinc-800 rounded-3xl space-y-4 animate-in slide-in-from-bottom-4 duration-500 delay-150">
                                                    <ToggleItem label="Privacy Screen" description="Blur on focus loss" checked={settings.privacyScreen} onChange={v => updateSettings({ privacyScreen: v })} />
                                                    <ToggleItem label="Mask PII Data" checked={settings.maskSensitiveData} onChange={v => updateSettings({ maskSensitiveData: v })} />
                                                </div>
                                                <div className="p-6 bg-zinc-900/40 border border-zinc-800 rounded-3xl space-y-4 animate-in slide-in-from-bottom-4 duration-500 delay-200">
                                                    <ToggleItem label="Clipboard Guard" description="Wipe data on app exit" checked={settings.secureClipboard} onChange={v => updateSettings({ secureClipboard: v })} />
                                                    <ToggleItem label="Block Screen Cap" checked={settings.preventScreenshots} onChange={v => updateSettings({ preventScreenshots: v })} />
                                                </div>
                                            </div>
                                        </div>
                                    </div>
                                )}

                                {/* SHORTCUTS SECTION */}
                                {activeTab === 'shortcuts' && (
                                    <div className="space-y-8">
                                        <h1 className="text-xl font-bold text-zinc-100">Shortcuts</h1>
                                        <div className="grid grid-cols-1 border border-zinc-800 rounded-3xl overflow-hidden bg-zinc-900/20">
                                            {[
                                                { label: 'Omni-Search', key: 'Cmd + P' },
                                                { label: 'Hyper-New Page', key: 'Cmd + N' },
                                                { label: 'System Settings', key: 'Cmd + ,' },
                                                { label: 'Terminate Buffer', key: 'Cmd + W' },
                                                { label: 'Switch Context', key: 'Ctrl + Tab' },
                                                { label: 'Collapse Deck', key: 'Cmd + \\' },
                                            ].map((s, i) => (
                                                <div key={i} className="flex items-center justify-between px-6 py-4 hover:bg-white/5 transition-all cursor-default border-b border-zinc-800/50 last:border-0 group">
                                                    <span className="text-sm font-bold text-zinc-400 group-hover:text-purple-400">{s.label}</span>
                                                    <div className="flex gap-1.5 p-1 bg-black/40 rounded-lg">
                                                        {s.key.split(' ').map((key, ki) => (
                                                            <span key={ki} className="px-2 py-1 min-w-[32px] text-center border border-white/5 bg-zinc-800/50 text-[10px] font-black text-zinc-200 rounded shadow-2xl">{key}</span>
                                                        ))}
                                                    </div>
                                                </div>
                                            ))}
                                        </div>
                                    </div>
                                )}

                                {/* ADVANCED SECTION */}
                                {activeTab === 'advanced' && (
                                    <div className="space-y-8">
                                        <div className="flex items-center justify-between">
                                            <h1 className="text-xl font-bold text-zinc-100">Advanced</h1>
                                            <Zap size={20} className="text-amber-500 animate-pulse" />
                                        </div>

                                        <div className="p-6 bg-zinc-900/40 border border-zinc-800 rounded-3xl space-y-6 animate-in slide-in-from-bottom-4 duration-500 delay-75">
                                            <SelectItem label="AI Assistant" value={settings.aiProvider} options={['onyx-internal', 'openai', 'anthropic', 'local-ollama']} onChange={v => updateSettings({ aiProvider: v as any })} helpText={HELP_REGISTRY.aiProvider} />
                                            {settings.aiProvider !== 'onyx-internal' && (
                                                <div className="space-y-2">
                                                    <div className="text-[10px] text-zinc-600 font-mono">ENCRYPTED_API_KEY</div>
                                                    <input type="password" value={settings.aiApiKey} onChange={e => updateSettings({ aiApiKey: e.target.value })} className="w-full bg-zinc-950 border border-zinc-800 rounded-xl px-4 py-3 text-xs text-purple-400 outline-none" placeholder="sk-..." />
                                                </div>
                                            )}
                                            <div className="grid grid-cols-2 gap-8">
                                                <ToggleItem label="AI Predict" description="Predicts words as you type." checked={settings.aiAutocomplete} onChange={v => updateSettings({ aiAutocomplete: v })} />
                                                <ToggleItem label="Advanced Keys" description="Special keyboard controls." checked={settings.vimMode} onChange={v => updateSettings({ vimMode: v })} helpText={HELP_REGISTRY.vimMode} />
                                            </div>
                                        </div>

                                        <div className="p-8 border border-red-500/20 bg-red-500/5 rounded-3xl flex items-center justify-between gap-8">
                                            <div className="flex-1">
                                                <h3 className="text-red-400 font-black uppercase tracking-widest text-sm mb-2">Reset All Settings</h3>
                                                <p className="text-xs text-red-400/50 leading-relaxed italic">Wipes all your custom settings and returns the app to its original state.</p>
                                            </div>
                                            <button onClick={() => confirm('RESET ALL SETTINGS?') && (localStorage.clear(), window.location.reload())} className="px-8 py-3 bg-red-500 text-white font-black uppercase tracking-widest text-xs rounded-2xl hover:bg-red-400 transition-all shadow-xl shadow-red-500/20 active:scale-95">Reset App</button>
                                        </div>
                                    </div>
                                )}

                            </div>
                        )}

                    </div>

                    {/* Status Footer */}
                    <div className="px-8 py-3 bg-zinc-950/50 border-t border-zinc-800/50 flex items-center justify-between text-[10px] text-zinc-600 font-mono">
                        <div className="flex items-center gap-4">
                            <div className="flex items-center gap-1.5">
                                <div className="w-1.5 h-1.5 rounded-full bg-emerald-500" />
                                CORE STATUS: OPERATIONAL
                            </div>
                            <div className="text-zinc-700">|</div>
                            <div>VERS 0.4.2</div>
                        </div>
                        <div className="flex items-center gap-3">
                            <span className="opacity-40">ONYX LABS • 2026</span>
                            <div className="text-zinc-700">|</div>
                            <div className="flex items-center gap-1">
                                <Cpu size={10} />
                                RENDER: GPU_ACCEL
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    );
}

/* HELPER COMPONENTS */

function FeaturePreview({ type, active }: { type: 'brackets' | 'guides' | 'caret' | 'ligatures', active: boolean }) {
    return (
        <div className="p-3 bg-zinc-950/80 border border-zinc-800 rounded-xl mt-2 overflow-hidden group">
            <div className="text-[9px] font-bold text-zinc-600 uppercase tracking-tighter mb-2 flex justify-between">
                <span>Visual Simulation</span>
                <span className={active ? 'text-emerald-500 animate-pulse' : 'text-zinc-700'}>{active ? 'Active' : 'Neutral'}</span>
            </div>
            {type === 'ligatures' && (
                <div className="font-mono text-[11px] flex gap-4 items-center justify-center py-1">
                    <span className="text-zinc-400">{'->'}</span>
                    <span className={active ? 'text-purple-400 font-bold' : 'text-zinc-600'}>➔</span>
                    <span className="text-zinc-400">{'!='}</span>
                    <span className={active ? 'text-amber-400 font-bold' : 'text-zinc-600'}>≠</span>
                </div>
            )}
            {type === 'caret' && (
                <div className="h-6 flex items-center justify-center">
                    <div className={`w-0.5 h-4 bg-purple-500 ${active ? 'animate-[pulse_1s_infinite]' : 'animate-none'}`} />
                    <span className="ml-2 text-[10px] text-zinc-500 font-mono">Input active</span>
                </div>
            )}
            {type === 'brackets' && (
                <div className="font-mono text-[11px] flex gap-1 items-center justify-center py-1">
                    <span className={active ? 'text-yellow-500' : 'text-zinc-400'}>{'('}</span>
                    <span className={active ? 'text-blue-500' : 'text-zinc-400'}>{'['}</span>
                    <span className={active ? 'text-rose-500' : 'text-zinc-400'}>{'{'}</span>
                    <span className="text-zinc-200">code</span>
                    <span className={active ? 'text-rose-500' : 'text-zinc-400'}>{'}'}</span>
                    <span className={active ? 'text-blue-500' : 'text-zinc-400'}>{']'}</span>
                    <span className={active ? 'text-yellow-500' : 'text-zinc-400'}>{'('}</span>
                </div>
            )}
            {type === 'guides' && (
                <div className="font-mono text-[11px] space-y-1 py-1">
                    <div className="flex gap-2">
                        <div className={`w-px h-3 ${active ? 'bg-zinc-700' : 'bg-transparent'}`} />
                        <span className="text-zinc-500">function main()</span>
                    </div>
                    <div className="flex gap-2">
                        <div className={`w-px h-3 ${active ? 'bg-zinc-700 ml-4' : 'bg-transparent ml-4'}`} />
                        <span className="text-zinc-500">return true;</span>
                    </div>
                </div>
            )}
        </div>
    );
}


function HelpTooltip({ text }: { text: string }) {
    return (
        <div className="group relative inline-block ml-1.5 align-middle">
            <div className="w-3.5 h-3.5 rounded-full border border-zinc-800 flex items-center justify-center cursor-help transition-all group-hover:border-purple-500/50 group-hover:bg-purple-500/5 animate-pulse group-hover:animate-none">
                <Info size={8} className="text-zinc-600 group-hover:text-purple-400" />
            </div>
            <div className="absolute bottom-full left-1/2 -translate-x-1/2 mb-2 w-48 p-3 bg-zinc-900 border border-zinc-800 rounded-xl shadow-2xl opacity-0 invisible group-hover:opacity-100 group-hover:visible transition-all duration-300 z-[200] pointer-events-none scale-90 group-hover:scale-100">
                <div className="absolute bottom-[-4px] left-1/2 -translate-x-1/2 w-2 h-2 bg-zinc-900 border-r border-b border-zinc-800 rotate-45" />
                <p className="text-[10px] text-zinc-300 leading-relaxed italic">{text}</p>
            </div>
        </div>
    );
}


function SecurityShield({ score }: { score: number }) {
    return (
        <div className="relative w-32 h-32 flex items-center justify-center group">
            <div className={`absolute inset-0 border-2 rounded-full border-purple-500/5 animate-[ping_3s_linear_infinite] opacity-20`} />
            <div className={`absolute inset-4 border border-purple-500/10 rounded-full transition-all duration-1000 ${score > 1 ? 'scale-100 opacity-100' : 'scale-50 opacity-0'}`} />
            <div className={`absolute inset-8 border border-purple-500/20 rounded-full transition-all duration-700 ${score > 2 ? 'scale-100 opacity-100' : 'scale-50 opacity-0'}`} />
            <Shield size={40} className={`transition-all duration-500 relative z-10 ${score > 0 ? 'text-purple-400 drop-shadow-[0_0_20px_rgba(168,85,247,0.4)]' : 'text-zinc-800'}`} />
        </div>
    );
}


function SelectItem({ label, value, options, onChange, helpText }: { label: string, value: string, options: string[], onChange: (val: string) => void, helpText?: string }) {
    return (
        <div className="space-y-3">
            <label className="text-[10px] font-black text-zinc-600 uppercase tracking-widest block pl-1">
                {label}
                {helpText && <HelpTooltip text={helpText} />}
            </label>
            <div className="flex flex-wrap gap-1.5">
                {options.map(opt => (
                    <button
                        key={opt}
                        onClick={() => onChange(opt)}
                        className={`px-3 py-1.5 rounded-xl text-[10px] font-bold border transition-all duration-300 ${value === opt
                            ? 'bg-purple-500/10 border-purple-500/50 text-purple-400 shadow-[0_0_15px_rgba(168,85,247,0.1)]'
                            : 'bg-zinc-900 border-zinc-800 text-zinc-500 hover:text-zinc-300 hover:border-zinc-700'
                            }`}
                    >
                        {opt.replace('-', ' ').toUpperCase()}
                    </button>
                ))}
            </div>
        </div>
    );
}

function RangeItem({ label, value, min, max, step = 1, unit = '', onChange, helpText }: { label: string, value: number, min: number, max: number, step?: number, unit?: string, onChange: (val: number) => void, helpText?: string }) {
    return (
        <div className="space-y-4">
            <div className="flex justify-between items-center px-1">
                <label className="text-[10px] font-black text-zinc-500 uppercase tracking-widest">
                    {label}
                    {helpText && <HelpTooltip text={helpText} />}
                </label>
                <span className="text-[10px] font-mono font-black text-purple-400 bg-purple-500/10 px-2 py-0.5 rounded-lg border border-purple-500/20">{value}{unit}</span>
            </div>
            <input
                type="range" min={min} max={max} step={step}
                value={value}
                onChange={(e) => onChange(parseFloat(e.target.value))}
                className="w-full accent-purple-500 h-1 bg-zinc-900 rounded-full appearance-none cursor-crosshair hover:bg-zinc-800 transition-colors"
            />
        </div>
    );
}

function ToggleItem({ label, description, checked, onChange, helpText }: { label: string, description?: string, checked: boolean, onChange: (val: boolean) => void, helpText?: string }) {
    return (
        <div className="flex items-center justify-between py-2 group">
            <div className="flex-1 min-w-0 pr-6">
                <div className="flex items-center gap-1">
                    <p className="text-sm font-black text-zinc-200 group-hover:text-white transition-colors tracking-tight">{label}</p>
                    {helpText && <HelpTooltip text={helpText} />}
                </div>
                {description && <p className="text-[10px] text-zinc-500 mt-0.5 leading-tight font-medium">{description}</p>}
            </div>
            <button
                onClick={() => onChange(!checked)}
                className={`w-10 h-6 rounded-full relative transition-all duration-500 ring-1 ring-white/5 ${checked ? 'bg-purple-600 shadow-[0_0_20px_rgba(168,85,247,0.3)]' : 'bg-zinc-900'}`}
            >
                <div className={`absolute top-1 w-4 h-4 bg-white rounded-full transition-all duration-500 shadow-2xl ${checked ? 'right-1' : 'left-1'}`} />
            </button>
        </div>
    );
}
