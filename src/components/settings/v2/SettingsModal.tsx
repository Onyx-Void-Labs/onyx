import { useState } from 'react';
import { useSettings } from '../../../contexts/SettingsContext';
import { X } from 'lucide-react';
import SettingsSidebar from './SettingsSidebar.tsx';
import AccountTab from './tabs/AccountTab.tsx';
import AppearanceTab from './tabs/AppearanceTab.tsx';
import EditorTab from './tabs/EditorTab.tsx';
import KeybindsTab from './tabs/KeybindsTab.tsx';
import AboutTab from './tabs/AboutTab.tsx';
import FeaturesTab from './tabs/FeaturesTab.tsx';
import DataTab from './tabs/DataTab.tsx';
import SecurityTab from './tabs/SecurityTab.tsx';

interface SettingsModalProps {
    user: any;
    onLogout: () => void;
}

export default function SettingsModal({ user, onLogout }: SettingsModalProps) {
    const { isSettingsOpen, toggleSettings } = useSettings();
    const [activeTab, setActiveTab] = useState('account');
    const [showAuth, setShowAuth] = useState(false);

    if (!isSettingsOpen) return null;

    const handleTabChange = (tabId: string) => {
        if (activeTab === tabId && tabId === 'account') {
            // "Back to Local" feature
            setShowAuth(false);
        } else {
            setActiveTab(tabId);
        }
    };

    return (
        <div className="fixed inset-0 z-[100] flex items-center justify-center p-4">
            {/* Backdrop */}
            <div
                className="absolute inset-0 bg-black/60 backdrop-blur-md animate-in fade-in duration-300"
                onClick={() => toggleSettings(false)}
            />

            {/* Modal Container */}
            <div className="relative w-full max-w-5xl max-h-[90vh] h-[700px] bg-[#09090b] border border-white/5 rounded-3xl shadow-2xl flex overflow-hidden animate-in zoom-in-95 duration-200">

                {/* Close Button - Moved to avoid overlap */}
                <button
                    onClick={() => toggleSettings(false)}
                    className="absolute top-6 right-6 z-[70] p-2 text-zinc-500 hover:text-white hover:bg-white/10 rounded-full transition-all"
                >
                    <X size={20} />
                </button>

                {/* Sidebar - Pass handleTabChange instead of setActiveTab */}
                <SettingsSidebar activeTab={activeTab} setActiveTab={handleTabChange} user={user} />

                {/* Content Area */}
                <div className="flex-1 flex flex-col overflow-hidden bg-black/20">
                    <div className="flex-1 overflow-y-auto custom-scrollbar p-8">
                        {activeTab === 'account' && (
                            <AccountTab
                                user={user}
                                onLogout={onLogout}
                                showAuth={showAuth}
                                setShowAuth={setShowAuth}
                            />
                        )}
                        {activeTab === 'security' && <SecurityTab />}
                        {activeTab === 'appearance' && <AppearanceTab />}
                        {activeTab === 'editor' && <EditorTab />}
                        {activeTab === 'shortcuts' && <KeybindsTab />}
                        {activeTab === 'features' && <FeaturesTab />}
                        {activeTab === 'storage' && <DataTab />}
                        {activeTab === 'about' && <AboutTab />}
                    </div>
                </div>
            </div>
        </div>
    );
}
