import React, { createContext, useContext, useState, useEffect } from 'react';

export type FontFamily = 'Inter' | 'JetBrains Mono' | 'Georgia' | 'System' | 'Slab' | 'Playfair' | 'Outfit' | 'Montserrat';
export type AccentColor = 'purple' | 'emerald' | 'blue' | 'amber' | 'rose' | 'zinc' | 'crimson' | 'ocean';
export type CursorStyle = 'smooth' | 'solid' | 'blink' | 'none' | 'block' | 'underline';
export type ThemeVariant = 'dark' | 'light' | 'system' | 'midnight' | 'oled';
export type ConflictStrategy = 'manual' | 'cloud-wins' | 'local-wins';
export type StorageProvider = 'onyx-cloud' | 's3' | 'webdav' | 'gdrive' | 'local-only';
export type LayoutDensity = 'compact' | 'normal' | 'relaxed';
export type BackgroundEffect = 'none' | 'mesh' | 'particles' | 'aurora' | 'grain' | 'pulse';

export interface SettingsState {
    // Basic & Layout
    fontFamily: FontFamily; // Legacy/Global fallback
    uiFontFamily: FontFamily;
    editorFontFamily: FontFamily;
    sidebarFontFamily: FontFamily;
    codeFontFamily: FontFamily;

    fontSize: number; // Global editor size
    uiFontSize: number;
    sidebarFontSize: number;

    lineHeight: number;
    letterSpacing: number;
    wordSpacing: number;
    paragraphSpacing: number;
    maxContentWidth: number;
    contentPadding: number;
    layoutDensity: LayoutDensity;
    showMath: boolean;
    showDividers: boolean;
    isSettingsOpen: boolean;

    // Appearance (UI/UX)
    accentColor: AccentColor;
    customAccentHue: number;
    themeVariant: ThemeVariant;
    sidebarOpacity: number;
    windowBlur: boolean;
    customCursor: boolean;
    uiAnimationSpeed: 'fast' | 'normal' | 'relaxed' | 'disabled';
    glassIntensity: number;
    fontWeight: 'light' | 'normal' | 'medium' | 'bold';
    roundedness: 'none' | 'small' | 'medium' | 'large' | 'full';
    sidebarPosition: 'left' | 'right';
    showTitlebar: boolean;
    tabStyle: 'glass' | 'chrome' | 'minimal';
    acrylicOpacity: number;
    backgroundEffect: BackgroundEffect;
    cornerRadius: number;
    iconSet: 'classic' | 'neon' | 'minimal' | 'onyx-bold';
    language: string;
    cloudEnabled: boolean;
    appVersion: string;
    activeFeatures: {
        latex: boolean;
        math: boolean;
        ai: boolean;
        spellcheck: boolean;
        dailyNotes: boolean;
        graphView: boolean;
    };

    // Editor Behavior & Performance
    lineNumbers: boolean;
    relativeLineNumbers: boolean;
    spellcheck: boolean;
    wordWrap: boolean;
    scrollPastEnd: boolean;
    cursorStyle: CursorStyle;
    highlightActiveLine: boolean;
    showWhitespace: 'none' | 'selection' | 'all';
    autoClosingBrackets: boolean;
    autoClosingQuotes: boolean;
    folding: boolean;
    indentSize: number;
    tabType: 'spaces' | 'tabs';
    matchBrackets: boolean;
    lineHighlightOpacity: number;
    typewriterMode: boolean;
    focusMode: boolean;
    indentationGuides: boolean;
    stickyScroll: boolean;
    minimap: boolean;
    autoSaveInterval: number;
    renderLinkPreviews: boolean;
    lineWrappingIndent: number;
    smoothScrolling: boolean;
    cursorSmoothCaretAnimation: boolean;
    bracketPairColorization: boolean;
    renderControlCharacters: boolean;
    wordWrapColumn: number;
    codeLens: boolean;
    fontLigatures: boolean;
    cursorBlinking: 'blink' | 'smooth' | 'phase' | 'expand' | 'solid';

    // Universal Storage & Sync
    storageMode: 'cloud' | 'local';
    storageProvider: StorageProvider;
    s3Config: {
        endpoint: string;
        bucket: string;
        accessKey: string;
        secretKey: string;
        region: string;
        useSSL: boolean;
    };
    autoSync: boolean;
    syncInterval: number;
    backgroundSync: boolean;
    conflictStrategy: ConflictStrategy;
    syncOnLaunch: boolean;
    compressionEnabled: boolean;
    compressionLevel: 'none' | 'low' | 'high' | 'ultra';
    offlineMode: boolean;
    partialSync: boolean;
    syncTrash: boolean;
    retryInterval: number;
    wasmSync: boolean;
    cacheTTL: number;
    networkTimeout: number;
    binaryDeltaEncoding: boolean;
    maxConcurrentSyncs: number;
    attachmentSyncMode: 'all' | 'manual' | 'never';
    autoCleanupThreshold: number; // MB

    // Security & Privacy
    autoLockTimeout: number;
    encryptOnSave: boolean;
    biometricUnlock: boolean;
    telemetryEnabled: boolean;
    confirmDelete: boolean;
    privacyScreen: boolean;
    zeroKnowledgeMode: boolean;
    maskSensitiveData: boolean;
    secureClipboard: boolean;
    preventScreenshots: boolean;

    // Account & Presence
    accountBadgeType: 'none' | 'founder' | 'contributor' | 'pro' | 'onyx-core';
    showSyncHeatmap: boolean;
    customBannerUrl: string;
    userName: string;
    userBio: string;

    // Data, Export & Assets
    autoExportJson: boolean;
    backupFrequency: 'daily' | 'weekly' | 'none';
    attachmentPath: string;
    maxAttachmentSize: number;
    retainVersionHistory: boolean;
    versionHistoryLimit: number;

    // Experimental & AI
    aiProvider: 'onyx-internal' | 'openai' | 'anthropic' | 'local-ollama';
    aiApiKey: string;
    aiAutocomplete: boolean;
    aiTone: 'professional' | 'creative' | 'technical' | 'casual';
    vimMode: boolean;
    zenModeOnLaunch: boolean;
    gpuAcceleration: boolean;
    mirrorEnabled: boolean;
    mirrorPath: string;
    mirrorDeleteToBin: boolean;
}

interface SettingsContextType extends SettingsState {
    settings: SettingsState;
    updateSettings: (updates: Partial<SettingsState>) => void;
    toggleSettings: (open?: boolean) => void;
}

const SettingsContext = createContext<SettingsContextType | undefined>(undefined);

const STORAGE_KEY = 'onyx-settings-v3';

export const DEFAULT_SETTINGS: SettingsState = {
    fontFamily: 'Inter',
    uiFontFamily: 'Inter',
    editorFontFamily: 'Inter',
    sidebarFontFamily: 'Inter',
    codeFontFamily: 'JetBrains Mono',

    fontSize: 16,
    uiFontSize: 14,
    sidebarFontSize: 13,

    lineHeight: 1.6,
    letterSpacing: 0,
    wordSpacing: 0,
    paragraphSpacing: 1.5,
    maxContentWidth: 800,
    contentPadding: 40,
    layoutDensity: 'normal',
    showMath: true,
    showDividers: true,
    isSettingsOpen: false,

    accentColor: 'purple',
    customAccentHue: 270,
    themeVariant: 'dark',
    sidebarOpacity: 100,
    windowBlur: true,
    customCursor: true,
    uiAnimationSpeed: 'normal',
    glassIntensity: 40,
    fontWeight: 'normal',
    roundedness: 'medium',
    sidebarPosition: 'left',
    showTitlebar: true,
    tabStyle: 'glass',
    acrylicOpacity: 80,
    backgroundEffect: 'mesh',
    cornerRadius: 12,
    iconSet: 'neon',
    language: 'English',
    cloudEnabled: true,
    appVersion: '1.2.4-stable',
    activeFeatures: {
        latex: true,
        math: true,
        ai: true,
        spellcheck: true,
        dailyNotes: true,
        graphView: true
    },

    lineNumbers: true,
    relativeLineNumbers: false,
    spellcheck: true,
    wordWrap: true,
    scrollPastEnd: true,
    cursorStyle: 'smooth',
    highlightActiveLine: true,
    showWhitespace: 'none',
    autoClosingBrackets: true,
    autoClosingQuotes: true,
    folding: true,
    indentSize: 4,
    tabType: 'spaces',
    matchBrackets: true,
    lineHighlightOpacity: 5,
    typewriterMode: false,
    focusMode: false,
    indentationGuides: true,
    stickyScroll: false,
    minimap: false,
    autoSaveInterval: 2000,
    renderLinkPreviews: true,
    lineWrappingIndent: 2,
    smoothScrolling: true,
    cursorSmoothCaretAnimation: true,
    bracketPairColorization: true,
    renderControlCharacters: false,
    wordWrapColumn: 80,
    codeLens: true,
    fontLigatures: true,
    cursorBlinking: 'smooth',

    storageMode: 'cloud',
    storageProvider: 'onyx-cloud',
    s3Config: {
        endpoint: '',
        bucket: '',
        accessKey: '',
        secretKey: '',
        region: 'us-east-1',
        useSSL: true,
    },
    autoSync: true,
    syncInterval: 5000,
    backgroundSync: true,
    conflictStrategy: 'cloud-wins',
    syncOnLaunch: true,
    compressionEnabled: true,
    compressionLevel: 'high',
    offlineMode: false,
    partialSync: false,
    syncTrash: true,
    retryInterval: 5000,
    wasmSync: true,
    cacheTTL: 3600,
    networkTimeout: 10000,
    binaryDeltaEncoding: true,
    maxConcurrentSyncs: 5,
    attachmentSyncMode: 'all',
    autoCleanupThreshold: 500,

    autoLockTimeout: 30,
    encryptOnSave: false,
    biometricUnlock: false,
    telemetryEnabled: false,
    confirmDelete: true,
    privacyScreen: false,
    zeroKnowledgeMode: true,
    maskSensitiveData: false,
    secureClipboard: true,
    preventScreenshots: false,

    accountBadgeType: 'pro',
    showSyncHeatmap: true,
    customBannerUrl: '',
    userName: '',
    userBio: '',

    autoExportJson: false,
    backupFrequency: 'none',
    attachmentPath: './attachments',
    maxAttachmentSize: 50,
    retainVersionHistory: true,
    versionHistoryLimit: 50,

    aiProvider: 'onyx-internal',
    aiApiKey: '',
    aiAutocomplete: false,
    aiTone: 'creative',
    vimMode: false,
    zenModeOnLaunch: false,
    gpuAcceleration: true,
    mirrorEnabled: false,
    mirrorPath: '',
    mirrorDeleteToBin: true,
};

export function SettingsProvider({ children }: { children: React.ReactNode }) {
    const [settings, setSettings] = useState<SettingsState>(() => {
        const saved = localStorage.getItem(STORAGE_KEY);
        if (saved) {
            try {
                return { ...DEFAULT_SETTINGS, ...JSON.parse(saved), isSettingsOpen: false };
            } catch (e) {
                return DEFAULT_SETTINGS;
            }
        }
        return DEFAULT_SETTINGS;
    });

    useEffect(() => {
        const { isSettingsOpen, ...toSave } = settings;
        localStorage.setItem(STORAGE_KEY, JSON.stringify(toSave));
    }, [settings]);

    const updateSettings = (updates: Partial<SettingsState>) => {
        setSettings(s => ({ ...s, ...updates }));
    };

    const toggleSettings = (open?: boolean) => {
        setSettings(s => ({ ...s, isSettingsOpen: open !== undefined ? open : !s.isSettingsOpen }));
    };

    return (
        <SettingsContext.Provider value={{
            ...settings,
            settings, // Expose full settings object for easy access
            updateSettings,
            toggleSettings
        }}>
            {children}
        </SettingsContext.Provider>
    );
}

export function useSettings() {
    const context = useContext(SettingsContext);
    if (!context) throw new Error('useSettings must be used within a SettingsProvider');
    return context;
}
