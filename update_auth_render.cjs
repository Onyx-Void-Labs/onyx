const fs = require('fs');
const path = 'src/components/auth/AuthForms.tsx';
let content = fs.readFileSync(path, 'utf8');

// 1. Helper Function: Random Tag Generator
const randomTagFn = `
    const generateTag = () => {
        const tag = Math.floor(1000 + Math.random() * 9000).toString();
        setCustomTag(tag);
    };
`;
// Insert before main component or inside? Inside is better as it sets state.
// We added setCustomTag in previous step.

// 2. Add handleOtpVerify (for new flow)
const handleOtpVerifyLogic = `
    const handleOtpVerify = async (e: React.FormEvent) => {
        e.preventDefault();
        setLoading(true);
        try {
            const inputHash = await MasterKeyService.hashString(otpCode);
            if (inputHash === guestOtpHash) {
                // Success
                setMode('signup_username');
                generateTag(); // Pre-generate a tag
                setMessage({ type: 'success', text: "Verified! Create your identity." });
            } else {
                 setMessage({ type: 'error', text: "Incorrect code." });
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
        // Generate Recovery Phrase
        const mnemonic = bip39.generateMnemonic();
        setGeneratedPhrase(mnemonic);
        setMode('signup_recovery');
    };

    const handleRecoveryConfirm = (e: React.FormEvent) => {
        e.preventDefault();
        setMode('secure_account_choice');
    };
`;

// Insert these handlers before the return statement
content = content.replace('// --- NEW SIGNUP FLOW HANDLERS ---', handleOtpVerifyLogic);

// 3. Update Identity Render to show "Email or Username"
content = content.replace('placeholder="name@example.com"', 'placeholder="name@example.com or username"');
content = content.replace('type="email"', 'type="text"'); // Allow text for username

// 4. Add Render Blocks for new modes
// We need to find where the `magic_link_sent` block ends or `identity` block ends to insert new blocks.
// Let's replace the whole `return (` structure or just append cases.
// It's cleaner to use a replace on the render block if it's modular.
// The file is huge.
// Let's insert after `mode === 'identity'` block.

const newRenderBlocks = `
            {mode === 'signup_otp' && (
                <div className="animate-in fade-in slide-in-from-right-8 duration-500 space-y-6">
                    <BackBtn onClick={() => setMode('identity')} />
                    <AuthHeader title="Check your email" icon={Mail} subtitle={\`We sent a code to \${identifier}\`} />
                    
                    <form onSubmit={handleOtpVerify} className="space-y-6 pt-12">
                         <div className="text-center">
                            <AuthInput
                                type="text"
                                value={otpCode}
                                onChange={(e: any) => setOtpCode(e.target.value.replace(/[^0-9]/g, '').slice(0, 6))}
                                placeholder="000000"
                                autoFocus
                                className="text-center tracking-[1em] text-2xl font-mono"
                            />
                        </div>
                        <button
                            type="submit"
                            disabled={loading || otpCode.length < 6}
                            className="w-full bg-purple-600 hover:bg-purple-700 text-white py-4 rounded-xl font-bold shadow-lg transition-all hover:scale-[1.02] active:scale-[0.98] disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2"
                        >
                            {loading ? <Loader2 size={16} className="animate-spin" /> : "Verify Code"}
                        </button>
                    </form>
                </div>
            )}

            {mode === 'signup_username' && (
                <div className="animate-in fade-in slide-in-from-right-8 duration-500 space-y-6">
                    <BackBtn onClick={() => setMode('signup_otp')} />
                    <AuthHeader title="Create Identity" icon={Fingerprint} subtitle="How should we call you?" />

                    <form onSubmit={handleUsernameSubmit} className="space-y-6 pt-24">
                        <div className="flex items-center gap-2">
                            <div className="flex-1">
                                <AuthInput
                                    value={displayName}
                                    onChange={(e: any) => setDisplayName(e.target.value)}
                                    placeholder="Username"
                                    autoFocus
                                    required
                                />
                            </div>
                            <div className="w-24 relative">
                                <div className="absolute inset-y-0 left-3 flex items-center pointer-events-none text-zinc-500 font-bold">#</div>
                                <input 
                                    type="text" 
                                    value={customTag}
                                    onChange={(e) => setCustomTag(e.target.value.replace(/[^0-9]/g, '').slice(0, 4))}
                                    className="w-full bg-zinc-900/50 border border-white/5 rounded-full pl-8 pr-4 py-4 text-white focus:outline-none focus:border-purple-500/50 transition-all text-sm font-mono shadow-inner"
                                    placeholder="0000"
                                />
                            </div>
                             <button
                                type="button"
                                onClick={generateTag}
                                className="p-4 bg-zinc-800 hover:bg-zinc-700 rounded-full text-zinc-400 hover:text-white transition-colors"
                                title="Randomize Tag"
                            >
                                <RefreshCw size={20} />
                            </button>
                        </div>

                        <button
                            type="submit"
                            disabled={!displayName || !customTag}
                            className="w-full bg-purple-600 hover:bg-purple-700 text-white py-4 rounded-xl font-bold shadow-lg transition-all hover:scale-[1.02] active:scale-[0.98] disabled:opacity-50 flex items-center justify-center gap-2"
                        >
                            Continue <ArrowRight size={16} />
                        </button>
                    </form>
                </div>
            )}

            {mode === 'signup_recovery' && (
                <div className="animate-in fade-in slide-in-from-right-8 duration-500 space-y-6">
                    <BackBtn onClick={() => setMode('signup_username')} />
                    <AuthHeader title="Recovery Phrase" icon={ShieldCheck} subtitle="Save these 12 words securely" />

                    <div className="space-y-6 pt-20">
                         <div className="p-4 bg-red-500/10 border border-red-500/20 rounded-xl flex items-start gap-3">
                            <AlertTriangle className="text-red-400 shrink-0 mt-0.5" size={18} />
                            <p className="text-xs text-red-200 leading-relaxed">
                                This phrase is the <strong>ONLY</strong> way to recover your account if you lose your password. We cannot recover it for you.
                            </p>
                        </div>

                        <div className="grid grid-cols-3 gap-2 p-4 bg-black/20 rounded-2xl border border-white/5">
                            {generatedPhrase?.split(' ').map((word, i) => (
                                <div key={i} className="flex items-center gap-2 bg-zinc-900/50 px-3 py-2 rounded-lg border border-white/5">
                                    <span className="text-[10px] text-zinc-600 font-mono select-none">{i + 1}</span>
                                    <span className="text-sm font-medium text-zinc-300 font-mono select-all">{word}</span>
                                </div>
                            ))}
                        </div>

                        <div className="flex gap-3">
                             <button
                                type="button"
                                onClick={() => {
                                    if (generatedPhrase) navigator.clipboard.writeText(generatedPhrase);
                                    setMessage({ type: 'success', text: "Copied to clipboard" });
                                }}
                                className="flex-1 bg-zinc-800 hover:bg-zinc-700 text-zinc-300 py-3 rounded-xl font-medium text-sm transition-colors flex items-center justify-center gap-2"
                            >
                                <Copy size={16} /> Copy
                            </button>
                            {/* Download Button could go here */}
                        </div>

                         <form onSubmit={handleRecoveryConfirm}>
                            <button
                                type="submit"
                                className="w-full bg-purple-600 hover:bg-purple-700 text-white py-4 rounded-xl font-bold shadow-lg transition-all hover:scale-[1.02] active:scale-[0.98] flex items-center justify-center gap-2"
                            >
                                I have saved my phrase <ArrowRight size={16} />
                            </button>
                        </form>
                    </div>
                </div>
            )}
`;

// Insert after identity block
// We look for `{mode === 'identity' && (` ... closing `)}`
// It ends with `)}` at line 600-700 range.
// Let's use a unique string identifier.
// The identity block ends with a specific footer?
// Let's just append these blocks at the end of the return statement before `return (` closes?
// No, they are peer to identity block.
// Let's find `{mode === 'identity' && (` and append BEFORE it? No.
// Let's append AFTER it.
// We need to find the closing brace of identity block.
// Unique string in identity block: "Recover account"</button>\n                        </div>\n                    </div>\n                )}"
// A safe bet is to put it after the closing `)}` of identity.
// Or just put it inside the main div container if there is one?
// The component returns ` <div className="relative w-full max-w-md p-8 ...> ... {mode === 'identity' && ...} `
// So we can find `mode === 'identity'` and look for its matching `)}`.
// Easier: Search for `{mode === 'identity' && (` and replace it with ` {newUserModes} {mode === 'identity' && (` ? No order matters somewhat for Z-index? No.
// Let's just prepend to `mode === 'identity'`.
content = content.replace('{mode === \'identity\' && (', newRenderBlocks + '{mode === \'identity\' && (');


fs.writeFileSync(path, content);
console.log("Success");
