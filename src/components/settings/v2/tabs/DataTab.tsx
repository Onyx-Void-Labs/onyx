import { Database, Cloud, HardDrive, Server, Shield, Lock, WifiOff, FolderOpen, Download, Upload, Loader2 } from 'lucide-react';
import { useSettings } from '../../../../contexts/SettingsContext';
import { useSync } from '../../../../contexts/SyncContext';
import { open } from '@tauri-apps/plugin-dialog';
import { openPath } from '@tauri-apps/plugin-opener';
import { appDataDir, join } from '@tauri-apps/api/path';
import { writeTextFile, readDir, readTextFile } from '@tauri-apps/plugin-fs';
import { useState, useEffect } from 'react';
import * as Y from 'yjs';
import { IndexeddbPersistence } from 'y-indexeddb';

export default function DataTab() {
    const { files, createFile } = useSync();
    const {
        storageProvider,
        updateSettings,
        mirrorEnabled,
        mirrorPath,
        mirrorDeleteToBin
    } = useSettings();

    const [dbPath, setDbPath] = useState<string>('Loading...');

    useEffect(() => {
        appDataDir().then(path => setDbPath(path));
    }, []);

    const [showLocalWarning, setShowLocalWarning] = useState(false);

    const handleProviderChange = (provider: 'onyx-cloud' | 'local-only' | 's3') => {
        if (provider === 'local-only') {
            // Check if logged in
            import('../../../../lib/pocketbase').then(({ pb }) => {
                if (pb.authStore.isValid) {
                    setShowLocalWarning(true);
                } else {
                    confirmLocalSwitch();
                }
            });
        } else if (provider === 'onyx-cloud') {
            updateSettings({
                storageProvider: 'onyx-cloud',
                offlineMode: false,
                cloudEnabled: true
            });
        } else {
            // S3 / Custom placeholder logic
            updateSettings({ storageProvider: provider });
        }
    };

    const confirmLocalSwitch = () => {
        // Sign out if logged in
        import('../../../../lib/pocketbase').then(({ pb }) => {
            pb.authStore.clear();
        });

        updateSettings({
            storageProvider: 'local-only',
            offlineMode: true,
            cloudEnabled: false
        });
        setShowLocalWarning(false);
    };

    const handleSelectMirrorPath = async () => {
        try {
            const selected = await open({
                directory: true,
                multiple: false,
                title: 'Select Mirror Folder'
            });

            if (selected && typeof selected === 'string') {
                updateSettings({ mirrorPath: selected });
            }
        } catch (error) {
            console.error('Failed to open dialog:', error);
        }
    };

    const [isExporting, setIsExporting] = useState(false);
    const [isImporting, setIsImporting] = useState(false);

    const handleExportAll = async () => {
        try {
            const selected = await open({
                directory: true,
                multiple: false,
                title: 'Select Export Folder'
            });

            if (!selected || typeof selected !== 'string') return;

            setIsExporting(true);
            let count = 0;

            for (const file of files) {
                // 1. Create temp doc to read from IndexedDB
                const doc = new Y.Doc();
                const persistence = new IndexeddbPersistence(`onyx-note-${file.id}`, doc);

                await new Promise<void>((resolve) => {
                    persistence.once('synced', () => {
                        resolve();
                    });
                });

                const content = doc.getText('codemirror').toString();
                const safeTitle = file.title.replace(/[^a-z0-9\u00a0-\uffff\-_\. ]/gi, '_').trim() || 'Untitled';

                // Handle duplicate names in export by overwriting for now (simplified)
                let fileName = `${safeTitle}.md`;
                let fullPath = await join(selected, fileName);

                await writeTextFile(fullPath, content);

                persistence.destroy();
                doc.destroy();
                count++;
            }

            console.log(`Exported ${count} notes.`);
            setIsExporting(false);
        } catch (error) {
            console.error('Export failed:', error);
            setIsExporting(false);
        }
    };

    const handleImport = async () => {
        try {
            const selected = await open({
                directory: true,
                multiple: false,
                title: 'Select Import Folder'
            });

            if (!selected || typeof selected !== 'string') return;

            setIsImporting(true);
            const entries = await readDir(selected);
            let count = 0;

            for (const entry of entries) {
                if (entry.isFile && entry.name.endsWith('.md')) {
                    const fullPath = await join(selected, entry.name);
                    const content = await readTextFile(fullPath);
                    const title = entry.name.replace(/\.md$/i, '');

                    // Create new note
                    const newId = createFile(title); // Metadata created

                    // Initialize content
                    const doc = new Y.Doc();
                    const persistence = new IndexeddbPersistence(`onyx-note-${newId}`, doc);

                    await new Promise<void>((resolve) => {
                        persistence.once('synced', () => resolve());
                    });

                    doc.transact(() => {
                        doc.getText('codemirror').insert(0, content);
                        doc.getMap('meta').set('title', title);
                    });

                    // Allow persistence to save
                    await new Promise(r => setTimeout(r, 50));

                    persistence.destroy();
                    doc.destroy();
                    count++;
                }
            }

            setIsImporting(false);
        } catch (error) {
            console.error('Import failed:', error);
            setIsImporting(false);
        }
    };

    return (
        <div className="space-y-8 animate-in fade-in slide-in-from-bottom-4 duration-500 pb-10 relative">
            {showLocalWarning && (
                <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/60 backdrop-blur-sm animate-in fade-in duration-200">
                    <div className="w-full max-w-md bg-zinc-900 border border-zinc-800 rounded-xl shadow-2xl p-6 space-y-6 animate-in zoom-in-95 duration-200">
                        <div className="flex items-center gap-3 text-red-500">
                            <div className="p-3 bg-red-500/10 rounded-full">
                                <Shield size={24} />
                            </div>
                            <h3 className="text-lg font-bold text-zinc-100">Switch to Local Only?</h3>
                        </div>

                        <div className="space-y-3">
                            <p className="text-zinc-400 text-sm leading-relaxed">
                                You are currently signed in. Switching to <span className="text-zinc-200 font-medium">Local Only</span> mode will:
                            </p>
                            <ul className="space-y-2 text-sm text-zinc-400">
                                <li className="flex items-start gap-2">
                                    <span className="text-red-500 mt-0.5">•</span>
                                    <span>Sign you out of your Onyx Cloud account</span>
                                </li>
                                <li className="flex items-start gap-2">
                                    <span className="text-red-500 mt-0.5">•</span>
                                    <span>Disable all cloud sync features</span>
                                </li>
                                <li className="flex items-start gap-2">
                                    <span className="text-red-500 mt-0.5">•</span>
                                    <span>Make Onyx operate 100% offline</span>
                                </li>
                            </ul>
                            <p className="text-red-400/80 text-xs italic bg-red-500/5 p-3 rounded-lg border border-red-500/10">
                                Note: Make sure you have your Master Key or Password saved safely. You will need them to sign back in.
                            </p>
                        </div>

                        <div className="flex items-center gap-3 pt-2">
                            <button
                                onClick={() => setShowLocalWarning(false)}
                                className="flex-1 py-2.5 px-4 rounded-lg bg-zinc-800 hover:bg-zinc-700 text-zinc-300 text-sm font-medium transition-colors"
                            >
                                Cancel
                            </button>
                            <button
                                onClick={confirmLocalSwitch}
                                className="flex-1 py-2.5 px-4 rounded-lg bg-red-600 hover:bg-red-500 text-white text-sm font-bold shadow-lg shadow-red-900/20 transition-all"
                            >
                                Confirm Switch
                            </button>
                        </div>
                    </div>
                </div>
            )}

            <div>
                <h2 className="text-2xl font-bold text-white mb-1">Storage & Privacy</h2>
                <p className="text-zinc-400 text-sm">Choose where your data lives and how it's protected.</p>
            </div>

            {/* Privacy Highlights (Restored Glow, No Icon Box, Always Active, Clean Icons) */}
            <div className="grid grid-cols-3 gap-4">
                <div className="relative p-5 rounded-2xl bg-zinc-900/40 border border-emerald-500/20 overflow-hidden group cursor-default transition-all duration-500 hover:bg-zinc-900/60 hover:border-emerald-500/30">
                    <div className="absolute inset-0 bg-emerald-500/10 blur-xl opacity-100" />
                    <div className="relative z-10">
                        <div className="flex items-center gap-3 mb-2">
                            <Lock size={18} className="text-emerald-400" />
                            <h4 className="text-sm font-bold text-emerald-100/90">AES-256</h4>
                        </div>
                        <p className="text-[11px] text-zinc-500 group-hover:text-zinc-400 leading-relaxed font-medium transition-colors">
                            Military-grade encryption for your notes at rest. Your keys, your data.
                        </p>
                    </div>
                </div>

                <div className="relative p-5 rounded-2xl bg-zinc-900/40 border border-blue-500/20 overflow-hidden group cursor-default transition-all duration-500 hover:bg-zinc-900/60 hover:border-blue-500/30">
                    <div className="absolute inset-0 bg-blue-500/10 blur-xl opacity-100" />
                    <div className="relative z-10">
                        <div className="flex items-center gap-3 mb-2">
                            <HardDrive size={18} className="text-blue-400" />
                            <h4 className="text-sm font-bold text-blue-100/90">Local-First</h4>
                        </div>
                        <p className="text-[11px] text-zinc-500 group-hover:text-zinc-400 leading-relaxed font-medium transition-colors">
                            Works 100% offline. We sync only when you're ready and online.
                        </p>
                    </div>
                </div>

                <div className="relative p-5 rounded-2xl bg-zinc-900/40 border border-purple-500/20 overflow-hidden group cursor-default transition-all duration-500 hover:bg-zinc-900/60 hover:border-purple-500/30">
                    <div className="absolute inset-0 bg-purple-500/10 blur-xl opacity-100" />
                    <div className="relative z-10">
                        <div className="flex items-center gap-3 mb-2">
                            <Shield size={18} className="text-purple-400" />
                            <h4 className="text-sm font-bold text-purple-100/90">Zero-Knowledge</h4>
                        </div>
                        <p className="text-[11px] text-zinc-500 group-hover:text-zinc-400 leading-relaxed font-medium transition-colors">
                            In Local Mode, no telemetry or data ever leaves your machine.
                        </p>
                    </div>
                </div>
            </div>

            {/* Storage Provider Hero */}
            <div className="space-y-4">
                <h3 className="text-sm font-bold text-zinc-400 uppercase tracking-widest flex items-center gap-2">
                    <Database size={14} /> Primary Storage Provider
                </h3>

                <div className="grid grid-cols-3 gap-4">
                    {/* Onyx Cloud */}
                    <button
                        onClick={() => handleProviderChange('onyx-cloud')}
                        className={`relative p-5 rounded-2xl border text-left flex flex-col gap-3 transition-all duration-300 group ${storageProvider === 'onyx-cloud'
                            ? 'bg-purple-500/10 border-purple-500/50 shadow-lg shadow-purple-500/20'
                            : 'bg-zinc-900/30 border-white/5 hover:border-purple-500/30 hover:bg-zinc-900/50'
                            }`}
                    >
                        <div className={`p-3 rounded-xl w-fit transition-colors ${storageProvider === 'onyx-cloud' ? 'bg-purple-500 text-white' : 'bg-zinc-800 text-zinc-400 group-hover:text-purple-400'
                            }`}>
                            <Cloud size={24} />
                        </div>
                        <div>
                            <div className={`font-bold text-base mb-1 ${storageProvider === 'onyx-cloud' ? 'text-white' : 'text-zinc-300'}`}>
                                Onyx Cloud
                            </div>
                            <p className="text-xs text-zinc-500 leading-relaxed">
                                Encrypted sync across devices with AI features and backups.
                            </p>
                        </div>
                        {storageProvider === 'onyx-cloud' && (
                            <div className="absolute top-4 right-4 text-purple-400 animate-pulse">
                                <div className="w-2 h-2 rounded-full bg-current" />
                            </div>
                        )}
                    </button>

                    {/* Local Device */}
                    <button
                        onClick={() => handleProviderChange('local-only')}
                        className={`relative p-5 rounded-2xl border text-left flex flex-col gap-3 transition-all duration-300 group ${storageProvider === 'local-only'
                            ? 'bg-emerald-500/10 border-emerald-500/50 shadow-lg shadow-emerald-500/20'
                            : 'bg-zinc-900/30 border-white/5 hover:border-emerald-500/30 hover:bg-zinc-900/50'
                            }`}
                    >
                        <div className={`p-3 rounded-xl w-fit transition-colors ${storageProvider === 'local-only' ? 'bg-emerald-500 text-white' : 'bg-zinc-800 text-zinc-400 group-hover:text-emerald-400'
                            }`}>
                            <HardDrive size={24} />
                        </div>
                        <div>
                            <div className={`font-bold text-base mb-1 ${storageProvider === 'local-only' ? 'text-white' : 'text-zinc-300'}`}>
                                Local Only
                            </div>
                            <p className="text-xs text-zinc-500 leading-relaxed">
                                Data never leaves this device. Sync and online features disabled.
                            </p>
                        </div>
                        {storageProvider === 'local-only' && (
                            <div className="absolute top-4 right-4 text-emerald-400">
                                <WifiOff size={16} />
                            </div>
                        )}
                    </button>

                    {/* Self-Hosted / Custom */}
                    <button
                        onClick={() => handleProviderChange('s3')}
                        className={`relative p-5 rounded-2xl border text-left flex flex-col gap-3 transition-all duration-300 group ${storageProvider === 's3'
                            ? 'bg-zinc-200/5 border-white/20 shadow-lg'
                            : 'bg-zinc-900/30 border-white/5 hover:border-white/10 hover:bg-zinc-900/50'
                            }`}
                    >
                        <div className={`p-3 rounded-xl w-fit transition-colors ${storageProvider === 's3' ? 'bg-white text-black' : 'bg-zinc-800 text-zinc-500 group-hover:text-zinc-300'
                            }`}>
                            <Server size={24} />
                        </div>
                        <div>
                            <div className={`font-bold text-base mb-1 ${storageProvider === 's3' ? 'text-white' : 'text-zinc-500'}`}>
                                Self-Hosted
                            </div>
                            <p className="text-xs text-zinc-500 leading-relaxed">
                                Connect to your own VPS or compatible storage (S3/MinIO).
                            </p>
                        </div>
                        {storageProvider === 's3' && (
                            <div className="absolute top-4 right-4 text-white">
                                <div className="w-2 h-2 rounded-full bg-current shadow-[0_0_10px_white]" />
                            </div>
                        )}
                    </button>
                </div>
            </div>

            {/* Self-Hosted Configuration (Conditional) */}
            {storageProvider === 's3' && (
                <div className="p-6 rounded-2xl border border-white/10 bg-zinc-900/40 space-y-4 animate-in fade-in slide-in-from-top-2">
                    <div className="flex items-center gap-3 mb-2">
                        <div className="p-2 bg-purple-500/10 rounded-lg text-purple-400">
                            <Server size={18} />
                        </div>
                        <div>
                            <h3 className="text-sm font-bold text-white">Server Configuration</h3>
                            <p className="text-xs text-zinc-500">Enter the URL of your self-hosted instance.</p>
                        </div>
                    </div>

                    <div className="grid grid-cols-2 gap-4">
                        <div className="space-y-1.5">
                            <label className="text-[10px] font-bold text-zinc-500 uppercase tracking-wider ml-1">Server URL</label>
                            <input
                                type="text"
                                placeholder="https://api.your-domain.com"
                                className="w-full bg-zinc-950/50 border border-white/5 rounded-xl px-4 py-2.5 text-sm text-white focus:border-purple-500/50 focus:outline-none transition-colors font-mono"
                            />
                        </div>
                        <div className="space-y-1.5">
                            <label className="text-[10px] font-bold text-zinc-500 uppercase tracking-wider ml-1">Live Endpoint (Optional)</label>
                            <input
                                type="text"
                                placeholder="wss://api.your-domain.com/realtime"
                                className="w-full bg-zinc-950/50 border border-white/5 rounded-xl px-4 py-2.5 text-sm text-white focus:border-purple-500/50 focus:outline-none transition-colors font-mono"
                            />
                        </div>
                    </div>

                    <div className="flex justify-end">
                        <a
                            href="https://docs.onyx.app/self-hosting" // Placeholder
                            target="_blank"
                            rel="noopener noreferrer"
                            className="text-[10px] text-purple-400 hover:text-purple-300 font-bold flex items-center gap-1 transition-colors"
                        >
                            Read the Self-Hosting Guide →
                        </a>
                    </div>
                </div>
            )}

            {/* Local Mirror (Available for Local Only and others) */}
            <div className={`p-6 rounded-2xl border transition-all duration-300 ${mirrorEnabled ? 'border-amber-500/20 bg-amber-500/5' : 'border-white/5 bg-zinc-900/40'}`}>
                <div className="flex items-center justify-between">
                    <div className="flex items-center gap-3">
                        <div className={`p-2 rounded-lg transition-colors ${mirrorEnabled ? 'bg-amber-500/20 text-amber-400' : 'bg-zinc-800 text-zinc-400'}`}>
                            <HardDrive size={18} />
                        </div>
                        <div>
                            <h3 className="text-sm font-bold text-white">Local File Mirror</h3>
                            <p className="text-xs text-zinc-500">
                                Save a copy of your notes as Markdown files on your disk.
                            </p>
                        </div>
                    </div>
                    <label className="relative inline-flex items-center cursor-pointer">
                        <input
                            type="checkbox"
                            className="sr-only peer"
                            checked={mirrorEnabled}
                            onChange={(e) => updateSettings({ mirrorEnabled: e.target.checked })}
                        />
                        <div className="w-11 h-6 bg-zinc-800 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-amber-500"></div>
                    </label>
                </div>

                {mirrorEnabled && (
                    <div className="mt-4 space-y-4 animate-in fade-in slide-in-from-top-1">
                        <div className="p-3 bg-amber-500/10 border border-amber-500/20 rounded-xl flex gap-3 items-start">
                            <div className="mt-0.5 min-w-[16px] text-amber-500">
                                <Shield size={16} />
                            </div>
                            <p className="text-xs text-amber-200/80 leading-relaxed">
                                <strong className="text-amber-200">Warning:</strong> Mirrored files are <strong>NOT encrypted</strong>. Anyone with access to your computer can read them.
                            </p>
                        </div>

                        <div className="space-y-1.5">
                            <label className="text-[10px] font-bold text-zinc-500 uppercase tracking-wider ml-1">Mirror Location</label>
                            <div className="flex gap-2">
                                <div className="flex-1 bg-zinc-950/50 border border-white/5 rounded-xl px-4 py-2.5 text-sm text-zinc-400 font-mono truncate cursor-text select-all" title={mirrorPath || '~/Documents/Onyx Notes'}>
                                    {mirrorPath || '~/Documents/Onyx Notes'}
                                </div>
                                <button
                                    onClick={handleSelectMirrorPath}
                                    className="px-4 bg-zinc-800 hover:bg-zinc-700 text-white rounded-xl text-xs font-bold transition-colors border border-white/5"
                                >
                                    Change
                                </button>
                            </div>
                        </div>

                        <div className="flex justify-end pt-2">
                            <a
                                href="https://docs.onyx.app/local-sync"
                                target="_blank"
                                rel="noopener noreferrer"
                                className="text-[10px] text-amber-500/60 hover:text-amber-400 font-bold flex items-center gap-1 transition-colors group"
                            >
                                How to sync with Syncthing, Git, or iCloud <span className="group-hover:translate-x-0.5 transition-transform">→</span>
                            </a>
                        </div>

                        {/* Delete to Recycle Bin Toggle */}
                        <div className="flex items-center justify-between pt-2 border-t border-amber-500/10">
                            <div>
                                <h4 className="text-xs font-semibold text-zinc-300">Delete to Recycle Bin</h4>
                                <p className="text-[10px] text-zinc-500">When deleting notes, move mirror file to Bin instead of permanent delete.</p>
                            </div>
                            <label className="relative inline-flex items-center cursor-pointer">
                                <input
                                    type="checkbox"
                                    className="sr-only peer"
                                    checked={mirrorDeleteToBin}
                                    onChange={(e) => updateSettings({ mirrorDeleteToBin: e.target.checked })}
                                />
                                <div className="w-11 h-6 bg-zinc-800 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-amber-500"></div>
                            </label>
                        </div>
                    </div>
                )}
            </div>

            {/* Internal Database Location (Read-Only) */}
            <div className="p-6 bg-zinc-900/30 border border-white/5 rounded-2xl space-y-3">
                <div className="flex items-center justify-between">
                    <div>
                        <h3 className="text-sm font-bold text-zinc-300">Internal Database</h3>
                        <p className="text-xs text-zinc-500 mt-1">
                            The location of your encrypted Onyx database.
                        </p>
                    </div>
                    <div className="p-2 bg-zinc-800/50 rounded-lg text-zinc-500">
                        <Database size={16} />
                    </div>
                </div>

                <div className="flex gap-2">
                    <div className="flex-1 bg-zinc-950/30 border border-white/5 rounded-xl px-4 py-2.5 text-xs text-zinc-500 font-mono truncate select-all">
                        {dbPath || 'Resolving paths...'}
                    </div>
                    <button
                        onClick={() => openPath(dbPath)}
                        className="px-3 bg-zinc-800 hover:bg-zinc-700 text-zinc-300 rounded-xl"
                    >
                        <FolderOpen size={14} />
                    </button>
                </div>
            </div>

            {/* Legacy Data Tools */}
            <div className="p-6 bg-zinc-900/30 border border-white/5 rounded-2xl flex items-center justify-between">
                <div>
                    <h3 className="text-sm font-bold text-zinc-300">Export / Import Data</h3>
                    <p className="text-xs text-zinc-500 mt-1">Create portable backups/archives of your notes.</p>
                </div>
                <div className="flex gap-2">
                    <button
                        onClick={handleImport}
                        disabled={isImporting}
                        className="flex items-center gap-2 px-4 py-2 bg-zinc-800 hover:bg-zinc-700 text-zinc-300 text-xs font-bold rounded-lg transition-colors border border-white/5 disabled:opacity-50"
                    >
                        {isImporting ? <Loader2 size={12} className="animate-spin" /> : <Upload size={12} />}
                        Import Folder
                    </button>
                    <button
                        onClick={handleExportAll}
                        disabled={isExporting}
                        className="flex items-center gap-2 px-4 py-2 bg-white text-black hover:bg-zinc-200 text-xs font-bold rounded-lg transition-colors shadow-lg disabled:opacity-50"
                    >
                        {isExporting ? <Loader2 size={12} className="animate-spin" /> : <Download size={12} />}
                        Export All
                    </button>
                </div>
            </div>
        </div >
    );
}
