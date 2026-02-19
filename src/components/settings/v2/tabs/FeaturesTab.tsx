import { Zap } from 'lucide-react';

export default function FeaturesTab() {
    return (
        <div className="space-y-6 animate-in fade-in slide-in-from-bottom-4 duration-500">
            <h2 className="text-2xl font-bold text-white mb-1">Experimental Features</h2>
            <p className="text-zinc-400 text-sm">Test drive upcoming capabilities.</p>

            <div className="p-8 bg-zinc-900/50 border border-white/5 rounded-3xl flex flex-col items-center justify-center text-center space-y-4">
                <div className="w-16 h-16 bg-purple-500/10 rounded-2xl flex items-center justify-center text-purple-400">
                    <Zap size={32} />
                </div>
                <div>
                    <h3 className="text-lg font-bold text-zinc-300">Lab Closed</h3>
                    <p className="text-sm text-zinc-500 max-w-sm mt-2">
                        There are no experimental features available for testing at the moment. Check back later!
                    </p>
                </div>
            </div>
        </div>
    );
}
