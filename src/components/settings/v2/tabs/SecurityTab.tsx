import { useState, useEffect } from 'react';
import { Shield, Lock, Smartphone, FileKey, Server, AlertTriangle } from 'lucide-react';
import { generateSecret, generateURI, verify } from 'otplib';
// @ts-ignore
import QRCode from 'qrcode';
import { pb } from '../../../../lib/pocketbase';
import { encryptNote, KeyRotationService } from '../../../../services/SecurityService';
import { RefreshCw, CheckCircle2 } from 'lucide-react';

export default function SecurityTab() {
    const [mfaEnabled, setMfaEnabled] = useState(false);

    // MFA State
    const [showMfaSetup, setShowMfaSetup] = useState(false);
    const [mfaSecret, setMfaSecret] = useState('');
    const [qrCodeUrl, setQrCodeUrl] = useState('');
    const [mfaCode, setMfaCode] = useState('');
    const [error, setError] = useState<string | null>(null);

    // Rotation State
    const [rotating, setRotating] = useState(false);
    const [showRotationConfirm, setShowRotationConfirm] = useState(false);


    // Check initial state
    useEffect(() => {
        if (pb.authStore.model?.mfa_enabled) {
            setMfaEnabled(true);
        }
    }, []);

    const handleSetupMfa = async () => {
        const secret = generateSecret();
        setMfaSecret(secret);

        const user = pb.authStore.model;
        const otpauth = generateURI({
            secret,
            label: user?.email || 'Onyx User',
            issuer: 'Onyx',
            algorithm: 'sha1',
        });

        try {
            const url = await QRCode.toDataURL(otpauth);
            setQrCodeUrl(url);
            setShowMfaSetup(true);
            setError(null);
        } catch (err) {
            console.error(err);
            setError("Failed to generate QR Code");
        }
    };

    const handleVerifyMfa = async () => {
        try {
            // verify is async in functional API (based on index.js)
            const isValid = await verify({ token: mfaCode, secret: mfaSecret });
            if (!isValid) {
                setError("Invalid Code");
                return;
            }

            // Encrypt secret with Master Key
            const mk = sessionStorage.getItem('onyx_mk');
            if (!mk) {
                setError("Session locked. Please re-login.");
                return;
            }

            const encryptedSecret = await encryptNote(mfaSecret, mk);

            await pb.collection('users').update(pb.authStore.model?.id!, {
                mfa_enabled: true,
                mfa_secret: JSON.stringify(encryptedSecret)
            });

            setMfaEnabled(true);
            setShowMfaSetup(false);
            setMfaSecret('');
            setMfaCode('');
            setError(null);
            setQrCodeUrl('');
        } catch (err: any) {
            console.error("MFA Verify Error:", err);
            setError(err.message || "Failed to enable MFA");
        }
    };

    const handleDisableMfa = async () => {
        try {
            if (!confirm("Are you sure you want to disable Two-Factor Authentication?")) return;

            const user = pb.authStore.model;
            if (!user) {
                setError("User not logged in.");
                return;
            }

            await pb.collection('users').update(user.id, {
                mfa_enabled: false,
                mfa_secret: ''
            });
            setMfaEnabled(false);
            setError(null);
        } catch (err) {
            console.error("MFA Disable Error:", err);
            setError("Failed to disable MFA.");
        }
    };

    const handleRotateKey = async () => {
        setRotating(true);
        setError(null);
        try {
            const mk = sessionStorage.getItem('onyx_mk');
            const user = pb.authStore.model;
            if (!mk || !user) throw new Error("Session invalid");

            // Reuse existing salt (limit of current user model)
            const salt = user.enc_salt;
            const { mnemonic, keyWrappedRk, recoveryHash } = await KeyRotationService.rotateRecoveryKeyWithSalt(mk, salt);

            // Update user
            await pb.collection('users').update(user.id, {
                key_wrapped_rk: keyWrappedRk,
                recovery_hash: recoveryHash
            });


            // Open modal or alert with NEW mnemonic
            alert(`NEW RECOVERY PHRASE:\n\n${mnemonic}\n\nSAVE THIS NOW. The old one is invalid.`);

            setShowRotationConfirm(false);

        } catch (e: any) {
            console.error("Rotation failed", e);
            setError("Failed to rotate key: " + e.message);
        } finally {
            setRotating(false);
        }
    };

    return (
        <div className="space-y-8 animate-in fade-in slide-in-from-bottom-4 duration-500">
            {/* Header */}
            <div>
                <h2 className="text-2xl font-bold text-white mb-1">Security & Privacy</h2>
                <p className="text-zinc-400 text-sm">Manage your encryption keys and account security.</p>
            </div>

            <div className="space-y-4">
                <h3 className="text-sm font-bold text-zinc-400 uppercase tracking-widest flex items-center gap-2">
                    <Lock size={14} /> Account Security
                </h3>

                {/* Security Cards Grid */}
                <div className="grid grid-cols-1 lg:grid-cols-2 gap-6 items-start">

                    {/* 2FA Section - Redesigned */}
                    <div className={`relative p-6 rounded-3xl border transition-all duration-500 overflow-hidden group ${mfaEnabled
                        ? 'bg-emerald-900/10 border-emerald-500/20 shadow-[0_0_30px_rgba(16,185,129,0.1)]'
                        : 'bg-red-900/5 border-red-500/10 hover:border-red-500/20'
                        }`}>
                        {mfaEnabled && (
                            <div className="absolute inset-0 bg-emerald-500/5 animate-pulse" />
                        )}

                        <div className="relative z-10 flex items-center justify-between">
                            <div className="flex items-center gap-4">
                                <div className={`w-12 h-12 rounded-2xl flex items-center justify-center transition-colors duration-500 ${mfaEnabled
                                    ? 'bg-emerald-500/20 text-emerald-400 shadow-[0_0_15px_rgba(16,185,129,0.2)]'
                                    : 'bg-red-500/10 text-red-400'
                                    }`}>
                                    <Smartphone size={24} />
                                </div>
                                <div>
                                    <div className={`font-bold transition-colors duration-300 ${mfaEnabled ? 'text-emerald-100' : 'text-zinc-200'}`}>
                                        Two-Factor Auth
                                    </div>
                                    <div className="text-xs text-zinc-500 mt-0.5 flex items-center gap-1.5">
                                        <div className={`w-1.5 h-1.5 rounded-full ${mfaEnabled ? 'bg-emerald-500 animate-pulse' : 'bg-red-500'}`} />
                                        {mfaEnabled ? 'Active & Protected' : 'Disabled'}
                                    </div>
                                </div>
                            </div>

                            <button
                                onClick={() => {
                                    if (showMfaSetup) {
                                        setShowMfaSetup(false);
                                        setQrCodeUrl('');
                                        setMfaSecret('');
                                        setMfaCode('');
                                    } else if (mfaEnabled) {
                                        handleDisableMfa();
                                    } else {
                                        handleSetupMfa();
                                    }
                                }}
                                className={`px-4 py-2 rounded-xl text-xs font-bold transition-all border ${showMfaSetup
                                    ? 'bg-zinc-800 border-zinc-700 text-zinc-300 hover:bg-zinc-700'
                                    : mfaEnabled
                                        ? 'bg-red-500/10 border-red-500/20 text-red-400 hover:bg-red-500/20'
                                        : 'bg-emerald-500 text-black border-emerald-400 hover:bg-emerald-400 shadow-lg shadow-emerald-500/20'
                                    }`}
                            >
                                {showMfaSetup ? 'Cancel' : mfaEnabled ? 'Disable' : 'Enable 2FA'}
                            </button>
                        </div>

                        {!mfaEnabled && showMfaSetup && qrCodeUrl && (
                            <div className="mt-6 pt-6 border-t border-white/5 animate-in slide-in-from-top-2 fade-in relative z-10">
                                <div className="flex flex-col gap-6">
                                    <div className="flex flex-col sm:flex-row gap-6 items-start">
                                        <div className="bg-white p-2 rounded-xl w-fit h-fit shrink-0 shadow-xl">
                                            <img src={qrCodeUrl} alt="2FA QR Code" className="w-32 h-32" />
                                        </div>
                                        <div className="space-y-3 flex-1">
                                            <div>
                                                <h4 className="text-sm font-bold text-zinc-200">Scan QR Code</h4>
                                                <p className="text-xs text-zinc-400 mt-1">
                                                    Use Google Authenticator or Authy to scan this code.
                                                </p>
                                            </div>
                                        </div>
                                    </div>

                                    <div className="space-y-3">
                                        <label className="text-[10px] uppercase font-bold text-zinc-500 tracking-wider">Verification Code</label>
                                        <div className="flex flex-col sm:flex-row gap-3">
                                            <input
                                                type="text"
                                                maxLength={6}
                                                className="bg-black/40 border border-zinc-700 rounded-lg px-4 py-2.5 text-sm text-white focus:outline-none focus:border-emerald-500 w-full sm:w-32 font-mono tracking-widest text-center"
                                                placeholder="000 000"
                                                value={mfaCode}
                                                onChange={(e) => setMfaCode(e.target.value.replace(/[^0-9]/g, ''))}
                                            />
                                            <button
                                                onClick={handleVerifyMfa}
                                                className="px-6 py-2.5 bg-emerald-500 hover:bg-emerald-400 text-black rounded-lg text-xs font-bold transition-colors w-full sm:w-auto shadow-lg shadow-emerald-500/20"
                                            >
                                                Activate 2FA
                                            </button>
                                        </div>
                                        {error && <p className="text-red-400 text-[10px] font-bold animate-in fade-in slide-in-from-top-1 bg-red-500/10 px-3 py-2 rounded-lg border border-red-500/20">{error}</p>}
                                    </div>
                                </div>
                            </div>
                        )}
                    </div>

                    {/* Emergency Kit Section */}
                    <div className="p-6 bg-zinc-900/30 border border-amber-500/20 rounded-3xl flex flex-col justify-between h-full group hover:border-amber-500/40 transition-colors relative overflow-hidden">
                        <div className="absolute top-0 right-0 p-8 opacity-[0.03] group-hover:opacity-[0.05] transition-opacity pointer-events-none text-amber-500">
                            <FileKey size={120} />
                        </div>

                        <div className="flex items-center justify-between mb-4 relative z-10">
                            <div className="flex items-center gap-4">
                                <div className="w-12 h-12 bg-amber-500/10 rounded-2xl flex items-center justify-center text-amber-500 border border-amber-500/20 shadow-[0_0_15px_rgba(245,158,11,0.1)]">
                                    <FileKey size={24} />
                                </div>
                                <div>
                                    <div className="font-bold text-zinc-100 group-hover:text-amber-200 transition-colors">Emergency Kit</div>
                                    <div className="text-xs text-zinc-500 mt-0.5 group-hover:text-zinc-400">Account Recovery</div>
                                </div>
                            </div>
                            <button
                                onClick={() => setShowRotationConfirm(true)}
                                className="px-3 py-1.5 bg-zinc-800 text-zinc-400 border border-white/5 rounded-lg text-[10px] font-bold hover:bg-amber-500 hover:text-black hover:border-amber-500 transition-all flex items-center gap-2"
                            >
                                <RefreshCw size={12} className={rotating ? "animate-spin" : ""} />
                                Rotate
                            </button>
                        </div>

                        <p className="text-xs text-zinc-500 leading-relaxed max-w-sm mb-4 relative z-10 group-hover:text-zinc-400 transition-colors">
                            Your Emergency Kit is the <strong>only</strong> way to restore access if you forget your password. Onyx cannot recover it for you.
                        </p>

                        {/* Status Indicator */}
                        <div className="flex items-center gap-2 text-[10px] font-bold text-amber-500 uppercase tracking-wider relative z-10">
                            <div className="w-2 h-2 rounded-full bg-amber-500 animate-pulse" />
                            <span>Encrypted on Device</span>
                        </div>

                        {showRotationConfirm && (
                            <div className="absolute inset-0 z-20 bg-zinc-950/90 backdrop-blur-sm flex items-center justify-center p-6 animate-in fade-in duration-200">
                                <div className="w-full bg-zinc-900 border border-orange-500/30 rounded-2xl p-4 shadow-2xl">
                                    <div className="flex items-start gap-3 mb-3">
                                        <AlertTriangle className="text-orange-500 shrink-0 mt-0.5" size={18} />
                                        <div>
                                            <h4 className="font-bold text-orange-500 text-sm">Rotate Recovery Key?</h4>
                                            <p className="text-[11px] text-zinc-400 mt-1 leading-relaxed">
                                                This invalidates your old Emergency Kit immediately. You will need to save the new one.
                                            </p>
                                        </div>
                                    </div>
                                    <div className="flex gap-2 justify-end">
                                        <button
                                            onClick={() => setShowRotationConfirm(false)}
                                            className="px-3 py-1.5 bg-zinc-800 text-zinc-300 rounded-lg text-[10px] font-bold hover:bg-zinc-700"
                                        >
                                            Cancel
                                        </button>
                                        <button
                                            onClick={handleRotateKey}
                                            disabled={rotating}
                                            className="px-3 py-1.5 bg-orange-500 text-black hover:bg-orange-400 rounded-lg text-[10px] font-bold"
                                        >
                                            {rotating ? 'Raotating...' : 'Confirm Rotation'}
                                        </button>
                                    </div>
                                </div>
                            </div>
                        )}
                    </div>
                </div>
            </div>

            {/* Security Architecture Grid - Redesigned */}
            <div className="space-y-4 pt-4 border-t border-white/5">
                <h3 className="text-sm font-bold text-zinc-400 uppercase tracking-widest flex items-center gap-2">
                    <Server size={14} /> Security Architecture
                </h3>

                <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
                    {/* Card 1: Client-Side Identity */}
                    <div className="group p-5 rounded-2xl bg-zinc-900/40 border border-purple-500/20 overflow-hidden relative cursor-default transition-all duration-500 hover:bg-zinc-900/60 hover:border-purple-500/40">
                        <div className="absolute inset-0 bg-purple-500/5 blur-xl group-hover:bg-purple-500/10 transition-colors" />
                        <div className="relative z-10">
                            <div className="flex items-start justify-between mb-3">
                                <div className="w-10 h-10 rounded-xl bg-purple-500/10 flex items-center justify-center text-purple-400 border border-purple-500/20 group-hover:scale-110 transition-transform duration-300 shadow-[0_0_15px_rgba(168,85,247,0.15)]">
                                    <Shield size={20} />
                                </div>
                                <span className="text-[10px] font-bold text-purple-400/70 uppercase tracking-widest bg-purple-500/10 px-2 py-1 rounded-lg border border-purple-500/10">Identity</span>
                            </div>
                            <h4 className="font-bold text-zinc-100 mb-1 group-hover:text-purple-300 transition-colors">Client-Side Auth</h4>
                            <p className="text-[11px] text-zinc-500 leading-relaxed group-hover:text-zinc-400 transition-colors">
                                Your identity is cryptographic. Authentication happens locally, preventing server-side impersonation.
                            </p>
                        </div>
                    </div>

                    {/* Card 2: Argon2id */}
                    <div className="group p-5 rounded-2xl bg-zinc-900/40 border border-emerald-500/20 overflow-hidden relative cursor-default transition-all duration-500 hover:bg-zinc-900/60 hover:border-emerald-500/40">
                        <div className="absolute inset-0 bg-emerald-500/5 blur-xl group-hover:bg-emerald-500/10 transition-colors" />
                        <div className="relative z-10">
                            <div className="flex items-start justify-between mb-3">
                                <div className="w-10 h-10 rounded-xl bg-emerald-500/10 flex items-center justify-center text-emerald-400 border border-emerald-500/20 group-hover:scale-110 transition-transform duration-300 shadow-[0_0_15px_rgba(16,185,129,0.15)]">
                                    <CheckCircle2 size={20} />
                                </div>
                                <span className="text-[10px] font-bold text-emerald-400/70 uppercase tracking-widest bg-emerald-500/10 px-2 py-1 rounded-lg border border-emerald-500/10">Hashing</span>
                            </div>
                            <h4 className="font-bold text-zinc-100 mb-1 group-hover:text-emerald-300 transition-colors">Argon2id KDF</h4>
                            <p className="text-[11px] text-zinc-500 leading-relaxed group-hover:text-zinc-400 transition-colors">
                                Industry-standard memory-hard hashing protects your Master Key from brute-force attacks.
                            </p>
                        </div>
                    </div>

                    {/* Card 3: Zero-Knowledge */}
                    <div className="group p-5 rounded-2xl bg-zinc-900/40 border border-blue-500/20 overflow-hidden relative cursor-default transition-all duration-500 hover:bg-zinc-900/60 hover:border-blue-500/40">
                        <div className="absolute inset-0 bg-blue-500/5 blur-xl group-hover:bg-blue-500/10 transition-colors" />
                        <div className="relative z-10">
                            <div className="flex items-start justify-between mb-3">
                                <div className="w-10 h-10 rounded-xl bg-blue-500/10 flex items-center justify-center text-blue-400 border border-blue-500/20 group-hover:scale-110 transition-transform duration-300 shadow-[0_0_15px_rgba(59,130,246,0.15)]">
                                    <Server size={20} />
                                </div>
                                <span className="text-[10px] font-bold text-blue-400/70 uppercase tracking-widest bg-blue-500/10 px-2 py-1 rounded-lg border border-blue-500/10">Privacy</span>
                            </div>
                            <h4 className="font-bold text-zinc-100 mb-1 group-hover:text-blue-300 transition-colors">Zero-Knowledge</h4>
                            <p className="text-[11px] text-zinc-500 leading-relaxed group-hover:text-zinc-400 transition-colors">
                                Even we can't see your data. Your Master Key never leaves this device unencrypted.
                            </p>
                        </div>
                    </div>
                </div>
            </div>

        </div>
    );

}
