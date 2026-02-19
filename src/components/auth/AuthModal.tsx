import { X } from 'lucide-react'
import AuthForms from './AuthForms'

interface AuthModalProps {
    isOpen: boolean;
    onClose: () => void;
}

export default function AuthModal({ isOpen, onClose }: AuthModalProps) {
    if (!isOpen) return null;

    return (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4 animate-in fade-in duration-300">
            {/* Breathing Backdrop */}
            <div
                className="absolute inset-0 bg-black/60 backdrop-blur-xl transition-all"
                onClick={onClose}
            />

            {/* Ambient Background Orbs */}
            <div className="absolute inset-0 overflow-hidden pointer-events-none">
                <div className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[500px] h-[500px] bg-purple-600/20 rounded-full blur-[100px] animate-pulse" />
                <div className="absolute top-1/4 left-1/4 w-[300px] h-[300px] bg-blue-600/10 rounded-full blur-[80px]" />
                <div className="absolute bottom-1/4 right-1/4 w-[300px] h-[300px] bg-emerald-600/10 rounded-full blur-[80px]" />
            </div>

            {/* Modal Content - Flush & Glassy */}
            <div className="relative w-full max-w-[420px] bg-zinc-950/40 border border-white/10 rounded-[2rem] p-8 shadow-2xl animate-in zoom-in-95 slide-in-from-bottom-5 duration-300 backdrop-blur-md">
                <button
                    onClick={onClose}
                    className="absolute top-5 right-5 p-2 text-zinc-500 hover:text-white transition-colors rounded-full hover:bg-white/5 z-50"
                >
                    <X size={20} />
                </button>

                <AuthForms
                    onSuccess={onClose}
                    onCancel={onClose}
                />
            </div>
        </div>
    )
}
