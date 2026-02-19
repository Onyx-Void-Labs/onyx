import { Heart } from 'lucide-react';

export default function AboutTab() {
    return (
        <div className="space-y-6 animate-in fade-in slide-in-from-bottom-4 duration-500">
            <h2 className="text-2xl font-bold text-white mb-1">About Onyx</h2>

            <div className="p-8 bg-zinc-900/50 border border-white/5 rounded-3xl flex flex-col items-center justify-center text-center space-y-6">
                <div className="w-20 h-20 bg-gradient-to-br from-indigo-500 to-purple-600 rounded-3xl flex items-center justify-center shadow-2xl shadow-purple-500/20">
                    <span className="text-4xl font-bold text-white">O</span>
                </div>

                <div>
                    <h3 className="text-2xl font-bold text-white">Onyx</h3>
                    <p className="text-zinc-500 font-mono text-xs mt-1">Version 0.0.3-alpha</p>
                </div>

                <p className="text-sm text-zinc-400 max-w-md leading-relaxed">
                    Designed for speed, security, and simplicity. <br />
                    Onyx is built with a Zero-Knowledge architecture to ensure your thoughts remain yours alone.
                </p>

                <div className="flex gap-2">
                    <span className="px-3 py-1 bg-white/5 rounded-full text-[10px] text-zinc-500 border border-white/5">React</span>
                    <span className="px-3 py-1 bg-white/5 rounded-full text-[10px] text-zinc-500 border border-white/5">Tauri</span>
                    <span className="px-3 py-1 bg-white/5 rounded-full text-[10px] text-zinc-500 border border-white/5">Rust</span>
                    <span className="px-3 py-1 bg-white/5 rounded-full text-[10px] text-zinc-500 border border-white/5">PocketBase</span>
                </div>

                <div className="pt-8 text-xs text-zinc-600 flex items-center gap-1.5">
                    Made with <Heart size={12} className="text-red-500/50 fill-red-500/50" /> by Omar
                </div>
            </div>
        </div>
    );
}
