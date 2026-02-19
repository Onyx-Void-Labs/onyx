import { useState } from 'react';
import { Type, Moon, Sun, Monitor, ChevronDown, Check } from 'lucide-react';
import { useSettings } from '../../../../contexts/SettingsContext';

export default function AppearanceTab() {
    const {
        uiFontFamily,
        uiFontSize,
        updateSettings
    } = useSettings();

    const [isFontOpen, setIsFontOpen] = useState(false);

    const fonts = [
        { id: 'Inter, sans-serif', name: 'Inter (Default)' },
        { id: 'Roboto, sans-serif', name: 'Roboto' },
        { id: 'System, sans-serif', name: 'System UI' },
        { id: '"Helvetica Neue", Helvetica, Arial, sans-serif', name: 'Helvetica Neue' },
        { id: 'Georgia, serif', name: 'Georgia' },
        { id: '"Courier New", monospace', name: 'Courier New' },
    ];

    const currentFontName = fonts.find(f => f.id === uiFontFamily)?.name || 'Custom';

    return (
        <div className="space-y-8 animate-in fade-in slide-in-from-bottom-4 duration-500">

            <div className="flex items-center justify-between">
                <div>
                    <h2 className="text-2xl font-bold text-white mb-1">Look & Feel</h2>
                    <p className="text-zinc-400 text-sm">Customize the aesthetics of your workspace.</p>
                </div>
            </div>

            {/* Theme Selection */}
            <div className="space-y-4">
                <h3 className="text-sm font-bold text-zinc-300 uppercase tracking-wider flex items-center gap-2">
                    <Moon size={16} /> Theme
                </h3>
                <div className="grid grid-cols-3 gap-4">
                    {['Dark', 'Light', 'System'].map((theme) => (
                        <button
                            key={theme}
                            className={`p-4 rounded-2xl border flex flex-col items-center gap-3 transition-all ${theme === 'Dark'
                                    ? 'bg-zinc-800 border-purple-500/50 shadow-[0_0_20px_rgba(168,85,247,0.15)]'
                                    : 'bg-zinc-900/40 border-zinc-800 hover:bg-zinc-800 hover:border-zinc-700'
                                }`}
                        >
                            <div className={`w-full h-24 rounded-xl border flex items-center justify-center ${theme === 'Dark' ? 'bg-[#09090b] border-white/5' : theme === 'Light' ? 'bg-white border-zinc-200' : 'bg-gradient-to-br from-[#09090b] to-white border-white/10'
                                }`}>
                                {theme === 'Dark' && <Moon size={24} className="text-purple-400" />}
                                {theme === 'Light' && <Sun size={24} className="text-orange-400" />}
                                {theme === 'System' && <Monitor size={24} className="text-zinc-400" />}
                            </div>
                            <span className={`text-xs font-bold ${theme === 'Dark' ? 'text-white' : 'text-zinc-500'}`}>
                                {theme}
                            </span>
                        </button>
                    ))}
                </div>
            </div>

            {/* Typography */}
            <div className="space-y-4">
                <h3 className="text-sm font-bold text-zinc-300 uppercase tracking-wider flex items-center gap-2">
                    <Type size={16} /> Typography
                </h3>

                <div className="p-6 bg-zinc-900/50 border border-white/5 rounded-3xl space-y-6">
                    {/* Font Dropdown */}
                    <div className="space-y-3 relative">
                        <label className="text-xs font-bold text-zinc-500 uppercase tracking-wide">Interface Font</label>

                        <button
                            onClick={() => setIsFontOpen(!isFontOpen)}
                            className="w-full flex items-center justify-between px-4 py-3 bg-black/20 border border-white/5 hover:bg-white/5 rounded-xl transition-all text-sm text-zinc-200"
                        >
                            <span style={{ fontFamily: uiFontFamily }}>{currentFontName}</span>
                            <ChevronDown size={16} className={`text-zinc-500 transition-transform ${isFontOpen ? 'rotate-180' : ''}`} />
                        </button>

                        {isFontOpen && (
                            <div className="absolute top-full left-0 right-0 mt-2 bg-[#18181b] border border-white/10 rounded-xl shadow-2xl z-50 overflow-hidden animate-in fade-in zoom-in-95 duration-100 max-h-60 overflow-y-auto custom-scrollbar">
                                {fonts.map((font) => (
                                    <button
                                        key={font.id}
                                        onClick={() => {
                                            updateSettings({ uiFontFamily: font.id as any });
                                            setIsFontOpen(false);
                                        }}
                                        className="w-full flex items-center justify-between px-4 py-3 hover:bg-white/5 text-left text-sm transition-colors"
                                    >
                                        <span style={{ fontFamily: font.id }} className={uiFontFamily === font.id ? 'text-purple-400 font-bold' : 'text-zinc-300'}>
                                            {font.name}
                                        </span>
                                        {uiFontFamily === font.id && <Check size={14} className="text-purple-400" />}
                                    </button>
                                ))}
                            </div>
                        )}
                    </div>

                    {/* Font Size Slider */}
                    <div className="space-y-3">
                        <div className="flex justify-between">
                            <label className="text-xs font-bold text-zinc-500 uppercase tracking-wide">Interface Scale</label>
                            <span className="text-xs font-mono text-zinc-400">{uiFontSize}px</span>
                        </div>
                        <input
                            type="range"
                            min="12"
                            max="20"
                            value={uiFontSize || 14}
                            onChange={(e) => updateSettings({ uiFontSize: parseInt(e.target.value) })}
                            className="w-full h-1.5 bg-black/40 rounded-full appearance-none cursor-pointer [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:w-4 [&::-webkit-slider-thumb]:h-4 [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:bg-purple-500 hover:[&::-webkit-slider-thumb]:bg-purple-400"
                        />
                    </div>
                </div>
            </div>
        </div>
    );
}
