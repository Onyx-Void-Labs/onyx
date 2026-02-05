
import React, { useState, useEffect } from 'react';
import { X, Lock, Eye, EyeOff } from 'lucide-react';

interface LockModalProps {
    isOpen: boolean;
    onClose: () => void;
    onConfirm: (password: string) => void;
    noteTitle: string;
}

export default function LockModal({ isOpen, onClose, onConfirm, noteTitle }: LockModalProps) {
    const [password, setPassword] = useState('');
    const [confirm, setConfirm] = useState('');
    const [error, setError] = useState('');
    const [showPassword, setShowPassword] = useState(false);
    const [strength, setStrength] = useState(0); // 0-4

    useEffect(() => {
        // Simple strength calc
        let s = 0;
        if (password.length > 4) s++;
        if (password.length > 8) s++;
        if (/[A-Z]/.test(password)) s++;
        if (/[0-9]/.test(password)) s++;
        if (/[^A-Za-z0-9]/.test(password)) s++;
        setStrength(Math.min(s, 4));
    }, [password]);

    if (!isOpen) return null;

    const handleSubmit = (e: React.FormEvent) => {
        e.preventDefault();
        setError('');

        if (password.length < 4) {
            setError('Password is too short (min 4 chars)');
            return;
        }

        if (password !== confirm) {
            setError('Passwords do not match');
            return;
        }

        onConfirm(password);
        // Reset
        setPassword('');
        setConfirm('');
    };

    return (
        <div className="fixed inset-0 z-[60] flex items-center justify-center bg-black/60 backdrop-blur-sm animate-in fade-in duration-200">
            <div className="w-[400px] bg-zinc-900 border border-zinc-800 rounded-xl shadow-2xl overflow-hidden animate-in zoom-in-95 duration-200">
                {/* Header */}
                <div className="h-14 flex items-center justify-between px-6 border-b border-zinc-800 bg-zinc-900/50">
                    <div className="flex items-center gap-2 text-zinc-100 font-medium">
                        <Lock size={16} className="text-purple-400" />
                        <span>Lock Note</span>
                    </div>
                    <button onClick={onClose} className="text-zinc-500 hover:text-zinc-300 transition-colors">
                        <X size={18} />
                    </button>
                </div>

                {/* Body */}
                <div className="p-6">
                    <p className="text-sm text-zinc-400 mb-6 leading-relaxed">
                        Encrypting <span className="text-zinc-200 font-semibold">{noteTitle || 'Untitled'}</span>.
                        <br />
                        <span className="text-red-400/90 text-xs mt-1 block">
                            ⚠️ Warning: If you forget this password, the content is lost forever. There is no recovery.
                        </span>
                    </p>

                    <form onSubmit={handleSubmit} className="space-y-4">
                        <div className="space-y-1">
                            <label className="text-xs font-medium text-zinc-500 uppercase tracking-wider flex justify-between">
                                <span>Password</span>
                                <span className={`text-[10px] ${strength < 2 ? 'text-red-400' : strength < 4 ? 'text-yellow-400' : 'text-green-400'
                                    }`}>
                                    {strength === 0 ? '' : strength < 2 ? 'Weak' : strength < 4 ? 'Medium' : 'Strong'}
                                </span>
                            </label>
                            <div className="relative">
                                <input
                                    type={showPassword ? "text" : "password"}
                                    autoFocus
                                    value={password}
                                    onChange={e => setPassword(e.target.value)}
                                    className="w-full bg-zinc-950 border border-zinc-800 rounded-lg px-3 py-2 pr-10 text-zinc-200 outline-none focus:border-purple-500/50 focus:ring-1 focus:ring-purple-500/50 transition-all"
                                    placeholder="Enter strong password"
                                />
                                <button
                                    type="button"
                                    onClick={() => setShowPassword(!showPassword)}
                                    className="absolute right-2 top-2 text-zinc-600 hover:text-zinc-400 transition-colors"
                                >
                                    {showPassword ? <EyeOff size={16} /> : <Eye size={16} />}
                                </button>
                            </div>

                            {/* Strength Meter Bar */}
                            <div className="h-1 w-full bg-zinc-800 rounded-full overflow-hidden mt-1 flex gap-0.5">
                                <div className={`h-full flex-1 rounded-full transition-all duration-300 ${strength >= 1 ? 'bg-red-500' : 'opacity-0'}`} />
                                <div className={`h-full flex-1 rounded-full transition-all duration-300 ${strength >= 2 ? 'bg-yellow-500' : 'opacity-0'}`} />
                                <div className={`h-full flex-1 rounded-full transition-all duration-300 ${strength >= 3 ? 'bg-green-500' : 'opacity-0'}`} />
                                <div className={`h-full flex-1 rounded-full transition-all duration-300 ${strength >= 4 ? 'bg-emerald-400' : 'opacity-0'}`} />
                            </div>
                        </div>

                        <div className="space-y-1">
                            <label className="text-xs font-medium text-zinc-500 uppercase tracking-wider">Confirm Password</label>
                            <input
                                type={showPassword ? "text" : "password"}
                                value={confirm}
                                onChange={e => setConfirm(e.target.value)}
                                className="w-full bg-zinc-950 border border-zinc-800 rounded-lg px-3 py-2 text-zinc-200 outline-none focus:border-purple-500/50 focus:ring-1 focus:ring-purple-500/50 transition-all"
                                placeholder="Repeat password"
                            />
                        </div>

                        {error && (
                            <div className="text-xs text-red-400 font-medium bg-red-500/10 px-3 py-2 rounded-md">
                                {error}
                            </div>
                        )}

                        <div className="flex justify-end pt-4 gap-2">
                            <button
                                type="button"
                                onClick={onClose}
                                className="px-4 py-2 text-sm text-zinc-400 hover:text-zinc-200 hover:bg-zinc-800 rounded-lg transition-colors"
                            >
                                Cancel
                            </button>
                            <button
                                type="submit"
                                disabled={!password || !confirm}
                                className="px-4 py-2 text-sm font-medium bg-purple-600 hover:bg-purple-500 text-white rounded-lg shadow-lg shadow-purple-900/20 transition-all disabled:opacity-50 disabled:cursor-not-allowed"
                            >
                                Encrypt & Lock
                            </button>
                        </div>
                    </form>
                </div>
            </div>
        </div>
    );
}
