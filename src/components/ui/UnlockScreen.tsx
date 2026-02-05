
import React, { useState } from 'react';
import { Lock, Unlock, ArrowRight, Eye, EyeOff } from 'lucide-react';

interface UnlockScreenProps {
    onUnlock: (password: string) => Promise<boolean>;
    title?: string;
}

export default function UnlockScreen({ onUnlock, title }: UnlockScreenProps) {
    const [password, setPassword] = useState('');
    const [isLoading, setIsLoading] = useState(false);
    const [error, setError] = useState(false);
    const [showPassword, setShowPassword] = useState(false);

    const handleSubmit = async (e?: React.FormEvent) => {
        if (e) e.preventDefault();
        if (!password) return;

        setIsLoading(true);
        setError(false);

        // Slight artificial delay for "security feel" + nice animation
        await new Promise(r => setTimeout(r, 500));

        const success = await onUnlock(password);
        if (!success) {
            setError(true);
            setIsLoading(false);
        }
        // If success, parent will likely unmount us
    };

    return (
        <div className="absolute inset-0 z-50 flex items-center justify-center bg-zinc-950/80 backdrop-blur-md animate-in fade-in duration-300">
            <div className={`
                w-[350px] bg-zinc-900 border border-zinc-800 rounded-2xl shadow-2xl p-8 flex flex-col items-center gap-6
                transition-transform duration-100 ease-in-out
                ${error ? 'translate-x-[-5px] border-red-500/50' : ''}
            `}>
                {/* Icon */}
                <div className={`
                    w-16 h-16 rounded-full flex items-center justify-center mb-2 transition-colors duration-500
                    ${error ? 'bg-red-500/10 text-red-400' : 'bg-purple-500/10 text-purple-400'}
                `}>
                    {isLoading ? <Unlock className="animate-pulse" size={32} /> : <Lock size={32} />}
                </div>

                {/* Text */}
                <div className="text-center">
                    <h2 className="text-xl font-bold text-zinc-100 mb-1">
                        {title || 'Protected Note'}
                    </h2>
                    <p className="text-sm text-zinc-500">
                        {error ? 'Incorrect password. Try again.' : 'Enter your password to unlock this note.'}
                    </p>
                </div>

                {/* Input Form */}
                <form onSubmit={handleSubmit} className="w-full relative group">
                    <div className="relative">
                        <input
                            type={showPassword ? "text" : "password"}
                            value={password}
                            onChange={(e) => {
                                setPassword(e.target.value);
                                if (error) setError(false);
                            }}
                            autoFocus
                            placeholder="Password"
                            className={`
                                w-full bg-zinc-950 border border-zinc-800 rounded-xl px-4 py-3 pr-20 text-zinc-200 outline-none
                                focus:border-purple-500/50 focus:ring-1 focus:ring-purple-500/50 transition-all
                                placeholder:text-zinc-700
                                ${error ? 'border-red-500/30 focus:border-red-500/50 focus:ring-red-500/20' : ''}
                            `}
                        />
                        <button
                            type="button"
                            onClick={() => setShowPassword(!showPassword)}
                            className="absolute right-10 top-3 text-zinc-600 hover:text-zinc-400 transition-colors"
                        >
                            {showPassword ? <EyeOff size={18} /> : <Eye size={18} />}
                        </button>
                    </div>
                    <button
                        type="submit"
                        disabled={isLoading || !password}
                        className="absolute right-2 top-2 p-1.5 bg-zinc-800 hover:bg-zinc-700 text-zinc-400 hover:text-white rounded-lg transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                    >
                        <ArrowRight size={16} />
                    </button>
                </form>
            </div>
        </div>
    );
}
