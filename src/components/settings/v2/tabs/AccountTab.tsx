import { useState, useEffect } from 'react';
import { loadStripe } from '@stripe/stripe-js';
import { User, Shield, Check, Star, Crown, LogOut, Copy } from 'lucide-react';
import { pb } from '../../../../lib/pocketbase';
import { encryptFile, decryptFile, encryptData, decryptData } from '../../../../services/SecurityService';
import AuthForms from '../../../auth/AuthForms';

interface AccountTabProps {
    user: any;
    onLogout: () => void;
    // Lifted State
    showAuth: boolean;
    setShowAuth: (show: boolean) => void;
}

// Pricing Configuration
const PLANS = [
    {
        id: 'free',
        name: 'Starter',
        price: 'Free',
        storage: '1GB',
        features: ['Cloud Sync', 'E2E Encryption', 'Basic Tools'],
        color: 'text-zinc-400',
        bg: 'bg-zinc-500/10',
        border: 'border-zinc-500/20',
        glow: 'shadow-zinc-500/40',
        hoverBorder: 'hover:border-zinc-500/50',
        icon: User
    },
    {
        id: 'basic',
        name: 'Basic',
        price: '$1',
        period: '/mo',
        storage: '25GB',
        features: ['All Starter Features', 'Priority Sync', 'Extended History'],
        color: 'text-emerald-400',
        bg: 'bg-emerald-500/10',
        border: 'border-emerald-500/20',
        glow: 'shadow-emerald-500/40',
        hoverBorder: 'hover:border-emerald-500/50',
        icon: Check
    },
    {
        id: 'pro',
        name: 'Pro',
        price: '$2',
        period: '/mo',
        storage: '50GB',
        features: ['All Basic Features', 'Advanced Search', '24/7 Support'],
        color: 'text-blue-400',
        bg: 'bg-blue-500/10',
        border: 'border-blue-500/20',
        glow: 'shadow-blue-500/40',
        hoverBorder: 'hover:border-blue-500/50',
        icon: Star
    },
    {
        id: 'ultra',
        name: 'Ultra',
        price: '$3',
        period: '/mo',
        storage: '200GB',
        features: ['All Pro Features', 'AI Assistant', 'Early Access'],
        color: 'text-purple-400',
        bg: 'bg-purple-500/10',
        border: 'border-purple-500/20',
        glow: 'shadow-purple-500/40',
        hoverBorder: 'hover:border-purple-500/50',
        icon: Crown
    }
];

export default function AccountTab({ user, onLogout, showAuth, setShowAuth }: AccountTabProps) {
    // Shared State / Auth Logic
    const [loadingPlan, setLoadingPlan] = useState<string | null>(null);

    // Guest State
    const [localName, setLocalName] = useState(() => localStorage.getItem('onyx_local_name') || 'Local User');
    const [localAvatar, setLocalAvatar] = useState<string | null>(() => localStorage.getItem('onyx_local_avatar'));
    const [isEditingName, setIsEditingName] = useState(false);

    // Save local name changes
    const handleLocalNameChange = (name: string) => {
        setLocalName(name);
        localStorage.setItem('onyx_local_name', name);
    };

    // Handle Local Avatar Upload (Base64 for simplicity, max 1MB)
    const handleLocalAvatarUpload = (e: React.ChangeEvent<HTMLInputElement>) => {
        const file = e.target.files?.[0];
        if (!file) return;

        if (file.size > 1024 * 1024) {
            alert("Image must be smaller than 1MB for local storage.");
            return;
        }

        const reader = new FileReader();
        reader.onloadend = () => {
            const result = reader.result as string;
            setLocalAvatar(result);
            localStorage.setItem('onyx_local_avatar', result);
        };
        reader.readAsDataURL(file);
    };

    // Authenticated State
    const [cloudName, setCloudName] = useState('');
    const [isEditingCloudName, setIsEditingCloudName] = useState(false);
    const [decryptedAvatar, setDecryptedAvatar] = useState<string | null>(null);

    // Decrypt Profile Data on Load
    useEffect(() => {
        if (!user) return;

        const loadProfile = async () => {
            const mk = sessionStorage.getItem('onyx_mk');
            if (mk) {
                // Decrypt Name
                const rawName = user.name || 'Onyx User';
                const realName = await decryptData(rawName, mk);
                setCloudName(realName);

                // Decrypt Avatar if exists
                if (user.avatar) {
                    try {
                        const url = `${import.meta.env.VITE_POCKETBASE_URL}/api/files/${user.collectionId}/${user.id}/${user.avatar}`;
                        const resp = await fetch(url);
                        const buffer = await resp.arrayBuffer();
                        const blobUrl = await decryptFile(buffer, mk);
                        setDecryptedAvatar(blobUrl);
                    } catch (e) {
                        console.error("Failed to decrypt avatar:", e);
                    }
                }
            } else {
                setCloudName(user.name || 'Locked');
            }
        };

        loadProfile();
    }, [user]);

    const handleSubscribe = async (planId: string) => {
        if (!import.meta.env.VITE_STRIPE_PUBLISHABLE_KEY) {
            console.error("Stripe key missing");
            return;
        }

        setLoadingPlan(planId);
        try {
            const stripe = await loadStripe(import.meta.env.VITE_STRIPE_PUBLISHABLE_KEY);
            const response = await fetch(`${import.meta.env.VITE_POCKETBASE_URL}/api/create-checkout-session`, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                    'Authorization': pb.authStore.token
                },
                body: JSON.stringify({
                    priceId: planId,
                    userId: user.id
                })
            });

            if (response.ok) {
                const session = await response.json();
                if (stripe) {
                    await (stripe as any).redirectToCheckout({ sessionId: session.id });
                }
            } else {
                console.warn("Backend not ready, mocking selection");
            }
        } catch (error) {
            console.error('Subscription error:', error);
        } finally {
            setLoadingPlan(null);
        }
    };

    const [hasCopiedId, setHasCopiedId] = useState(false);

    const copyUserId = () => {
        if (user?.id) {
            navigator.clipboard.writeText(user.id);
            setHasCopiedId(true);
            setTimeout(() => setHasCopiedId(false), 2000);
        }
    };

    if (!user) {
        if (showAuth) {
            // FULL SCREEN TAKEOVER for Auth
            return (
                <div className="h-full flex flex-col items-center justify-center animate-in fade-in zoom-in-95 duration-500">
                    <div className="w-full max-w-lg">
                        <AuthForms onCancel={() => setShowAuth(false)} onSuccess={() => { }} />
                    </div>
                </div>
            )
        }

        // --- GUEST / LOCAL VIEW ---
        return (
            <div className="space-y-8 animate-in fade-in slide-in-from-bottom-4 duration-500">
                {/* Local Profile Header */}
                <div className="flex items-center justify-between">
                    <div>
                        <h2 className="text-2xl font-bold text-white mb-1">Account & Profile</h2>
                        <p className="text-zinc-400 text-sm">Manage your local identity.</p>
                    </div>
                </div>

                {/* Profile Card - Beautified - No Border, No Shadow Box */}
                <div className="p-6 bg-white/5 rounded-3xl flex items-center gap-6 group transition-all duration-300 relative overflow-hidden backdrop-blur-md hover:scale-[1.02]">
                    <div className="absolute inset-0 bg-gradient-to-br from-purple-500/5 via-blue-500/5 to-transparent pointer-events-none group-hover:opacity-100 transition-opacity duration-500" />
                    <div className="absolute top-0 right-0 w-64 h-64 bg-purple-500/5 blur-[80px] rounded-full pointer-events-none group-hover:bg-purple-500/10 transition-all duration-500" />

                    <div
                        className="w-20 h-20 bg-gradient-to-br from-indigo-500 to-purple-600 rounded-2xl flex items-center justify-center shadow-lg shadow-purple-500/20 relative group/localavatar cursor-pointer overflow-hidden z-10 hover:scale-105 transition-transform duration-300"
                        onClick={() => document.getElementById('local-avatar-upload')?.click()}
                    >
                        {localAvatar ? (
                            <img src={localAvatar} alt="Local Avatar" className="w-full h-full object-cover" />
                        ) : (
                            <User size={32} className="text-white/80 group-hover/localavatar:text-white transition-colors" />
                        )}
                        <div className="absolute inset-0 bg-black/40 opacity-0 group-hover/localavatar:opacity-100 transition-opacity flex items-center justify-center">
                            <span className="text-[10px] uppercase font-bold text-white">Upload</span>
                        </div>
                        <input
                            id="local-avatar-upload"
                            type="file"
                            accept="image/*"
                            className="hidden"
                            onChange={handleLocalAvatarUpload}
                        />
                    </div>

                    <div className="flex-1 space-y-2 z-10">
                        <div className="flex items-center gap-3">
                            {isEditingName ? (
                                <input
                                    autoFocus
                                    className="bg-transparent text-2xl font-bold text-white focus:outline-none border-b border-purple-500/50 mb-1 w-full max-w-[200px]"
                                    value={localName}
                                    onChange={(e) => setLocalName(e.target.value)}
                                    onBlur={() => {
                                        handleLocalNameChange(localName);
                                        setIsEditingName(false);
                                    }}
                                    onKeyDown={(e) => e.key === 'Enter' && e.currentTarget.blur()}
                                />
                            ) : (
                                <h2
                                    className="text-2xl font-bold text-white mb-1 hover:text-purple-300 cursor-pointer transition-all duration-300 flex items-center gap-2 group/name hover:scale-105 origin-left"
                                    onClick={() => setIsEditingName(true)}
                                >
                                    {localName}
                                    <span className="opacity-0 group-hover/name:opacity-100 text-[10px] font-normal text-zinc-500 uppercase tracking-wider border border-zinc-700 px-1.5 py-0.5 rounded ml-2">Edit</span>
                                </h2>
                            )}

                        </div>
                        <p className="text-sm text-zinc-400 font-medium">Settings are stored locally on this device.</p>
                    </div>
                </div>

                {/* Cloud Promo / Connect Section (Vibrant Hero) - Compact Version */}
                <div
                    className="relative rounded-[40px] overflow-hidden transition-all duration-300 group/hero shadow-2xl shadow-black/50 shimmer-trigger hover:scale-[1.02] backface-hidden transform-gpu"
                >
                    <div className="absolute inset-0 bg-gradient-to-br from-purple-900/20 via-zinc-900/90 to-black/95 backdrop-blur-xl group-hover/hero:bg-zinc-900/90 transition-colors duration-500" />
                    {/* Reduced Glow */}
                    <div className="absolute top-0 right-0 w-96 h-96 bg-purple-500/5 blur-[150px] rounded-full pointer-events-none group-hover/hero:bg-purple-500/5 transition-all duration-1000 group-hover/hero:translate-x-10 group-hover/hero:-translate-y-10" />

                    <div className="relative z-10 py-11 px-10 text-center space-y-8">
                        <div className="animate-in fade-in slide-in-from-bottom-4 duration-500 space-y-8 pointer-events-none">
                            {/* Icon Container - No Inset Shadow (Clean) */}
                            <div className="w-24 h-24 bg-gradient-to-tr from-purple-500/10 to-blue-500/10 rounded-3xl flex items-center justify-center mx-auto border border-white/5 relative overflow-hidden group/icon pointer-events-auto hover:bg-purple-500/10 hover:border-purple-500/20 transition-all duration-300">
                                <div className="absolute inset-0 bg-gradient-to-tr from-purple-500/20 via-transparent to-blue-500/20 opacity-0 group-hover/icon:opacity-100 transition-opacity duration-700" />

                                {/* Animated Rings - Only on Icon Hover */}
                                <div className="absolute inset-0 border border-purple-500/20 rounded-3xl scale-75 opacity-0 group-hover/icon:scale-110 group-hover/icon:opacity-100 transition-all duration-[2500ms] delay-100" />
                                <div className="absolute inset-0 border border-blue-500/20 rounded-3xl scale-50 opacity-0 group-hover/icon:scale-125 group-hover/icon:opacity-100 transition-all duration-[2500ms] delay-200" />

                                <Crown size={40} className="text-white z-10 relative" />
                            </div>

                            <div className="max-w-2xl mx-auto space-y-4 pointer-events-auto">
                                <h3 className="text-3xl font-bold tracking-tight premium-shimmer inline-block">
                                    Unlock Your Identity
                                </h3>
                                <p className="text-zinc-400 text-sm leading-relaxed font-medium group-hover/hero:text-zinc-300 transition-colors">
                                    Securely sync your notes across devices with <span className="text-emerald-400 font-bold">Zero-Knowledge Encryption</span>.
                                </p>
                            </div>

                            <div className="flex justify-center pt-2">
                                <button
                                    onClick={() => setShowAuth(true)}
                                    className="pointer-events-auto px-10 py-4 bg-white text-black rounded-full text-sm font-bold shadow-[0_0_20px_rgba(255,255,255,0.1)] flex items-center gap-2 hover:bg-white hover:scale-105 active:scale-95 transition-all duration-300"
                                >
                                    <Crown size={16} /> Begin Journey
                                </button>
                            </div>
                        </div>
                    </div>
                </div>

            </div>
        );
    }

    // --- AUTHENTICATED VIEW ---
    return (
        <div className="space-y-8 animate-in fade-in slide-in-from-bottom-4 duration-500">

            {/* Account Header */}
            <div className="p-6 bg-zinc-900/50 rounded-3xl flex items-center gap-6 relative overflow-hidden group">
                <div className="absolute top-0 right-0 w-64 h-64 bg-purple-500/5 blur-[100px] rounded-full pointer-events-none group-hover:bg-purple-500/10 transition-all duration-1000" />

                <div className="w-20 h-20 bg-gradient-to-br from-zinc-800 to-zinc-900 rounded-2xl flex items-center justify-center border border-white/10 shadow-xl z-10 relative group/avatar cursor-pointer overflow-hidden">
                    {decryptedAvatar ? (
                        <img
                            src={decryptedAvatar}
                            alt="Avatar"
                            className="w-full h-full object-cover"
                        />
                    ) : (
                        <User size={32} className="text-zinc-400" />
                    )}
                    <div className="absolute inset-0 bg-black/50 opacity-0 group-hover/avatar:opacity-100 transition-opacity flex items-center justify-center" onClick={() => document.getElementById('avatar-upload')?.click()}>
                        <span className="text-[10px] uppercase font-bold text-white">Upload</span>
                    </div>
                    <input
                        id="avatar-upload"
                        type="file"
                        accept="image/*"
                        className="hidden"
                        onChange={async (e) => {
                            if (e.target.files?.[0]) {
                                try {
                                    const mk = sessionStorage.getItem('onyx_mk');
                                    if (!mk) return alert("Session Locked.");

                                    const file = e.target.files[0];
                                    // Encrypt File
                                    const encryptedFile = await encryptFile(file, mk);

                                    // Spoof as image/png for PocketBase validation
                                    const spoofedFile = new File([encryptedFile], "avatar.png", { type: "image/png" });

                                    const formData = new FormData();
                                    formData.append('avatar', spoofedFile);

                                    const updated = await pb.collection('users').update(user.id, formData);
                                    pb.authStore.save(pb.authStore.token, updated);

                                    // Update local view
                                    const reader = new FileReader();
                                    reader.onload = (ev) => setDecryptedAvatar(ev.target?.result as string);
                                    reader.readAsDataURL(file);

                                } catch (err) {
                                    console.error("Avatar upload failed", err);
                                }
                            }
                        }}
                    />
                </div>

                <div className="flex-1 z-10">
                    {isEditingCloudName ? (
                        <input
                            autoFocus
                            className="bg-transparent text-2xl font-bold text-white focus:outline-none border-b border-purple-500/50 mb-1 w-full max-w-[200px]"
                            value={cloudName}
                            onChange={(e) => setCloudName(e.target.value)}
                            onBlur={async () => {
                                // Encrypt and Save
                                if (cloudName) {
                                    try {
                                        const mk = sessionStorage.getItem('onyx_mk');
                                        if (mk) {
                                            const encryptedName = await encryptData(cloudName, mk);
                                            const updated = await pb.collection('users').update(user.id, { name: encryptedName });
                                            pb.authStore.save(pb.authStore.token, updated);
                                        }
                                    } catch (err) {
                                        console.error("Name update failed", err);
                                    }
                                }
                                setIsEditingCloudName(false);
                            }}
                            onKeyDown={(e) => e.key === 'Enter' && e.currentTarget.blur()}
                        />
                    ) : (
                        <h2
                            className="text-2xl font-bold text-white mb-1 hover:text-purple-400 cursor-pointer transition-colors flex items-center gap-2 group/name"
                            onClick={() => {
                                // Don't reset to user.name here, keep cloudName (which is decrypted)
                                setIsEditingCloudName(true);
                            }}
                        >
                            {cloudName || 'Onyx User'}
                            <span className="opacity-0 group-hover/name:opacity-100 text-[10px] font-normal text-zinc-500 uppercase tracking-wider border border-zinc-700 px-1.5 py-0.5 rounded ml-2">Edit</span>
                        </h2>
                    )}
                    <button
                        onClick={copyUserId}
                        className="text-sm text-zinc-500 font-mono mb-3 hover:text-purple-400 transition-colors flex items-center gap-2 group/uid cursor-pointer"
                        title="Click to copy User ID"
                    >
                        ID: {user.id?.slice(0, 10)}...
                        <Copy size={12} className={`opacity-0 group-hover/uid:opacity-100 transition-opacity ${hasCopiedId ? 'text-emerald-400' : ''}`} />
                        {hasCopiedId && <span className="text-[10px] text-emerald-400 font-bold">Copied!</span>}
                    </button>
                    <div className="flex gap-2">
                        <span className="px-2 py-1 bg-emerald-500/10 border border-emerald-500/20 rounded-lg text-[10px] font-bold text-emerald-400 uppercase tracking-wider flex items-center gap-1.5">
                            <Shield size={10} /> Zero-Knowledge
                        </span>
                        <span className="px-2 py-1 bg-zinc-800 border border-white/5 rounded-lg text-[10px] font-bold text-zinc-500 uppercase tracking-wider">
                            Free Plan
                        </span>
                    </div>
                </div>

                <button
                    onClick={onLogout}
                    className="z-10 px-5 py-2.5 bg-red-500/10 hover:bg-red-500/20 text-red-400 border border-red-500/20 rounded-xl text-xs font-bold uppercase tracking-wider transition-all flex items-center gap-2"
                >
                    <LogOut size={16} /> Sign Out
                </button>
            </div>

            {/* Storage Usage */}
            <div className="space-y-3">
                <div className="flex justify-between text-xs font-medium text-zinc-400 uppercase tracking-wider px-1">
                    <span>Cloud Storage</span>
                    <span>12% Used (120MB / 1GB)</span>
                </div>
                <div className="h-3 w-full bg-zinc-900 rounded-full overflow-hidden border border-white/5">
                    <div className="h-full w-[12%] bg-gradient-to-r from-purple-600 to-blue-500 rounded-full shadow-[0_0_15px_rgba(168,85,247,0.5)]" />
                </div>
            </div>

            {/* Plans Grid */}
            <div className="space-y-4">
                <h3 className="text-sm font-bold text-zinc-300 uppercase tracking-wider flex items-center gap-2">
                    <Crown size={16} /> Subscription Plan
                </h3>
                <div className="grid grid-cols-4 gap-4">
                    {PLANS.map((plan) => (
                        <div
                            key={plan.id}
                            className={`relative p-5 rounded-3xl border flex flex-col gap-4 group transition-all duration-300 hover:-translate-y-2 ${plan.bg} ${plan.border} ${plan.hoverBorder} hover:${plan.glow} hover:shadow-2xl`}
                        >
                            {plan.id === 'ultra' && (
                                <div className="absolute -top-3 left-1/2 -translate-x-1/2 px-3 py-1 bg-purple-500 text-white text-[9px] font-bold uppercase tracking-widest rounded-full shadow-lg shadow-purple-500/40">
                                    Ultimate
                                </div>
                            )}

                            {/* Local Badge Removed per previous request */}

                            <div className="flex justify-between items-start">
                                <div>
                                    <h4 className={`text-sm font-bold ${plan.color} uppercase tracking-wide`}>{plan.name}</h4>
                                    <div className="flex items-baseline gap-0.5 mt-1">
                                        <span className="text-2xl font-bold text-white">{plan.price}</span>
                                        {plan.period && <span className="text-xs text-zinc-500">{plan.period}</span>}
                                    </div>
                                </div>
                                <plan.icon size={18} className={`${plan.color} opacity-50 group-hover:opacity-100 transition-opacity`} />
                            </div>

                            <div className="text-xs font-bold text-zinc-300 bg-black/20 px-3 py-2 rounded-xl text-center border border-white/5">
                                {plan.storage} Storage
                            </div>

                            <ul className="space-y-2 flex-1">
                                {plan.features.map((feat, i) => (
                                    <li key={i} className="text-[10px] text-zinc-400 flex items-center gap-2">
                                        <Check size={10} className={plan.color} />
                                        {feat}
                                    </li>
                                ))}
                            </ul>

                            <button
                                disabled={loadingPlan !== null}
                                onClick={() => handleSubscribe(plan.id)}
                                className={`w-full py-2.5 rounded-xl text-[10px] font-bold uppercase tracking-widest transition-all ${plan.id === 'free'
                                    ? 'bg-zinc-800 text-zinc-400 border border-white/5 cursor-default'
                                    : 'bg-white text-black hover:scale-[1.02] shadow-lg active:scale-95'
                                    }`}
                            >
                                {plan.id === 'free' ? 'Current Plan' : loadingPlan === plan.id ? 'Processing...' : 'Upgrade'}
                            </button>
                        </div>
                    ))}
                </div>

                <div className="flex justify-center pt-2">
                    <button className="text-[10px] text-zinc-500 hover:text-zinc-300 underline decoration-zinc-800 underline-offset-4 transition-colors">
                        Manage billing and invoices via Stripe
                    </button>
                </div>
            </div>

        </div>
    );
}
