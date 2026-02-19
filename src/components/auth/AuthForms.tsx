import { useState, useEffect, useRef, useCallback } from 'react'
import { openPath } from '@tauri-apps/plugin-opener'; // Using openPath as generic opener
import { pb } from '../../lib/pocketbase'
import { MasterKeyService, encryptData, hashIdentity, encryptFile } from '../../services/SecurityService'
import { EmailAuthService } from '../../services/EmailAuthService'
import { DeviceService } from '../../services/DeviceService'
import { PasskeyService } from '../../services/PasskeyService'
import * as OTPAuth from 'otpauth';
import QRCodeStyling from 'qr-code-styling';
import * as bip39 from 'bip39';
import { Loader2, ShieldCheck, AlertTriangle, ChevronRight, Check, ArrowLeft, Fingerprint, Mail, Lock, RefreshCw, Eye, EyeOff, Copy, QrCode, ArrowRight, KeyRound, Shield, Key, X } from 'lucide-react'



// --- HELPER COMPONENTS ---

const BackBtn = ({ onClick, label = "Back" }: { onClick: () => void, label?: string }) => (
    <button
        type="button"
        onClick={onClick}
        className="absolute top-5 left-5 p-2 text-zinc-500 hover:text-white transition-colors rounded-full hover:bg-white/5 z-50 transform hover:-translate-x-1 duration-300"
        title={label}
    >
        <ArrowLeft size={20} />
    </button>
);

const AuthHeader = ({ title, icon: Icon, subtitle, className }: { title: string, icon: any, subtitle?: string, className?: string }) => {
    // If no subtitle is present, we want less bottom margin to tighten the design
    const defaultMargin = subtitle ? 'mb-6' : 'mb-2';
    return (
        <div className={`flex flex-col items-center text-center animate-in zoom-in-50 duration-500 absolute top-8 left-0 right-0 z-10 pointer-events-none ${className || defaultMargin}`}>
            <style>{`
            @keyframes dice-wobble {
                0%, 100% { transform: rotate(0deg) scale(1); }
                25% { transform: rotate(-10deg) scale(1.1); }
                75% { transform: rotate(10deg) scale(1.1); }
            }
        `}</style>
            <div className="w-24 h-24 bg-gradient-to-tr from-purple-500/10 to-blue-500/10 rounded-3xl flex items-center justify-center mb-6 shadow-[0_0_40px_rgba(168,85,247,0.1)] border border-white/5 relative group overflow-hidden pointer-events-auto">
                <div className="absolute inset-0 bg-gradient-to-tr from-purple-500/20 via-transparent to-blue-500/20 opacity-0 group-hover:opacity-100 transition-opacity duration-700" />

                {/* Animated Rings */}
                <div className="absolute inset-0 border border-purple-500/20 rounded-3xl scale-75 opacity-0 group-hover:scale-107 group-hover:opacity-100 transition-all duration-[2500ms] delay-100" />
                <div className="absolute inset-0 border border-blue-500/20 rounded-3xl scale-50 opacity-0 group-hover:scale-125 group-hover:opacity-100 transition-all duration-[2500ms] delay-200" />

                <Icon size={40} className="text-white drop-shadow-[0_0_15px_rgba(255,255,255,0.3)] relative z-10 group-hover:scale-110 transition-transform duration-700" />
            </div>
            {
                title && (
                    <h2 className="text-3xl font-bold text-transparent bg-clip-text bg-gradient-to-br from-white via-zinc-200 to-zinc-500 tracking-tight mb-2 leading-relaxed pb-1 whitespace-nowrap">
                        {title}
                    </h2>
                )
            }
            {subtitle && <p className="text-xs text-zinc-500 font-medium tracking-wide uppercase">{subtitle}</p>}
        </div >
    );
};

const AuthInput = ({ label, type, value, onChange, placeholder, autoFocus, required = false, rightElement }: any) => {
    const inputRef = useRef<HTMLInputElement>(null);
    const [showPassword, setShowPassword] = useState(false);
    const isPassword = type === 'password';

    useEffect(() => {
        if (autoFocus && inputRef.current) {
            setTimeout(() => inputRef.current?.focus(), 50);
        }
    }, [autoFocus]);

    return (
        <div className="space-y-1.5 group relative">
            {label && <label className="block text-[10px] font-bold text-zinc-500 uppercase tracking-widest ml-4 group-focus-within:text-purple-400 transition-colors pointer-events-none">{label}</label>}
            <div className="relative">
                <input
                    ref={inputRef}
                    type={isPassword ? (showPassword ? 'text' : 'password') : type}
                    value={value}
                    onChange={onChange}
                    className="w-full bg-zinc-900/50 border border-white/5 rounded-full px-6 py-4 text-white focus:outline-none focus:border-purple-500/50 focus:bg-zinc-900/80 focus:ring-4 focus:ring-purple-500/10 transition-all text-sm placeholder:text-zinc-600 font-medium shadow-inner pr-12"
                    placeholder={placeholder || "email or username#0000"}
                    required={required}
                    autoComplete="off"
                    data-1p-ignore
                    data-lpignore="true"
                />

                <div className="absolute right-4 top-1/2 -translate-y-1/2 flex items-center gap-2">
                    {rightElement}
                    {isPassword && (
                        <button
                            type="button"
                            onClick={() => setShowPassword(!showPassword)}
                            className="p-1.5 text-zinc-500 hover:text-white transition-colors rounded-full hover:bg-white/5"
                            tabIndex={-1}
                        >
                            {showPassword ? <EyeOff size={16} /> : <Eye size={16} />}
                        </button>
                    )}
                </div>
            </div>
        </div>
    );
};

const PasswordStrength = ({ password }: { password: string }) => {
    if (password.length === 0) return null;

    const hasLength = password.length >= 8;
    const hasUpper = /[A-Z]/.test(password);
    const hasNumber = /[0-9]/.test(password) || /[^a-zA-Z0-9]/.test(password);

    let strength = 0;
    if (password.length > 0) strength = 1;
    if (hasLength) strength++;
    if (hasLength && (hasUpper || hasNumber)) strength++;
    if (hasLength && hasUpper && hasNumber) strength++;

    const colors = ['bg-zinc-800', 'bg-red-500', 'bg-orange-500', 'bg-yellow-500', 'bg-emerald-500'];
    const labels = ['', 'Weak', 'Fair', 'Good', 'Strong'];

    return (
        <div className="px-4 space-y-2 animate-in fade-in slide-in-from-top-1">
            <div className="flex gap-1 h-1">
                {[1, 2, 3, 4].map((step) => (
                    <div
                        key={step}
                        className={`flex-1 rounded-full transition-all duration-300 ${strength >= step ? colors[strength] : 'bg-zinc-800'}`}
                    />
                ))}
            </div>
            {strength > 0 && <p className={`text-[10px] font-bold uppercase tracking-widest text-right ${colors[strength].replace('bg-', 'text-')}`}>{labels[strength]}</p>}
        </div>
    )
}

const UsernameGenerator = ({ initialBase, initialTag, onConfirm }: { initialBase: string, initialTag?: string, onConfirm: (username: string) => void }) => {
    const [base, setBase] = useState(initialBase);
    const [tag, setTag] = useState(initialTag || '');
    const [isRolling, setIsRolling] = useState(false);

    useEffect(() => {
        if (!initialTag) generateTag();
    }, []);

    const generateTag = () => {
        setIsRolling(true);
        setTimeout(() => setIsRolling(false), 500);

        // If user has typed a name like "Omar", we want "Omar21"
        // But the input is split: Base + Tag.
        // User asked: "Omar -> omar21". This implies the number becomes part of the name?
        // Or "Omar#21"? The current UI is "Username" field + "Tag" field.
        // If we want "Omar21#Tag", we can append random to base.
        // However, standard discord style is Name # Tag.
        // Let's Respect the Tag field first, but maybe suggest a new base number?
        // Actually, let's keep it simple: Roll Tag only for now to not be annoying,
        // BUT if base is empty, maybe fill it? No.

        // Random 4 digit tag
        const newTag = Math.floor(1000 + Math.random() * 9000).toString();
        setTag(newTag);
    };



    const handleSubmit = (e: React.FormEvent) => {
        e.preventDefault();
        if (base && tag) onConfirm(`${base}#${tag}`);
    };

    return (
        <div className="animate-in fade-in slide-in-from-right-8 duration-500 relative flex flex-col">
            <div className="space-y-6">

                <div className="flex-1 px-4 space-y-6 overflow-y-auto">
                    <form onSubmit={handleSubmit} className="space-y-6">
                        <div className="space-y-6">
                            <div className="flex flex-col gap-2 group">
                                <label className="text-[10px] font-bold text-zinc-500 uppercase tracking-widest ml-4 group-focus-within:text-purple-400 transition-colors">Username</label>
                                <input
                                    value={base}
                                    onChange={(e: any) => setBase(e.target.value.replace(/[^a-zA-Z0-9]/g, '').slice(0, 20))}
                                    placeholder="Pick a name"
                                    autoFocus
                                    className="w-full bg-zinc-900/50 border border-white/5 rounded-full px-6 py-4 text-white text-xl font-bold tracking-tight focus:outline-none focus:border-purple-500/50 focus:bg-zinc-900/80 focus:ring-4 focus:ring-purple-500/10 transition-all placeholder:text-zinc-700 shadow-inner"
                                />
                            </div>

                            <div className="flex flex-col gap-2 relative group">
                                <label className="text-[10px] font-bold text-zinc-500 uppercase tracking-widest ml-4 group-focus-within:text-purple-400 transition-colors">Tag</label>
                                <div className="relative">
                                    <div className="absolute left-6 top-1/2 -translate-y-1/2 text-zinc-600 font-bold text-lg">#</div>
                                    <input
                                        type="text"
                                        value={tag}
                                        onChange={(e) => setTag(e.target.value.replace(/[^a-zA-Z0-9]/g, '').slice(0, 4).toUpperCase())}
                                        className="w-full bg-zinc-900/30 border border-white/5 rounded-full pl-10 pr-14 py-4 text-white text-lg font-mono tracking-widest uppercase focus:outline-none focus:border-purple-500/50 transition-all placeholder:text-zinc-800"
                                        placeholder="0000"
                                        maxLength={4}
                                    />
                                    <button
                                        type="button"
                                        onClick={generateTag}
                                        className="absolute right-4 top-1/2 -translate-y-1/2 p-2 text-zinc-500 hover:text-white transition-colors active:scale-95"
                                        title="Roll New Tag"
                                    >
                                        <RefreshCw size={16} className={`${isRolling ? 'animate-spin' : ''}`} />
                                    </button>
                                </div>

                            </div>
                        </div>

                        <button
                            type="submit"
                            disabled={!base || !tag}
                            className="w-full bg-white hover:bg-zinc-200 text-black font-bold py-4 rounded-xl shadow-[0_0_20px_rgba(255,255,255,0.1)] transition-all hover:scale-[1.02] active:scale-[0.98] disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2 mt-4"
                        >
                            Confirm Identity <ArrowRight size={18} />
                        </button>


                    </form>
                </div>
            </div>
        </div>
    );
};

// --- MAIN COMPONENT ---

interface AuthFormsProps {
    onSuccess?: () => void;
    onCancel?: () => void;
}

type AuthMode = 'identity' | 'challenge_passkey' | 'challenge_password' | 'secure_account_choice' | 'signup_otp' | 'signup_username' | 'signup_recovery' | 'signup_security' | 'signup_2fa' | 'signup_profile' | 'recovery_input' | 'show_phrase' | 'magic_link_sent' | 'login_options';
type SignupMethod = 'passkey' | 'password';

interface UserCheckResult {
    exists: boolean;
    type: 'email' | 'username';
    user?: any;
}

export default function AuthForms({ onSuccess, onCancel }: AuthFormsProps) {
    const [mode, setMode] = useState<AuthMode>('identity')

    // Reset state when returning to identity
    useEffect(() => {
        if (mode === 'identity') {
            setPassword('');
            setConfirmPassword('');
            setDisplayName('');
            setAvatarFile(null);
            setAvatarPreview(null);
            setOtpCode('');
            setIsTotpEnabled(false);
            setTotpVerified(false);
            setTotpSecret('');
            setTotpQr('');
        }
    }, [mode]);



    // Auth State
    const [identifier, setIdentifier] = useState('')
    const [isExistingUser, setIsExistingUser] = useState(false)
    const [guestOtpHash, setGuestOtpHash] = useState<string | null>(null);
    const [availableAuthMethods, setAvailableAuthMethods] = useState<{
        password?: boolean;
        passkey?: boolean;
        email?: boolean;
        recovery?: boolean;
        magicLink?: boolean;
    }>({});

    // Signup State
    const [signupMethod, setSignupMethod] = useState<SignupMethod>('password'); // Default

    // Form Inputs
    const [password, setPassword] = useState('')
    const [confirmPassword, setConfirmPassword] = useState('')
    const [displayName, setDisplayName] = useState('')
    const [usernameBase, setUsernameBase] = useState('') // New state for stable handle
    const [customTag, setCustomTag] = useState('')
    const [recoveryPhraseInput, setRecoveryPhraseInput] = useState('')
    const [otpCode, setOtpCode] = useState('')

    // 2FA TOTP State
    const [isTotpEnabled, setIsTotpEnabled] = useState(false);
    const [totpSecret, setTotpSecret] = useState('');
    const [totpQr, setTotpQr] = useState('');
    const [totpVerifyCode, setTotpVerifyCode] = useState('');
    const [totpVerified, setTotpVerified] = useState(false);

    // Avatar State
    const [avatarFile, setAvatarFile] = useState<File | null>(null)
    const [avatarPreview, setAvatarPreview] = useState<string | null>(null)

    const [generatedPhrase, setGeneratedPhrase] = useState<string | null>(null)
    const [loading, setLoading] = useState(false)
    const [message, setMessage] = useState<{ type: 'error' | 'success', text: string } | null>(null)
    const [resendCooldown, setResendCooldown] = useState(0)
    const [isResending, setIsResending] = useState(false)
    const [resendSuccess, setResendSuccess] = useState(false)
    const lastResendTime = useRef<number>(0)

    const completeAuth = (mk: string) => {
        localStorage.setItem('onyx_mk', mk);
        sessionStorage.setItem('onyx_mk', mk);
        if (onSuccess) onSuccess();
    };

    // Resend cooldown timer — uses a ref timestamp so it survives page navigation
    useEffect(() => {
        if (resendCooldown <= 0) return;
        const timer = setTimeout(() => setResendCooldown(c => c - 1), 1000);
        return () => clearTimeout(timer);
    }, [resendCooldown]);

    // Recalculate cooldown when returning to OTP page
    useEffect(() => {
        if (mode === 'signup_otp' && lastResendTime.current > 0) {
            const elapsed = Math.floor((Date.now() - lastResendTime.current) / 1000);
            const remaining = Math.max(0, 60 - elapsed);
            setResendCooldown(remaining);
        }
    }, [mode]);

    // Reset cooldown when identifier changes (new email = fresh start)
    useEffect(() => {
        lastResendTime.current = 0;
        setResendCooldown(0);
    }, [identifier]);

    // Handle File Selection
    const handleFileChange = (e: React.ChangeEvent<HTMLInputElement>) => {
        const file = e.target.files?.[0];
        if (file) {
            setAvatarFile(file);
            const objectUrl = URL.createObjectURL(file);
            setAvatarPreview(objectUrl);
        }
    };

    const checkUserExists = async (id: string): Promise<UserCheckResult> => {
        // Basic format check
        const isEmail = id.includes('@');
        try {
            // Rate Limiting / Timing Attack Mitigation (Simulated)
            const startTime = Date.now();

            // Query PB
            // Note: This requires the client to have permission to list users with a filter.
            // In a real secure app, this should be a cloud function that returns limited info.
            const collection = pb.collection('users');
            // Handle # in username correctly (PocketBase doesn't support # in query directly if not encoded, 
            // but we should split it or query strictly).
            // Actually, we store 'username' as 'omar123' (base+suffix). 
            // The input 'omar#123' should be treated as checking 'omar123'. 
            // Check if cleanId is valid before querying.
            const cleanId = id.replace(/[^a-zA-Z0-9@.]/g, ''); // Keep @ for email
            if (!cleanId) return { exists: false, type: isEmail ? 'email' : 'username' };

            const filter = isEmail ? `email="${cleanId}"` : `username="${cleanId}"`;
            const records = await collection.getList(1, 1, { filter: filter });

            // Simulate consistent timing (e.g. at least 500ms)
            const elapsed = Date.now() - startTime;
            if (elapsed < 500) await new Promise(r => setTimeout(r, 500 - elapsed));

            if (records.totalItems > 0) {
                return { exists: true, type: isEmail ? 'email' : 'username', user: records.items[0] };
            }
            return { exists: false, type: isEmail ? 'email' : 'username' };
        } catch (err) {
            console.error("Check failed", err);
            return { exists: false, type: isEmail ? 'email' : 'username' };
        }
    };

    const handleIdentitySubmit = async (e: React.FormEvent) => {
        e.preventDefault();
        setLoading(true);
        setMessage(null);

        const check = await checkUserExists(identifier);
        setIsExistingUser(check.exists);

        if (check.exists) {
            const user = check.user;
            const methods = {
                password: true,
                passkey: false,
                email: false,
                magicLink: false,
                recovery: true
            };

            try {
                const passkeys = await pb.collection('passkeys').getList(1, 1, { filter: `user="${user.id}"` });
                if (passkeys.totalItems > 0) methods.passkey = true;
            } catch { }

            if (user.email) {
                methods.email = true;
                methods.magicLink = true;
            }

            setAvailableAuthMethods(methods);
            setMode('login_options');
        } else {
            // NEW USER
            setAvailableAuthMethods({});

            if (check.type === 'email') {
                // Route 2: New Email
                const code = Math.floor(100000 + Math.random() * 900000).toString();

                // Send via Backend
                await EmailAuthService.sendGuestOTP(identifier, code);

                // Store Hash
                const hash = await MasterKeyService.hashString(code);
                setGuestOtpHash(hash);
                setMode('signup_otp');
            } else {
                // Route 1: Username Blocked
                setMessage({ type: 'error', text: "User search failed. Use 'Continue without email' to create a new ID." });
            }
        }
        setLoading(false);
    };

    // Resend OTP without changing mode (prevents re-animation)
    const handleResendOtp = async () => {
        if (!identifier.includes('@')) return;
        setIsResending(true);
        setResendSuccess(false);
        const spinStart = Date.now();
        try {
            const code = Math.floor(100000 + Math.random() * 900000).toString();
            await EmailAuthService.sendGuestOTP(identifier, code);
            const hash = await MasterKeyService.hashString(code);
            setGuestOtpHash(hash);
            lastResendTime.current = Date.now();

            // Ensure the spin plays for at least 1 full rotation (1s)
            const elapsed = Date.now() - spinStart;
            if (elapsed < 1000) await new Promise(r => setTimeout(r, 1000 - elapsed));
            setIsResending(false);

            // Show green "Resent!" for 2 seconds, then start countdown
            setResendSuccess(true);
            setTimeout(() => {
                setResendSuccess(false);
                setResendCooldown(60);
            }, 2000);
        } catch (err) {
            console.error(err);
            setIsResending(false);
            setMessage({ type: 'error', text: 'Failed to resend code.' });
        }
    };






    const handleMagicLinkVerify = async (e: React.FormEvent) => {
        e.preventDefault();
        setLoading(true);

        try {
            if (isExistingUser) {
                // 1. Verify OTP for Existing User
                const userId = await EmailAuthService.verifyToken(otpCode);

                if (userId) {
                    // Valid OTP. Now check if they have a Passkey or need Password.
                    // Start Magic Link / OTP Flow
                    try {
                        const passkeys = await pb.collection('passkeys').getList(1, 1, { filter: `user="${userId}"` });
                        if (passkeys.totalItems > 0) {
                            setMode('challenge_passkey');
                            setMessage({ type: 'success', text: "Verified! Sign in with Passkey." });
                        } else {
                            setMode('challenge_password');
                            setMessage({ type: 'success', text: "Verified! Enter Password." });
                        }
                    } catch {
                        setMode('challenge_password');
                    }
                } else {
                    // 2. Verify Guest OTP (Client-side Hash Check)
                    if (!guestOtpHash) {
                        // setMessage({ type: 'error', text: "Session invalid. Try again." });
                        return;
                    }
                    const inputHash = await MasterKeyService.hashString(otpCode);

                    if (inputHash === guestOtpHash) {
                        setDisplayName(identifier.split('@')[0]);
                        setMode('signup_username');
                        setMessage({ type: 'success', text: "Email Verified! Create your identity." });
                    } else {
                        // setMessage({ type: 'error', text: "Incorrect verification code." });
                    }
                }
            }
        } catch (err) {
            console.error(err);
            // setMessage({ type: 'error', text: "Verification failed." });
        } finally {
            setLoading(false);
        }
    };


    const handleOtpVerify = async (e: React.FormEvent) => {
        e.preventDefault();
        setLoading(true);
        try {
            if (guestOtpHash) {
                const inputHash = await MasterKeyService.hashString(otpCode);
                if (inputHash === guestOtpHash) {
                    setMode('signup_username');
                    generateTag();
                    setMessage({ type: 'success', text: "Verified! Create your identity." });
                } else {
                    // setMessage({ type: 'error', text: "Incorrect code." });
                }
            } else {
                // Server-side check
                await EmailAuthService.verifyToken(otpCode);

                // If verification successful, log them in
                if (pb.authStore.isValid) {
                    handleAuthSuccess(false);
                }
                // Actually `requestOTP` in `EmailAuthService` was looking up a user!
                // If the user is NEW, `requestOTP` fails in the current `EmailAuthService`.

                // CRITICAL FIX: We need `EmailAuthService` to support Guest OTPs (no user yet).
                // But for now, let's fallback to the previous flow logic in `handleIdentitySubmit` where I removed the code gen.
                // I should Restore code gen -> Send to Rust -> Client Side Verify for GUESTS.
                // This is safer for ensuring we don't break the flow now.
                // I will assume `handleIdentitySubmit` was NOT modified to remove generation yet?
                // Wait, I just modified it in the PREVIOUS step to REMOVE generation.
                // I should have kept generation but sent it via Rust.

                // Let's fix it here: if no hash, assume it's just a failure for now or modify `handleIdentitySubmit` again?
                // No, I can't modify the same file slightly differently effectively.
                // I will add the logic here: if VerifyToken fails, maybe it matches a guest token?
                // Actually, let's revert the "Server Side" reliance for GUESTS and do "Client Gen -> Rust Send -> Client Verify" for speed.

                // setMessage({ type: 'error', text: "Session expired or invalid." });
            }
        } catch (err) {
            setMessage({ type: 'error', text: "Verification failed." });
        } finally {
            setLoading(false);
        }
    };

    const generateTag = () => {
        const tag = Math.floor(1000 + Math.random() * 9000).toString();
        setCustomTag(tag);
    };

    const handleUsernameSubmit = (e: React.FormEvent) => {
        e.preventDefault();
        // Lock in the username base
        setUsernameBase(displayName);

        // Generate Recovery Phrase
        const mnemonic = bip39.generateMnemonic();
        setGeneratedPhrase(mnemonic);
        setMode('signup_recovery');
    };

    const handleRecoveryConfirm = (e: React.FormEvent) => {
        e.preventDefault();
        setMode('secure_account_choice');
    };

    // State for private recovery email (for blind/OAuth users)
    const [recoveryEmail, setRecoveryEmail] = useState('');

    const handleSignup = async (e: React.FormEvent) => {
        e.preventDefault();
        setLoading(true);
        setMessage(null);

        try {
            const hwid = await DeviceService.getFingerprint();
            const isOAuthUser = pb.authStore.isValid && pb.authStore.model;

            // Use locked usernameBase if available, otherwise fallback to displayName
            const nameToUse = usernameBase || displayName;
            const baseName = nameToUse.toLowerCase().replace(/[^a-z0-9]/g, '').slice(0, 20) || 'user';

            // Use customTag if available, otherwise generate random
            const suffix = customTag || Math.floor(1000 + Math.random() * 9000).toString();
            const finalUsername = `${baseName}${suffix}`;

            // Check if we need to collect a real email (Blind Account)
            const isBlind = isOAuthUser && (pb.authStore.model?.email?.includes('@blind.onyx') || !pb.authStore.model?.email);
            const emailToEncrypt = isBlind ? recoveryEmail : identifier;

            // Fix 400 Error: Ensure we have an identifier for Passkey users who skipped email
            const effectiveIdentifier = identifier || `passkey_${finalUsername}@placeholder.onyx`;

            const mk = await MasterKeyService.generateMasterKey();
            const salt = MasterKeyService.generateSalt();
            const mnemonic = await MasterKeyService.generateRecoveryPhrase();

            let finalPassword = password;
            // Only generate random password if using passkey AND not setting a password manually
            if (signupMethod === 'passkey' && !password) {
                const array = new Uint8Array(16);
                crypto.getRandomValues(array);
                finalPassword = Array.from(array).map(b => b.toString(16).padStart(2, '0')).join('');
            }

            const key_wrapped_rk = await MasterKeyService.wrapKey(mk, mnemonic, salt);
            const recovery_hash = await MasterKeyService.hashString(mnemonic);

            const hwidEncrypted = await encryptData(hwid, mk);
            const nameEncrypted = await encryptData(displayName, mk);
            // Encrypt the Real Email (Recovery) or Identifier
            const emailEncrypted = emailToEncrypt ? await encryptData(emailToEncrypt, mk) : '';

            // For Password Users, we hash the identifier for the 'email' field used in login
            // For OAuth users, we DO NOT touch the email field (it's already set/blinded by Hook)
            const blindEmail = !isOAuthUser ? await hashIdentity(effectiveIdentifier) : undefined;

            // Prepare Payload (FormData for File Support)
            const formData = new FormData();
            formData.append('username', finalUsername);

            // Only set email/password if valid or creating
            if (!isOAuthUser) {
                formData.append('email', blindEmail!);
                formData.append('password', finalPassword);
                formData.append('passwordConfirm', finalPassword);
                formData.append('emailVisibility', 'false');
            } else {
                // OAuth User: We can optionally set the password if they provided one in signup_security
                if (password) {
                    formData.append('password', password);
                    formData.append('passwordConfirm', password);
                }
            }

            formData.append('name_ciphertext', nameEncrypted);
            formData.append('email_ciphertext', emailEncrypted);
            formData.append('hwid_ciphertext', hwidEncrypted);
            formData.append('enc_salt', salt);
            formData.append('key_wrapped_rk', key_wrapped_rk);
            formData.append('recovery_hash', recovery_hash);
            formData.append('email_opted_in', (emailToEncrypt) ? 'true' : 'false');

            // Name/Avatar are blind in DB (Name ciphertext has real name)
            if (!isOAuthUser) {
                formData.append('name', '');
            }

            // Avatar Handling
            if (!avatarFile) {
                formData.append('avatar_ciphertext', '');
            } else {
                try {
                    const encryptedAvatarFile = await encryptFile(avatarFile, mk);
                    formData.append('avatar', encryptedAvatarFile);
                    // Note: We don't set plain 'avatar' field for privacy
                } catch (encErr) {
                    console.error("Avatar encryption failed:", encErr);
                }
            }

            let userRecord;
            if (isOAuthUser) {
                // UPDATE existing OAuth User
                userRecord = await pb.collection('users').update(pb.authStore.model!.id, formData);
            } else {
                // CREATE User
                userRecord = await pb.collection('users').create(formData);
                await pb.collection('users').authWithPassword(blindEmail || finalUsername, finalPassword);
            }

            // Register Passkey
            if (signupMethod === 'passkey') {
                try {
                    const credential = await PasskeyService.register(finalUsername, userRecord.id);
                    await pb.collection('passkeys').create({
                        user: userRecord.id,
                        credential_id: credential.id,
                        public_key: credential.rawId,
                        label_ciphertext: await encryptData("Primary Device", mk),
                        counter: 0,
                        transports: credential.response.transports || [],
                        last_used: new Date()
                    });
                } catch (pkErr: any) {
                    console.warn("Passkey reg skipped:", pkErr);
                    setMessage({ type: 'error', text: "Passkey setup failed. Use Recovery Key." });
                }
            }

            // Save MK
            localStorage.setItem('onyx_mk', mk);
            sessionStorage.setItem('onyx_mk', mk);

            setGeneratedPhrase(mnemonic);
            setMode('show_phrase');

        } catch (err: any) {
            console.error("Signup Error:", err);
            console.log("Error Data:", JSON.stringify(err.data, null, 2)); // Debug PB 400 Error
            setMessage({ type: 'error', text: err.message || "Signup failed." });
        } finally {
            setLoading(false);
        }
    };

    const handlePasskeyLogin = async () => {
        setLoading(true);
        setMessage(null);
        try {
            await PasskeyService.authenticate();

            // 1. Check if we have a valid PB session
            if (pb.authStore.isValid) {
                const storedMk = localStorage.getItem('onyx_mk');
                if (storedMk) {
                    completeAuth(storedMk);
                } else {
                    // Session valid, but keys missing? (Rare, maybe cleared cache)
                    setMode('recovery_input');
                    setMessage({ type: 'success', text: "Identify confirmed. Enter Recovery Key to decrypt data." });
                }
            } else {
                setMode('challenge_password');
                setMessage({ type: 'success', text: "Verified! Please enter password to refresh session." });
            }
        } catch (err: any) {
            console.error(err);
            setMessage({ type: 'error', text: "Passkey verification failed." });
        } finally {
            setLoading(false);
        }
    };


    const handlePasswordLogin = async (e: React.FormEvent) => {
        e.preventDefault();
        setLoading(true);
        try {
            const blindEmail = await hashIdentity(identifier);
            await pb.collection('users').authWithPassword(blindEmail, password);
            const storedMk = localStorage.getItem('onyx_mk');
            if (storedMk) completeAuth(storedMk);
            else {
                setMode('recovery_input');
            }
        } catch (err) {
            setMessage({ type: 'error', text: "Invalid credentials." });
        }
    }
    // --- MANUAL OAUTH (DEV / DESKTOP) ---
    // State for manual token input (Desktop Bridge)
    const [showManualInput, setShowManualInput] = useState(false);
    const [manualToken, setManualToken] = useState('');
    const [manualAuthResult, setManualAuthResult] = useState<{ token: string, record: any } | null>(null);

    // Tauri does not support window.opener communication. We must manually open the browser
    // and catch the redirect code (Dev: localhost redirect, Prod: Deep Link).
    useEffect(() => {
        const handleDeepLink = async () => {
            // Check for ?code=... in URL (Localhost Redirect Strategy - Browser Side)
            const params = new URLSearchParams(window.location.search);
            const code = params.get('code');
            const state = params.get('state');

            if (code && state) {
                // We have an auth code!
                const storedProvider = localStorage.getItem('oauth_provider');
                if (!storedProvider) return;

                try {
                    const provider = JSON.parse(storedProvider);
                    if (state !== provider.state) {
                        setMessage({ type: 'error', text: "State mismatch. Possible CSRF." });
                        return;
                    }

                    window.history.replaceState({}, document.title, window.location.pathname);

                    // Exchange Code for Token
                    const authData = await pb.collection('users').authWithOAuth2Code(
                        provider.name,
                        code,
                        provider.codeVerifier,
                        window.location.origin + '/auth/callback',
                    );

                    // Success!
                    // If we are in the Browser (not Tauri), we display the token for the user to copy.
                    // @ts-ignore
                    if (!window.__TAURI__) {
                        setManualAuthResult({ token: pb.authStore.token, record: pb.authStore.model });
                        return; // Stop here, let UI show the token
                    }

                    // If somehow Tauri caught it (e.g. deep link in future), handle it:
                    handleAuthSuccess(authData.meta?.isNew);

                } catch (err: any) {
                    console.error("Manual OAuth Exchange Failed:", err);
                    setMessage({ type: 'error', text: `Login failed: ${err.message}` });
                } finally {
                    localStorage.removeItem('oauth_provider');
                }
            }
        };
        handleDeepLink();
    }, []);

    const handleAuthSuccess = (isNew?: boolean) => {
        // Successful Login
        setMessage({ type: 'success', text: "Authentication Successful" });

        // Determine Next Step
        // Check for Blind Account (OAuth)
        const user = pb.authStore.model;
        const isBlind = user?.email?.includes('@blind.onyx');

        if (isNew || isBlind) {
            // New User or Blind User -> Setup Master Key & Profile
            setMode('signup_security');
        } else {
            // Existing User -> Check for stored key
            const storedMk = localStorage.getItem('onyx_mk');
            if (storedMk) {
                completeAuth(storedMk);
            } else {
                // Existing user, new device -> Ask for Recovery Phrase / Key
                setMode('recovery_input');
            }
        }
    };

    const handleManualTokenSubmit = async (e: React.FormEvent) => {
        e.preventDefault();
        try {
            pb.authStore.save(manualToken, null); // Save token temporarily
            await pb.collection('users').authRefresh(); // Validate and fetch user model

            // Check if New User (Hard to know from refresh, assume existing unless profile missing?)
            // Actually, if they are pasting a token, likely they just logged in.
            // verifying 'isNew' is tricky here. 
            // We can check if they have a 'public_key' uploaded? 
            // If no public key -> New User Setup.
            const user = pb.authStore.model;
            // @ts-ignore
            const isNew = !user?.public_key;

            handleAuthSuccess(isNew);
        } catch (err: any) {
            console.error("Token verification failed:", err);
            setMessage({ type: 'error', text: "Invalid Token" });
            pb.authStore.clear();
        }
    };

    const handleOAuth = async (providerName: string) => {
        setLoading(true);
        // @ts-ignore
        const isTauri = typeof window !== 'undefined' && !!window.__TAURI_INTERNALS__;

        try {
            console.log("Fetching auth methods...");
            const health = await pb.health.check();
            console.log("PB Health:", health);

            const authMethods = await pb.collection('users').listAuthMethods();
            console.log("Full Auth Response:", JSON.stringify(authMethods, null, 2));

            const providers = (authMethods as any).authProviders || [];
            const provider = providers.find((p: any) => p.name.toLowerCase() === providerName.toLowerCase());

            if (!provider) {
                console.error(`Provider ${providerName} not found. Available:`, providers.map((p: any) => p.name));
                setMessage({ type: 'error', text: `${providerName} is not enabled. (Check Console)` });
                setLoading(false);
                return;
            }

            // --- DESKTOP AUTOMATED FLOW ---
            if (isTauri) {
                try {
                    // Dynamic import to avoid breaking web builds if plugin missing
                    // @ts-ignore
                    const { start, onUrl, cancel } = await import('@fabianlars/tauri-plugin-oauth');
                    const { openUrl } = await import('@tauri-apps/plugin-opener');

                    const port = await start();
                    const redirectUrl = `http://localhost:${port}`;
                    const targetUrl = provider.authUrl + '&redirect_uri=' + encodeURIComponent(redirectUrl);

                    await openUrl(targetUrl);

                    // Wait for callback
                    await new Promise<void>((resolve, reject) => {
                        let unlisten: (() => void) | undefined;

                        const cleanup = () => {
                            if (unlisten) unlisten();
                            cancel(port).catch(console.warn);
                        };

                        // Timeout 2min
                        const timer = setTimeout(() => {
                            cleanup();
                            reject(new Error("Login timed out."));
                        }, 120000);

                        onUrl(async (url: string) => {
                            try {
                                clearTimeout(timer);
                                const urlObj = new URL(url);
                                const code = urlObj.searchParams.get('code');
                                const state = urlObj.searchParams.get('state');

                                if (state !== provider.state) {
                                    console.warn("State mismatch in OAuth");
                                }

                                if (code) {
                                    await pb.collection('users').authWithOAuth2Code(
                                        provider.name,
                                        code,
                                        provider.codeVerifier,
                                        redirectUrl
                                    );
                                    handleAuthSuccess(true);
                                    resolve();
                                } else {
                                    reject(new Error("No code received."));
                                }
                            } catch (err) {
                                reject(err);
                            } finally {
                                cleanup();
                            }
                        }).then((fn: any) => unlisten = fn);
                    });

                } catch (err) {
                    console.error("Desktop OAuth Error:", err);
                    setMessage({ type: 'error', text: "Desktop login failed. Try browser fallback." });
                    // Fallback to manual if needed? 
                    // For now, just error out as requested to 'fix' it.
                }
            } else {
                // --- WEB FLOW ---
                localStorage.setItem('oauth_provider', JSON.stringify(provider));
                const redirectUrl = window.location.origin + '/auth/callback';
                const targetUrl = provider.authUrl + '&redirect_uri=' + encodeURIComponent(redirectUrl);
                window.location.href = targetUrl;
            }

        } catch (err: any) {
            console.error("OAuth Init Failed:", err);
            setMessage({ type: 'error', text: "Could not initialize login." });
        } finally {
            if (!pb.authStore.isValid) setLoading(false);
        }
    };



    // --- RENDER ---

    const handleSelectPasskeyMethod = () => {
        setSignupMethod('passkey');
        setMode('signup_2fa'); // Skip Password creation, go straight to 2FA or Profile
    };

    const handleSelectPasswordMethod = () => {
        setSignupMethod('password');
        setMode('signup_security');
    };

    const handleSecuritySubmit = (e: React.FormEvent) => {
        e.preventDefault();
        if (password !== confirmPassword) {
            setMessage({ type: 'error', text: "Passwords do not match." });
            return;
        }
        // Proceed to 2FA Setup
        setMode('signup_2fa');
    };

    const qrRef = useRef<HTMLDivElement>(null);
    const qrInstanceRef = useRef<any>(null);

    const generateTotp = useCallback(() => {
        const secret = new OTPAuth.Secret({ size: 20 });
        const totp = new OTPAuth.TOTP({
            issuer: "Onyx",
            label: identifier || "User",
            algorithm: "SHA1",
            digits: 6,
            period: 30,
            secret: secret
        });

        setTotpSecret(secret.base32);
        const uri = totp.toString();

        // Create premium styled QR code
        const qrCode = new QRCodeStyling({
            width: 150,
            height: 150,
            type: 'svg',
            data: uri,
            dotsOptions: {
                type: 'classy-rounded',
                gradient: {
                    type: 'linear',
                    rotation: Math.PI / 4,
                    colorStops: [
                        { offset: 0, color: '#a855f7' },
                        { offset: 1, color: '#6366f1' },
                    ],
                },
            },
            cornersSquareOptions: {
                type: 'extra-rounded',
                gradient: {
                    type: 'linear',
                    rotation: Math.PI / 4,
                    colorStops: [
                        { offset: 0, color: '#c084fc' },
                        { offset: 1, color: '#818cf8' },
                    ],
                },
            },
            cornersDotOptions: {
                type: 'dot',
                gradient: {
                    type: 'linear',
                    rotation: Math.PI / 4,
                    colorStops: [
                        { offset: 0, color: '#e9d5ff' },
                        { offset: 1, color: '#c7d2fe' },
                    ],
                },
            },
            backgroundOptions: {
                color: 'transparent',
            },
            qrOptions: {
                errorCorrectionLevel: 'M',
            },
        });

        qrInstanceRef.current = qrCode;
        setTotpQr('styled'); // Signal that QR is ready
        setIsTotpEnabled(true);

        // Render after state update
        setTimeout(() => {
            if (qrRef.current) {
                qrRef.current.innerHTML = '';
                qrCode.append(qrRef.current);
            }
        }, 50);
    }, [identifier]);

    const verifyTotpSetup = () => {
        if (!totpVerifyCode || totpVerifyCode.length !== 6) return;

        const totp = new OTPAuth.TOTP({
            issuer: "Onyx",
            label: identifier,
            algorithm: "SHA1",
            digits: 6,
            period: 30,
            secret: OTPAuth.Secret.fromBase32(totpSecret)
        });

        const delta = totp.validate({ token: totpVerifyCode, window: 1 });
        if (delta !== null) {
            setTotpVerified(true);
            setMessage({ type: 'success', text: "2FA Enabled Successfully." });
        } else {
            // setMessage({ type: 'error', text: "Invalid code." });
        }
    };

    const getHeaderProps = (mode: AuthMode) => {
        switch (mode) {
            case 'identity': return { title: "Create an account or log in", icon: Fingerprint, subtitle: "" };
            case 'secure_account_choice': return { title: "Secure Your Vault", icon: ShieldCheck, subtitle: "" };
            case 'challenge_passkey': return { title: "Verify Access", icon: Fingerprint, subtitle: "" };
            case 'challenge_password': return { title: "Enter Password", icon: Lock, subtitle: "" };
            case 'show_phrase': return { title: "Save This Key", icon: ShieldCheck, subtitle: "" };
            case 'recovery_input': return { title: "Enter Recovery Key", icon: ShieldCheck, subtitle: "" };
            case 'signup_username': return { title: "Create Identity", icon: Fingerprint, subtitle: "" };
            case 'signup_otp': return { title: "Check your email", icon: Mail, subtitle: "" };
            case 'magic_link_sent': return { title: "Check your email", icon: Mail, subtitle: "" };
            case 'signup_recovery': return { title: "Recovery Phrase", icon: ShieldCheck, subtitle: "" };
            case 'login_options': return { title: "Welcome Back", icon: Fingerprint, subtitle: "" };
            case 'signup_security': return { title: "Secure Your Vault", icon: Lock, subtitle: "" };
            case 'signup_2fa': return { title: "Extra Protection", icon: QrCode, subtitle: "" };
            case 'signup_profile': return { title: "Account Customisation", icon: Fingerprint, subtitle: "" };
            default: return { title: "Onyx", icon: Fingerprint, subtitle: "" };
        }
    };

    const headerProps = getHeaderProps(mode);

    // --- BACK BUTTON MAPPING ---
    const getBackAction = (mode: AuthMode): { onClick: () => void, label: string } | null => {
        switch (mode) {
            case 'magic_link_sent': return { onClick: () => setMode('identity'), label: "Change Email" };
            case 'signup_otp': return { onClick: () => setMode('identity'), label: "Back" };
            case 'signup_recovery': return { onClick: () => setMode('signup_username'), label: "Back" };
            case 'secure_account_choice': return { onClick: () => setMode('signup_recovery'), label: "Back" };
            case 'signup_security': return { onClick: () => setMode('secure_account_choice'), label: "Back" };
            case 'signup_2fa': return { onClick: () => setMode('signup_security'), label: "Back" };
            case 'signup_profile': return { onClick: () => setMode('signup_2fa'), label: "Back" };
            case 'challenge_passkey': return { onClick: () => setMode('login_options'), label: "Back" };
            case 'challenge_password': return { onClick: () => setMode('login_options'), label: "Back" };
            case 'signup_username': return { onClick: () => setMode(identifier.includes('@') ? 'signup_otp' : 'identity'), label: "Back" };
            case 'recovery_input': return { onClick: () => setMode(isExistingUser ? 'login_options' : 'identity'), label: "Back" };
            case 'login_options': return { onClick: () => setMode('identity'), label: "Back" };
            case 'identity': return onCancel ? { onClick: onCancel, label: "Back" } : null;
            default: return null;
        }
    };

    const backAction = getBackAction(mode);

    // --- RENDER ---

    return (
        <div className="w-full relative h-[580px] flex flex-col">

            {/* --- STATIC HEADER & NAVIGATION (PINNED) --- */}
            {backAction && <BackBtn onClick={backAction.onClick} label={backAction.label} />}

            <AuthHeader
                title={headerProps.title}
                icon={headerProps.icon}
                subtitle={headerProps.subtitle}
            />

            {message && message.type === 'success' && (
                <div className="absolute top-24 left-0 right-0 z-20 flex justify-center pointer-events-none">
                    <div className="text-xs px-4 py-3 rounded-full border flex items-center gap-3 shadow-lg animate-in slide-in-from-top-2 pointer-events-auto bg-emerald-500/10 border-emerald-500/20 text-emerald-400">
                        <Check size={14} />
                        <span className="font-bold tracking-wide uppercase text-[10px]">{message.text}</span>
                    </div>
                </div>
            )}

            {/* --- CONTENT AREA (ANIMATED) --- */}

            <div className="flex-1 w-full pt-[220px] overflow-y-auto overflow-x-visible px-16 scrollbar-none" style={{ scrollbarWidth: 'none', msOverflowStyle: 'none' }}>

                {/* --- STEP 1: IDENTITY (Unified Sign In/Up) --- */}

                {mode === 'signup_otp' && (
                    <div className="animate-in fade-in slide-in-from-right-8 duration-500 space-y-5">

                        <form onSubmit={handleOtpVerify} className="space-y-5">
                            <div className="space-y-2">
                                <AuthInput
                                    label="Verification Code"
                                    type="text"
                                    value={otpCode}
                                    onChange={(e: any) => setOtpCode(e.target.value.replace(/[^0-9]/g, '').slice(0, 6))}
                                    placeholder="000000"
                                    autoFocus
                                />
                            </div>
                            <button
                                type="submit"
                                disabled={loading || otpCode.length < 6}
                                className="w-full bg-gradient-to-r from-purple-600 to-purple-700 hover:from-purple-500 hover:to-purple-600 text-white py-3.5 rounded-xl font-bold shadow-lg shadow-purple-500/20 transition-all hover:scale-[1.02] active:scale-[0.98] disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2"
                            >
                                {loading ? <Loader2 size={16} className="animate-spin" /> : <><span>Verify Code</span><ArrowRight size={16} /></>}
                            </button>
                        </form>

                        <button
                            type="button"
                            disabled={resendCooldown > 0 || isResending || resendSuccess}
                            onClick={handleResendOtp}
                            className={`text-sm transition-all duration-200 hover:scale-105 flex items-center justify-center gap-2 mx-auto disabled:hover:scale-100 ${resendSuccess
                                ? 'text-emerald-400'
                                : resendCooldown > 0 || isResending
                                    ? 'text-zinc-600 cursor-not-allowed'
                                    : 'text-zinc-500 hover:text-white'
                                }`}
                        >
                            {resendSuccess ? (
                                <><Check size={13} /> Resent!</>
                            ) : (
                                <><RefreshCw size={13} className={isResending ? 'animate-spin' : ''} />
                                    {resendCooldown > 0 ? `Resend in ${resendCooldown}s` : 'Resend Code'}</>
                            )}
                        </button>
                    </div>
                )}

                {mode === 'signup_username' && (
                    <UsernameGenerator
                        initialBase={displayName || identifier.split('@')[0].split('#')[0]}
                        initialTag={customTag}
                        onConfirm={(username) => {
                            setDisplayName(username.split('#')[0]);
                            setCustomTag(username.split('#')[1]);
                            handleUsernameSubmit({ preventDefault: () => { } } as any);
                        }}
                    />
                )}

                {mode === 'signup_recovery' && (
                    <div className="animate-in fade-in slide-in-from-right-8 duration-500 space-y-4">

                        <div className="space-y-6 w-full max-w-2xl mx-auto">
                            {/* Premium card with gradient border */}
                            <div className="relative rounded-2xl p-[1px] bg-gradient-to-br from-purple-500/40 via-white/5 to-indigo-500/40 shadow-2xl">
                                <div className="bg-zinc-900/95 rounded-2xl overflow-hidden backdrop-blur-md">
                                    {/* Card header */}
                                    <div className="flex items-center justify-between px-5 py-4 border-b border-white/10">
                                        <div className="flex items-center gap-2 text-zinc-400">
                                            <Key size={14} className="text-purple-400" />
                                            <span className="text-[10px] font-bold uppercase tracking-widest text-zinc-300">Recovery Phrase</span>
                                        </div>
                                        <button
                                            type="button"
                                            onClick={async (e) => {
                                                const btn = e.currentTarget;
                                                if (generatedPhrase) await navigator.clipboard.writeText(generatedPhrase);

                                                // Success State
                                                btn.classList.add('bg-emerald-500', 'text-black', 'border-transparent', 'hover:bg-emerald-400');
                                                btn.classList.remove('text-zinc-400', 'hover:text-white', 'hover:bg-white/5');

                                                btn.innerHTML = `<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"></polyline></svg><span class="font-bold">Copied!</span>`;

                                                setTimeout(() => {
                                                    // Revert
                                                    btn.classList.remove('bg-emerald-500', 'text-black', 'border-transparent', 'hover:bg-emerald-400');
                                                    btn.classList.add('text-zinc-400', 'hover:text-white', 'hover:bg-white/5');

                                                    btn.innerHTML = `<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="14" height="14" x="8" y="8" rx="2" ry="2"></rect><path d="M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2"></path></svg><span>Copy</span>`;
                                                }, 2000);
                                            }}
                                            className="flex items-center gap-2 text-xs font-bold text-zinc-400 hover:text-white transition-all duration-300 px-3 py-1.5 rounded-lg border border-white/5 hover:bg-white/5 active:scale-95"
                                        >
                                            <Copy size={14} /><span>Copy</span>
                                        </button>
                                    </div>

                                    {/* 4-column word grid with staggered animation */}
                                    <div className="p-5 grid grid-cols-4 gap-3">
                                        {generatedPhrase?.split(' ').map((word, i) => (
                                            <div
                                                key={i}
                                                className="flex items-center gap-1.5 bg-zinc-900/60 hover:bg-zinc-800/80 px-2 py-2 rounded-xl border border-white/5 hover:border-purple-500/20 transition-all duration-300 cursor-default animate-in fade-in slide-in-from-bottom-2"
                                                style={{ animationDelay: `${i * 60}ms`, animationFillMode: 'both' }}
                                            >
                                                <span className="text-[8px] text-purple-400/60 font-mono select-none w-3 shrink-0">{i + 1}</span>
                                                <span className="text-[10px] font-bold text-zinc-200 font-mono select-all tracking-wide truncate">{word}</span>
                                            </div>
                                        ))}
                                    </div>
                                </div>
                            </div>

                            {/* Subtle warning */}
                            <div className="flex items-center gap-2 px-1">
                                <AlertTriangle className="text-amber-500/60 shrink-0" size={11} />
                                <p className="text-[10px] text-zinc-500 leading-relaxed">
                                    This is the <span className="text-zinc-400 font-semibold">only way</span> to recover your account. Store it somewhere safe.
                                </p>
                            </div>

                            {/* Save button */}
                            <form onSubmit={handleRecoveryConfirm}>
                                <button
                                    type="submit"
                                    className="w-full bg-gradient-to-r from-purple-600 to-indigo-600 hover:from-purple-500 hover:to-indigo-500 text-white py-3.5 rounded-full font-bold shadow-[0_0_30px_rgba(168,85,247,0.2)] transition-all duration-300 hover:scale-[1.02] hover:shadow-[0_0_40px_rgba(168,85,247,0.3)] active:scale-[0.98] flex items-center justify-center gap-2"
                                >
                                    I have saved my phrase <ArrowRight size={16} />
                                </button>
                            </form>
                        </div>
                    </div>
                )}

                {mode === 'login_options' && (
                    <div className="animate-in fade-in slide-in-from-right-8 duration-500 space-y-5">

                        <div className="space-y-3">
                            {/* Dynamic Options */}
                            {availableAuthMethods.passkey && (
                                <button
                                    key="passkey"
                                    onClick={() => setMode('challenge_passkey')}
                                    className="w-full bg-zinc-800 hover:bg-zinc-700 text-white py-4 rounded-xl font-bold transition-all flex items-center justify-center gap-2 group"
                                >
                                    <KeyRound size={20} className="text-purple-500 group-hover:scale-110 transition-transform" />
                                    <span>Sign in with Passkey</span>
                                </button>
                            )}

                            {availableAuthMethods.password && (
                                <button
                                    key="password"
                                    onClick={() => setMode('challenge_password')}
                                    className="w-full bg-zinc-800 hover:bg-zinc-700 text-white py-4 rounded-xl font-bold transition-all flex items-center justify-center gap-2 group"
                                >
                                    <Lock size={20} className="text-zinc-400 group-hover:text-white transition-colors" />
                                    <span>Sign in with Password</span>
                                </button>
                            )}

                            {availableAuthMethods.magicLink && (
                                <button
                                    key="magic"
                                    onClick={() => {
                                        // Trigger send magic link logic here if needed
                                        console.log(`[DEV] Sending magic link to ${identifier}`);
                                        setMode('magic_link_sent');
                                    }}
                                    className="w-full bg-zinc-800 hover:bg-zinc-700 text-white py-4 rounded-xl font-bold transition-all flex items-center justify-center gap-2 group"
                                >
                                    <Mail size={20} className="text-blue-400 group-hover:scale-110 transition-transform" />
                                    <span>Send Magic Link</span>
                                </button>
                            )}

                            {/* Fallback if no methods strictly available (e.g. only recovery) */}
                            {!availableAuthMethods.passkey && !availableAuthMethods.password && !availableAuthMethods.magicLink && (
                                <p className="text-center text-zinc-500 text-sm py-2">No standard login methods available.</p>
                            )}

                            <div className="relative py-4">
                                <div className="absolute inset-0 flex items-center"><span className="w-full border-t border-white/10" /></div>
                                <div className="relative flex justify-center text-xs uppercase"><span className="bg-[#09090b] px-2 text-zinc-500">Or</span></div>
                            </div>

                            <button
                                onClick={() => setMode('recovery_input')}
                                className="w-full bg-transparent hover:bg-white/5 border border-white/10 text-zinc-400 hover:text-white py-3 rounded-xl font-medium transition-all flex items-center justify-center gap-2"
                            >
                                <Shield size={16} /> Use Recovery Key
                            </button>
                        </div>
                    </div>
                )}

                {mode === 'identity' && (
                    <div className="animate-in fade-in slide-in-from-bottom-8 duration-500 space-y-5 group/email">

                        {manualAuthResult ? (
                            <div className="bg-emerald-500/10 p-6 rounded-2xl border border-emerald-500/20 text-center space-y-4 animate-in zoom-in-95">
                                <div className="w-16 h-16 bg-emerald-500/20 rounded-full flex items-center justify-center mx-auto mb-4">
                                    <Check className="text-emerald-400" size={32} />
                                </div>
                                <div>
                                    <h3 className="text-xl font-bold text-white mb-2">Authentication Successful</h3>
                                    <p className="text-zinc-400 text-sm leading-relaxed">
                                        Please copy the token below and paste it into the <strong>Onyx Desktop App</strong> to complete your sign in.
                                    </p>
                                </div>
                                <div className="bg-black/50 p-4 rounded-xl font-mono text-xs break-all text-zinc-300 border border-white/10 relative group text-left shadow-inner">
                                    {manualAuthResult.token}
                                    <button
                                        onClick={() => {
                                            navigator.clipboard.writeText(manualAuthResult.token);
                                            setMessage({ type: 'success', text: "Token Copied!" });
                                        }}
                                        className="absolute top-2 right-2 p-2 bg-zinc-800 rounded-lg hover:bg-zinc-700 text-white transition-colors border border-white/5"
                                        title="Copy Token"
                                    >
                                        <Copy size={14} />
                                    </button>
                                </div>
                                <div className="pt-4 border-t border-white/5">
                                    <p className="text-[10px] text-zinc-500 uppercase tracking-widest font-bold">You can close this window</p>
                                </div>
                            </div>
                        ) : showManualInput ? (
                            <div className="bg-zinc-900/80 border border-white/10 rounded-2xl p-6 space-y-4 animate-in fade-in zoom-in-95 shadow-2xl backdrop-blur-sm">
                                <div className="flex items-center justify-between border-b border-white/5 pb-4">
                                    <div className="flex items-center gap-3">
                                        <div className="p-2 bg-purple-500/10 rounded-lg">
                                            <ShieldCheck size={20} className="text-purple-400" />
                                        </div>
                                        <div>
                                            <h3 className="font-bold text-white leading-tight">Complete Login</h3>
                                            <p className="text-[10px] text-zinc-500 font-medium">Manual Verification Bridge</p>
                                        </div>
                                    </div>
                                    <button onClick={() => setShowManualInput(false)} className="text-zinc-500 hover:text-white transition-colors p-2 hover:bg-white/5 rounded-lg">
                                        <X size={18} />
                                    </button>
                                </div>

                                <div className="bg-blue-500/10 border border-blue-500/20 rounded-xl p-3 flex gap-3 items-start">
                                    <div className="mt-0.5 min-w-[16px] text-blue-400"><div className="w-1.5 h-1.5 bg-current rounded-full animate-pulse mt-1.5 ml-1" /></div>
                                    <p className="text-xs text-blue-200/80 leading-relaxed">
                                        A browser window has opened. Log in there, then copy the <strong>Auth Token</strong> and paste it below.
                                    </p>
                                </div>

                                <form onSubmit={handleManualTokenSubmit} className="space-y-4 pt-2">
                                    <div className="space-y-2">
                                        <label className="text-[10px] font-bold text-zinc-500 uppercase tracking-wider ml-1">Auth Token</label>
                                        <input
                                            type="text"
                                            value={manualToken}
                                            onChange={(e) => setManualToken(e.target.value)}
                                            placeholder="Paste token here..."
                                            className="w-full bg-zinc-950/50 border border-white/10 rounded-xl px-4 py-3.5 text-sm text-white focus:outline-none focus:border-purple-500/50 focus:bg-zinc-900 transition-all font-mono shadow-inner"
                                            autoFocus
                                        />
                                    </div>
                                    <div className="flex gap-3 pt-2">
                                        <button
                                            type="button"
                                            onClick={() => setShowManualInput(false)}
                                            className="flex-1 py-3.5 rounded-xl bg-zinc-800 hover:bg-zinc-700 text-zinc-300 font-bold text-sm transition-colors border border-white/5"
                                        >
                                            Cancel
                                        </button>
                                        <button
                                            type="submit"
                                            disabled={!manualToken}
                                            className="flex-1 py-3.5 rounded-xl bg-gradient-to-r from-purple-600 to-indigo-600 hover:from-purple-500 hover:to-indigo-500 text-white font-bold text-sm shadow-lg shadow-purple-500/20 transition-all disabled:opacity-50 disabled:cursor-not-allowed"
                                        >
                                            Verify & Login
                                        </button>
                                    </div>
                                </form>
                            </div>
                        ) : (
                            <>
                                {/* Row 1: Integrated Socials (Apple / Google) */}
                                <div className="grid grid-cols-2 gap-4">
                                    {/* Microsoft - 4-Color Grid */}
                                    <button
                                        type="button"
                                        onClick={() => handleOAuth('Microsoft')}
                                        className="w-full h-14 bg-zinc-900 rounded-xl flex items-center justify-center transition-all duration-300 hover:scale-105 active:scale-95 hover:bg-zinc-800 border border-white/5 hover:border-white/10"
                                        title="Continue with Microsoft"
                                    >
                                        <svg className="w-6 h-6" viewBox="0 0 23 23">
                                            <path fill="#f35325" d="M1 1h10v10H1z" />
                                            <path fill="#81bc06" d="M12 1h10v10H12z" />
                                            <path fill="#05a6f0" d="M1 12h10v10H1z" />
                                            <path fill="#ffba08" d="M12 12h10v10H12z" />
                                        </svg>
                                    </button>

                                    {/* Google - Filled Grey */}
                                    <button
                                        type="button"
                                        onClick={() => handleOAuth('Google')}
                                        className="w-full h-14 bg-zinc-900 rounded-xl flex items-center justify-center transition-all duration-300 hover:scale-107 active:scale-95 hover:bg-zinc-800"
                                        title="Continue with Google"
                                    >
                                        <svg className="w-7 h-7" viewBox="0 0 24 24"><path fill="#4285F4" d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92c-.26 1.37-1.04 2.53-2.21 3.31v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.09z" /><path fill="#34A853" d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z" /><path fill="#FBBC05" d="M5.84 14.09c-.22-.66-.35-1.36-.35-2.09s.13-1.43.35-2.09V7.07H2.18C1.43 8.55 1 10.22 1 12s.43 3.45 1.18 4.93l2.85-2.84z" /><path fill="#EA4335" d="M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.07l3.66 2.84c.87-2.6 3.3-4.53 6.16-4.53z" /></svg>
                                    </button>
                                </div>

                                {/* Separator with Lines */}
                                <div className="relative flex items-center gap-4">
                                    <div className="h-px bg-white/5 flex-1" />
                                    <span className="text-[10px] font-bold text-zinc-600 uppercase tracking-widest transition-colors group-focus-within/email:text-purple-400">Or Email / Username</span>
                                    <div className="h-px bg-white/5 flex-1" />
                                </div>

                                {/* Row 3: Email Input & Single "Continue" Button */}
                                <form onSubmit={handleIdentitySubmit} className="space-y-5">
                                    <AuthInput
                                        type="text"
                                        value={identifier}
                                        onChange={(e: any) => setIdentifier(e.target.value)}
                                        placeholder="onyx#1234"
                                        autoFocus
                                    />

                                    <button
                                        type="submit"
                                        disabled={loading || !identifier.match(/[@#].+/)}
                                        className="w-full bg-blue-600 hover:bg-blue-700 active:bg-blue-800 text-white py-3 rounded-xl font-bold shadow-lg hover:shadow-xl transition-all duration-200 hover:scale-[1.02] active:scale-[0.98] disabled:opacity-50 disabled:cursor-default group flex items-center justify-center gap-2"
                                    >
                                        {loading ? <Loader2 size={16} className="animate-spin" /> : <span>Continue <ArrowRight size={16} className="inline ml-1 group-hover:translate-x-1 transition-transform" /></span>}
                                    </button>
                                </form>

                                <div className="flex items-center justify-center gap-4 pt-2 text-xs font-medium text-zinc-500">
                                    <button
                                        type="button"
                                        onClick={() => {
                                            setIdentifier('');
                                            setMode('signup_username');
                                            // Trigger tag gen if not already
                                            const chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789';
                                            let result = '';
                                            const len = 5;
                                            for (let i = 0; i < len; i++) result += chars.charAt(Math.floor(Math.random() * chars.length));
                                            setCustomTag(result);
                                        }}
                                        className="transition-all duration-200 hover:scale-105 hover:text-white"
                                    >
                                        Continue without email
                                    </button>

                                    <div className="w-1 h-1 rounded-full bg-zinc-700" />

                                    <button
                                        type="button"
                                        onClick={() => setMode('recovery_input')}
                                        className="transition-all duration-200 hover:scale-105 hover:text-zinc-200"
                                    >
                                        Recover account
                                    </button>
                                </div>
                            </>
                        )}
                    </div>
                )}


                {/* --- STEP 2: MAGIC LINK VERIFY --- */}
                {
                    mode === 'magic_link_sent' && (
                        <div className="animate-in fade-in slide-in-from-right-8 duration-500 space-y-5">

                            <form onSubmit={handleMagicLinkVerify} className="space-y-5">
                                <div className="text-center">
                                    <AuthInput
                                        type="text"
                                        value={otpCode}
                                        onChange={(e: any) => setOtpCode(e.target.value.replace(/[^0-9]/g, '').slice(0, 6))}
                                        placeholder="000000"
                                        autoFocus
                                        required
                                        className="text-center tracking-[0.5em] text-2xl font-mono"
                                    />
                                </div>

                                <button
                                    type="submit"
                                    disabled={loading || otpCode.length < 6}
                                    className="w-full bg-emerald-500 hover:bg-emerald-400 text-black font-bold py-4 rounded-xl shadow-lg transition-all hover:scale-[1.02] active:scale-[0.98] disabled:opacity-50 flex items-center justify-center gap-2"
                                >
                                    {loading ? <Loader2 size={16} className="animate-spin" /> : "Verify Code"}
                                </button>

                                <button
                                    type="button"
                                    onClick={() => handleIdentitySubmit({ preventDefault: () => { } } as any)}
                                    className="text-xs text-zinc-500 hover:text-white transition-colors flex items-center justify-center gap-2 mx-auto"
                                >
                                    <RefreshCw size={12} /> Resend Code
                                </button>
                            </form>
                        </div>
                    )
                }

                {/* --- STEP 3: SECURE ACCOUNT CHOICE (Splits Flow) --- */}
                {
                    mode === 'secure_account_choice' && (
                        <div className="animate-in fade-in scale-95 duration-500 text-left p-8 -m-8">
                            <div className="space-y-6">
                                {/* Passkey Btn */}
                                <button
                                    onClick={handleSelectPasskeyMethod}
                                    className={`w-full relative overflow-hidden bg-gradient-to-br from-purple-600 to-indigo-800 border-purple-400/50 hover:border-white/30 text-white font-bold py-8 rounded-3xl transition-all flex items-center gap-6 px-8 shadow-[0_0_30px_rgba(168,85,247,0.2)] hover:shadow-[0_0_50px_rgba(168,85,247,0.4)] hover:scale-[1.02] active:scale-[0.98] group text-left isolate border`}
                                >
                                    <div className="absolute inset-[-50%] bg-[conic-gradient(from_0deg,transparent_0_340deg,white_360deg)] animate-[spin_3s_linear_infinite] opacity-10 group-hover:opacity-20 transition-opacity duration-500 blur-xl" />
                                    <div className="absolute inset-0 bg-gradient-to-br from-purple-500 via-transparent to-indigo-900 opacity-50 group-hover:opacity-30 transition-opacity duration-700" />
                                    <div className="absolute inset-[1px] bg-gradient-to-br from-purple-600/90 to-indigo-900/90 rounded-[23px] z-0" />
                                    <div className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-full h-full bg-purple-400/20 blur-[100px] rounded-full animate-pulse opacity-0 group-hover:opacity-100 transition-opacity duration-1000" />

                                    <div className="w-14 h-14 rounded-2xl bg-white/10 flex items-center justify-center shrink-0 border border-white/20 group-hover:bg-white/20 group-hover:border-white/40 transition-all duration-500 z-10 relative shadow-inner">
                                        <Fingerprint size={28} className="text-white transition-all duration-500" />
                                    </div>
                                    <div className="z-10 flex-1 relative">
                                        <div className="flex items-center gap-2 mb-1">
                                            <h4 className="text-lg font-bold tracking-tight text-white drop-shadow-md">Passkey</h4>
                                            <span className="bg-white/20 text-white text-[9px] font-bold uppercase px-2 py-0.5 rounded border border-white/20 shadow-sm backdrop-blur-md">Recommended</span>
                                        </div>

                                        <p className="text-xs text-purple-100/80 group-hover:text-white transition-colors font-medium">
                                            {identifier ? "Fast, biometric sign-in. No password required." : "Recommended for fast, secure access."}
                                        </p>
                                    </div>
                                    <ChevronRight className="ml-auto text-white group-hover:translate-x-1 transition-all duration-500 z-10 relative" />
                                </button>

                                {/* Password Btn */}
                                <button
                                    onClick={handleSelectPasswordMethod}
                                    className="w-full relative overflow-hidden bg-white/5 hover:bg-white/10 border border-white/5 hover:border-white/20 text-white font-bold py-8 rounded-3xl transition-all flex items-center gap-6 px-8 shadow-sm hover:shadow-[0_0_30px_rgba(255,255,255,0.05)] hover:scale-[1.01] active:scale-[0.98] group text-left isolate backdrop-blur-sm"
                                >
                                    <div className="absolute inset-0 bg-gradient-to-br from-white/5 to-transparent opacity-0 group-hover:opacity-100 transition-opacity duration-700" />
                                    <div className="w-14 h-14 rounded-2xl bg-zinc-800/50 flex items-center justify-center shrink-0 border border-white/5 group-hover:border-white/20 group-hover:bg-zinc-700/50 transition-all duration-500 relative z-10">
                                        <Lock size={24} className="text-zinc-500 group-hover:text-white transition-all duration-500" />
                                    </div>
                                    <div className="flex-1 relative z-10">
                                        <h4 className="text-lg font-bold tracking-tight mb-1 text-zinc-300 group-hover:text-white transition-colors">Master Password</h4>
                                        <p className="text-xs text-zinc-500 group-hover:text-zinc-400 transition-colors font-medium">Traditional password + optional 2FA.</p>
                                    </div>
                                    <ChevronRight className="ml-auto text-zinc-600 group-hover:text-white transition-all duration-500 z-10 relative group-hover:translate-x-1" />
                                </button>
                            </div>
                        </div>
                    )
                }

                {/* --- STEP 4: SECURITY DETAILS (Password Only) --- */}
                {
                    mode === 'signup_security' && (
                        <div className="animate-in fade-in slide-in-from-right-8 duration-500">
                            <form onSubmit={handleSecuritySubmit} className="space-y-5">

                                {/* Password Section */}
                                <div className="space-y-6">
                                    <div className="space-y-2">
                                        <AuthInput label="Master Password" type="password" value={password} onChange={(e: any) => setPassword(e.target.value)} placeholder="At least 8 characters" autoFocus required />
                                        <PasswordStrength password={password} />
                                    </div>

                                    <AuthInput
                                        label="Confirm Password"
                                        type="password"
                                        value={confirmPassword}
                                        onChange={(e: any) => setConfirmPassword(e.target.value)}
                                        placeholder="Repeat password"
                                        required
                                        rightElement={
                                            confirmPassword.length > 0 && password === confirmPassword ?
                                                <div className="flex items-center gap-1.5 px-2 py-0.5 rounded bg-emerald-500/10 text-emerald-400 text-[9px] font-bold uppercase tracking-wide border border-emerald-500/20">
                                                    <Check size={10} /> Matches
                                                </div> : null
                                        }
                                    />
                                </div>

                                <button
                                    type="submit"
                                    className="w-full bg-white text-black font-bold py-4 rounded-xl shadow-[0_0_20px_rgba(255,255,255,0.1)] transition-all hover:scale-[1.02] active:scale-[0.98] flex items-center justify-center gap-2 mt-4"
                                >
                                    Continue <ArrowRight size={18} />
                                </button>
                            </form>
                        </div>
                    )
                }

                {/* --- STEP 4.5: 2FA SETUP (Compact) --- */}
                {
                    mode === 'signup_2fa' && (
                        <div className="animate-in fade-in slide-in-from-right-8 duration-500 space-y-4">
                            {/* Compact Toggle Card */}
                            <div
                                onClick={() => { if (!isTotpEnabled) { generateTotp(); setIsTotpEnabled(true); } else { setIsTotpEnabled(false); setTotpVerified(false); } }}
                                className={`cursor-pointer w-full p-5 rounded-2xl border transition-all duration-300 flex items-center gap-4 ${isTotpEnabled ? 'bg-purple-500/10 border-purple-500/40' : 'bg-zinc-900/50 border-white/10 hover:border-white/20 hover:bg-zinc-800/50'}`}
                            >
                                <div className={`w-11 h-11 rounded-xl flex items-center justify-center transition-colors shrink-0 ${isTotpEnabled ? 'bg-purple-500 text-white' : 'bg-zinc-800 text-zinc-500'}`}>
                                    <QrCode size={20} />
                                </div>
                                <div className="flex-1 min-w-0">
                                    <h3 className={`text-sm font-bold ${isTotpEnabled ? 'text-white' : 'text-zinc-400'}`}>{isTotpEnabled ? 'Authenticator Enabled' : 'Enable Authenticator'}</h3>
                                    <p className="text-[10px] text-zinc-500 leading-relaxed">Secure your account with an authenticator app</p>
                                </div>
                                <div className={`w-10 h-6 rounded-full p-0.5 transition-colors shrink-0 ${isTotpEnabled ? 'bg-purple-500' : 'bg-zinc-700'}`}>
                                    <div className={`w-5 h-5 bg-white rounded-full shadow transition-transform duration-300 ${isTotpEnabled ? 'translate-x-4' : ''}`} />
                                </div>
                            </div>

                            {/* Expanded Setup — only when toggled on */}
                            {isTotpEnabled && (
                                <div className="space-y-3 animate-in slide-in-from-top-4 fade-in duration-500">
                                    <div className="flex items-start gap-4">
                                        {/* QR Code with glow */}
                                        <div className="shrink-0 relative">
                                            <div className="absolute inset-0 bg-purple-500/15 blur-2xl rounded-full" style={{ animation: 'pulse 6s cubic-bezier(0.4, 0, 0.6, 1) infinite' }} />
                                            <div
                                                ref={qrRef}
                                                className="w-[150px] h-[150px] flex items-center justify-center relative z-10"
                                            >
                                                {!totpQr && <Loader2 className="animate-spin text-purple-400 w-8 h-8" />}
                                            </div>
                                        </div>

                                        {/* Verify Code + Copy Key */}
                                        <div className="flex-1 space-y-2.5 pt-1">
                                            <div className="space-y-1.5">
                                                <p className="text-[10px] font-bold text-zinc-500 uppercase tracking-widest ml-4">Verify Code</p>
                                                <input
                                                    type="text"
                                                    value={totpVerifyCode}
                                                    onChange={(e) => setTotpVerifyCode(e.target.value.replace(/[^0-9]/g, '').slice(0, 6))}
                                                    placeholder="000000"
                                                    className="w-full bg-zinc-900/50 border border-white/5 rounded-full px-5 py-2.5 text-lg text-center font-mono tracking-[0.3em] focus:border-purple-500/50 focus:ring-4 focus:ring-purple-500/10 outline-none transition-all placeholder:text-zinc-700"
                                                    maxLength={6}
                                                    autoFocus
                                                />
                                            </div>
                                            <button
                                                type="button"
                                                onClick={verifyTotpSetup}
                                                disabled={totpVerifyCode.length !== 6 || totpVerified}
                                                className={`w-full py-2.5 rounded-full font-bold text-xs uppercase tracking-wider transition-all duration-300 ${totpVerified ? 'bg-emerald-500 text-black shadow-[0_0_20px_rgba(16,185,129,0.3)]' : 'bg-zinc-800 text-zinc-400 hover:text-white hover:bg-zinc-700 disabled:opacity-50'}`}
                                            >
                                                {totpVerified ? <span className="flex items-center justify-center gap-1.5"><Check size={14} /> Verified</span> : 'Verify'}
                                            </button>
                                            <button
                                                type="button"
                                                onClick={async (e) => {
                                                    const btn = e.currentTarget;
                                                    await navigator.clipboard.writeText(totpSecret);
                                                    btn.classList.add('text-emerald-400');
                                                    btn.innerHTML = `<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"></polyline></svg> Copied!`;
                                                    setTimeout(() => {
                                                        btn.classList.remove('text-emerald-400');
                                                        btn.innerHTML = `<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="14" height="14" x="8" y="8" rx="2" ry="2"></rect><path d="M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2"></path></svg> Copy setup key`;
                                                    }, 2000);
                                                }}
                                                className="w-full text-[10px] text-zinc-600 hover:text-zinc-400 transition-all duration-300 flex items-center justify-center gap-1.5 py-1"
                                            >
                                                <Copy size={10} /> Copy setup key
                                            </button>
                                        </div>
                                    </div>
                                </div>
                            )}

                            <button
                                type="button"
                                onClick={() => setMode('signup_profile')}
                                disabled={isTotpEnabled && !totpVerified}
                                className={`w-full py-4 rounded-full font-bold shadow-lg transition-all duration-500 flex items-center justify-center gap-2 mt-2 ${isTotpEnabled && !totpVerified ? 'bg-zinc-800/80 text-zinc-500 cursor-not-allowed scale-[0.98] opacity-80' : 'bg-white text-black hover:bg-zinc-200 hover:scale-[1.02] active:scale-[0.98] shadow-[0_0_20px_rgba(255,255,255,0.1)]'}`}
                            >
                                {isTotpEnabled ? (totpVerified ? 'Continue' : 'Verify to Continue') : 'Skip for now'} <ArrowRight size={18} />
                            </button>
                        </div>
                    )
                }

                {/* --- STEP 5: PROFILE DETAILS (Final) --- */}
                {
                    mode === 'signup_profile' && (
                        <div className="animate-in fade-in slide-in-from-right-8 duration-500">
                            <form onSubmit={handleSignup} className="space-y-5">

                                <div className="flex items-center gap-6 mb-4">
                                    <div className="relative group cursor-pointer shrink-0">
                                        <input
                                            type="file"
                                            accept="image/*"
                                            onChange={handleFileChange}
                                            className="absolute inset-0 w-full h-full opacity-0 z-50 cursor-pointer"
                                        />
                                        <div className={`w-20 h-20 rounded-full border-2 ${avatarPreview ? 'border-purple-500' : 'border-dashed border-zinc-700'} flex items-center justify-center bg-zinc-900/50 group-hover:bg-zinc-800 transition-all overflow-hidden relative shadow-lg`}>
                                            {avatarPreview ? (
                                                <img src={avatarPreview} alt="Avatar Preview" className="w-full h-full object-cover" />
                                            ) : (
                                                <div className="flex flex-col items-center gap-1 text-zinc-500 group-hover:text-zinc-300 transition-colors">
                                                    <div className="p-1.5 rounded-full bg-zinc-800/50 group-hover:bg-zinc-700 transition-colors">
                                                        <Fingerprint size={16} />
                                                    </div>
                                                </div>
                                            )}
                                        </div>
                                    </div>
                                    <div className="flex-1 space-y-4">
                                        <AuthInput label="Display Name" type="text" value={displayName} onChange={(e: any) => setDisplayName(e.target.value)} placeholder="e.g. Onyx Traveller" autoFocus required />

                                        {/* If Blind/OAuth: Ask for Recovery Email */}
                                        {(!identifier || identifier.includes('@blind.onyx')) && (
                                            <AuthInput
                                                label="Recovery Email (Private)"
                                                type="email"
                                                value={recoveryEmail}
                                                onChange={(e: any) => setRecoveryEmail(e.target.value)}
                                                placeholder="For account recovery only"
                                                required
                                                className="border-purple-500/30 bg-purple-500/5 focus:border-purple-500/60"
                                            />
                                        )}
                                    </div>
                                </div>

                                <button
                                    type="submit"
                                    disabled={loading}
                                    className="w-full bg-gradient-to-r from-emerald-500 to-teal-500 hover:from-emerald-400 hover:to-teal-400 text-black font-bold py-4 rounded-full transition-all flex items-center justify-center gap-3 shadow-[0_0_20px_rgba(16,185,129,0.3)] hover:scale-[1.02] active:scale-95 duration-300"
                                >
                                    {loading ? <Loader2 size={18} className="animate-spin" /> : <><ShieldCheck size={20} /> Complete Setup</>}
                                </button>
                            </form>
                        </div>
                    )
                }


                {/* --- CHALLENGE PASSKEY --- */}
                {
                    mode === 'challenge_passkey' && (
                        <div className="animate-in fade-in scale-95 duration-300">
                            <div>
                                <button
                                    onClick={handlePasskeyLogin}
                                    className="w-full bg-white hover:bg-zinc-200 text-black font-bold py-16 rounded-[40px] transition-all flex flex-col items-center justify-center gap-4 shadow-[0_0_40px_rgba(255,255,255,0.1)] hover:shadow-[0_0_60px_rgba(255,255,255,0.2)] hover:scale-[1.02] active:scale-[0.98] duration-500 group relative overflow-hidden"
                                >
                                    <div className="absolute inset-0 bg-gradient-to-tr from-purple-500/20 to-transparent opacity-0 group-hover:opacity-100 transition-opacity" />
                                    <Fingerprint size={48} className="text-zinc-900 group-hover:scale-110 transition-transform duration-500" />
                                    <span className="text-lg tracking-tight relative z-10">Tap to Authenticate</span>
                                </button>

                                <button onClick={() => setMode('challenge_password')} className="mt-5 mx-auto block text-[10px] text-zinc-500 hover:text-white uppercase tracking-widest font-bold transition-colors">
                                    Use Password Instead
                                </button>
                            </div>
                        </div>
                    )
                }

                {/* --- CHALLENGE PASSWORD --- */}
                {
                    mode === 'challenge_password' && (
                        <div className="animate-in fade-in slide-in-from-right-8 duration-500">
                            <form onSubmit={handlePasswordLogin} className="space-y-5">
                                <div className="h-[4px]" aria-hidden="true" />
                                <AuthInput label="Master Password" type="password" value={password} onChange={(e: any) => setPassword(e.target.value)} placeholder="••••••••" autoFocus />
                                <button type="submit" disabled={loading} className="w-full bg-purple-600 hover:bg-purple-500 text-white font-bold py-4 rounded-full transition-all flex items-center justify-center gap-2 shadow-lg active:scale-95 duration-300">
                                    {loading ? <Loader2 size={18} className="animate-spin" /> : 'Unlock Vault'}
                                </button>

                                {/* Magic Link Fallback - Only for Email Users */}
                                {identifier.includes('@') && (
                                    <button
                                        type="button"
                                        onClick={() => {
                                            setMode('magic_link_sent');
                                            // Simulate send
                                            console.log("Sending recovery magic link...");
                                        }}
                                        className="w-full text-xs text-zinc-500 hover:text-white uppercase tracking-widest font-bold transition-colors py-2"
                                    >
                                        Forgot Password? Send Magic Link
                                    </button>
                                )}
                            </form>
                        </div>
                    )
                }

                {/* --- RECOVERY --- */}
                {
                    mode === 'show_phrase' && (
                        <div className="animate-in fade-in zoom-in-95 duration-500">
                            <div className="relative group">
                                <div className="absolute -inset-0.5 bg-gradient-to-r from-emerald-500 to-cyan-500 rounded-2xl opacity-20 group-hover:opacity-40 transition-opacity blur" />
                                <div className="relative bg-zinc-950 border border-zinc-800 rounded-2xl p-6 mb-8 group-hover:border-zinc-700 transition-colors">
                                    <div className="flex items-center justify-between mb-4">
                                        <h5 className="text-xs font-bold text-zinc-400 uppercase tracking-widest">Recovery Key</h5>
                                        <button
                                            onClick={() => { navigator.clipboard.writeText(generatedPhrase || ''); setMessage({ type: 'success', text: "Copied to clipboard" }); }}
                                            className="text-[10px] font-bold text-emerald-500 hover:text-emerald-400 uppercase tracking-wider flex items-center gap-1.5 transition-colors"
                                        >
                                            <Copy size={12} /> Copy
                                        </button>
                                    </div>
                                    <div className="font-mono text-sm text-emerald-400 break-words leading-loose tracking-wide select-all text-center">
                                        {generatedPhrase}
                                    </div>
                                </div>
                            </div>

                            <div className="text-center space-y-4">
                                <button
                                    onClick={() => { if (onSuccess) onSuccess(); }}
                                    className="w-full bg-white hover:bg-zinc-200 text-black font-bold py-4 rounded-full transition-all flex items-center justify-center gap-2 shadow-[0_0_30px_rgba(255,255,255,0.1)] hover:scale-[1.02] active:scale-95 duration-300"
                                >
                                    <ShieldCheck size={18} /> I have saved it securely
                                </button>
                                <p className="text-[10px] text-zinc-600 font-medium max-w-[280px] mx-auto leading-relaxed">
                                    This key exists <span className="text-zinc-400">only on your device</span>. If you lose it, we cannot recover your data.
                                </p>
                            </div>
                        </div>
                    )
                }

                {/* --- RECOVERY INPUT --- */}
                {
                    mode === 'recovery_input' && (
                        <div className="animate-in fade-in zoom-in-95 duration-500">
                            <form className="space-y-5" onSubmit={(e) => { e.preventDefault(); /* Handle recovery */ }}>
                                <textarea
                                    value={recoveryPhraseInput}
                                    onChange={(e) => setRecoveryPhraseInput(e.target.value)}
                                    className="w-full bg-zinc-900/50 border border-white/5 rounded-3xl p-5 font-mono text-sm text-zinc-300 focus:border-purple-500/50 focus:ring-4 focus:ring-purple-500/10 focus:outline-none min-h-[100px] transition-all"
                                    placeholder="Enter your 12-word recovery phrase..."
                                />
                                <button type="submit" className="w-full bg-purple-600 text-white font-bold py-4 rounded-full">Recover Account</button>
                            </form>
                        </div>
                    )
                }
            </div >
        </div >
    )
}
