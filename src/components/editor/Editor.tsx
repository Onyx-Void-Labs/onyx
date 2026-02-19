/**
 * EditorV2
 * Core CodeMirror 6 editor component for Onyx.
 * Handles markdown editing, theming, encryption, and custom highlighter extensions.
 */

import React, { useState, useEffect, useRef } from 'react';
import {
    EditorView,
    keymap,
    drawSelection,
    placeholder,
    ViewPlugin,
    Decoration,
    type DecorationSet,
    type ViewUpdate,
    highlightActiveLine as cmHighlightActiveLine,
    lineNumbers as cmLineNumbers
} from '@codemirror/view';
import { EditorState } from '@codemirror/state';
import { markdown, markdownLanguage, markdownKeymap } from '@codemirror/lang-markdown';
import { languages } from '@codemirror/language-data';
import { defaultKeymap, history, historyKeymap, indentWithTab } from '@codemirror/commands';
import { HighlightStyle, syntaxHighlighting, foldGutter, codeFolding, foldKeymap } from '@codemirror/language';
import { tags } from '@lezer/highlight';
import { hideMarkdown } from './extensions/hideMarkdown';
import { autoPairs } from './extensions/autoPairs';

import { mathLivePreview } from './extensions/mathLivePreview';
import { mathTooltip } from './extensions/mathTooltip';
import { mathMenuPlugin, mathMenuKeymap, mathMenuStateField, MathMenuState } from './extensions/mathAutocomplete';
import { mathAutoReplace } from './extensions/mathAutoReplace';

import { MathMenu } from './MathMenu';
import { FindWidget } from './FindWidget';

import { search } from '@codemirror/search';

// Yjs Imports
import * as Y from 'yjs';
import { HocuspocusProvider } from '@hocuspocus/provider';
import { IndexeddbPersistence } from 'y-indexeddb';
import { yCollab } from 'y-codemirror.next';
import { useSync } from '../../contexts/SyncContext';
import { useSettings } from '../../contexts/SettingsContext';
// import { writeTextFile, createDir, exists, BaseDirectory } from '@tauri-apps/plugin-fs'; // OLD
import { writeTextFile, readTextFile, mkdir, exists, rename, stat } from '@tauri-apps/plugin-fs';
import { documentDir, join } from '@tauri-apps/api/path';

import 'katex/dist/katex.min.css'; // KaTeX CSS


// ============================================
const onyxTheme = EditorView.theme({
    '&': {
        backgroundColor: 'transparent',
        color: '#f4f4f5', // zinc-100 (White-ish)
        height: '100%',
    },
    '&.cm-focused': {
        outline: 'none',
    },
    '.cm-scroller': {
        fontFamily: 'inherit',
        fontSize: '1.125rem',
        lineHeight: '1.75',
        padding: '0 2rem',
        overflow: 'auto',
    },
    '.cm-content': {
        caretColor: '#a855f7',
        padding: '0',
    },
    '.cm-cursor': {
        borderLeftColor: '#a855f7',
        borderLeftWidth: '2px',
    },

    '.cm-activeLine': {
        backgroundColor: 'transparent',
    },
    '.cm-gutters': {
        backgroundColor: 'transparent',
        border: 'none',
        color: '#52525b', // zinc-600
        minHeight: '100%',
    },
    '.cm-foldPlaceholder': {
        backgroundColor: '#27272a',
        border: 'none',
        color: '#a1a1aa',
    },
    // Fold Gutter Arrows
    '.cm-foldGutter span': {
        opacity: '0',
        transition: 'opacity 0.2s',
        cursor: 'pointer',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        height: '100%',
        transform: 'translateY(3px)',
    },
    '.cm-foldGutter span.cm-fold-closed': {
        opacity: '1',
        color: '#d4d4d8',
    },
    '.cm-gutters:hover .cm-foldGutter span': {
        opacity: '0.7',
    },
    '.cm-gutters:hover .cm-foldGutter span:hover': {
        opacity: '1',
    },
    // Invisible Fold Placeholder
    '.cm-folded-badge': {
        display: 'inline-block',
        width: '0',
        height: '0',
        overflow: 'hidden',
    },
    // Math Live Preview
    '.cm-math-live': {
        display: 'inline-block',
        position: 'relative',
        cursor: 'text',
    },
    // Tooltip
    '.cm-tooltip': {
        backgroundColor: '#18181b', // zinc-900
        border: '1px solid #27272a', // zinc-800
        borderRadius: '0.5rem',
        padding: '0.5rem',
        color: '#e4e4e7',
        boxShadow: '0 10px 15px -3px rgba(0, 0, 0, 0.5)',
    },
    // ========================================================================
    // SEARCH HIGHLIGHTS
    // ========================================================================

    // 1. Inactive Match (Purple Highlight)
    // - Background: Semi-transparent purple
    // - Layout Push: Margins force text to move, giving matches physical presence
    // - Padding: Adds breathing room around the text
    '.cm-searchMatch': {
        backgroundColor: 'rgba(195, 105, 255, 0.5)',
        textDecoration: 'none',
        position: 'relative',
        margin: '0 3px',
        padding: '0 4px',
        display: 'inline-block',
        lineHeight: '1.2',
        borderRadius: '0.5em',
    },

    // 2. Active Match (Pink Highlight)
    // - Inherits geometry (margin/padding) from base class
    // - Color: Distinct Pink to indicate current selection
    '.cm-searchMatch.cm-searchMatch-selected': {
        backgroundColor: 'rgba(255, 105, 180, 0.7)',
    },
    '.cm-searchMatch.cm-searchMatch-selected::after': {
        display: 'none',
    },

    // 3. Native Selection (Transparent)
    // - Set to transparent to avoid conflicts with custom highlights
    '.cm-selectionBackground, &.cm-focused .cm-selectionBackground': {
        backgroundColor: 'transparent',
        borderRadius: '0.5em',
    },

    // Hide default CodeMirror Search Panel
    '.cm-panel, .cm-search': {
        display: 'none !important',
    },
    // Blockquote Line Styling
    '.cm-blockquote-line': {
        backgroundColor: 'rgba(168, 85, 247, 0.05)',
        borderLeft: '4px solid #a855f7',
        borderRadius: '0 4px 4px 0',
        paddingLeft: '12px !important',
        marginTop: '0.5rem',
        marginBottom: '0.5rem',
    },
    // Divider Line Styling
    // BASE CLASS: Always applied. handles layout transparency.
    '.cm-hr-line': {
        position: 'relative',
        margin: '0', /* Remove margin to prevent collapse */
        paddingTop: '0rem', /* Reduced from 1.5rem to match visual collapse */
        paddingBottom: '0rem',
        cursor: 'text',
        color: 'transparent', /* Hide text by default */
        lineHeight: '1.75', /* Specific line height match */
    },
    // The Purple Line (Pseudo-element)
    '.cm-hr-line::after': {
        content: '""',
        position: 'absolute',
        top: '50%',
        left: '0',
        right: '0',
        height: '4px',
        backgroundColor: '#a855f7',
        borderRadius: '2px',
        transform: 'translateY(-50%)',
        pointerEvents: 'none',
    },
    // MODIFIER: Active State (Cursor ON line)
    // Simply makes text visible and hides the purple line.
    // Layout properties (padding, etc) are inherited from base class, guaranteeing NO SHIFT.
    '.cm-hr-active': {
        color: 'inherit !important',
    },
    '.cm-hr-active span': {
        color: 'inherit !important', /* Force text visibility on inner span */
    },
    '.cm-hr-active::after': {
        display: 'none', /* Hide the purple line */
    },
    // ========================================================================
    // REMOTE CURSORS (Yjs) - ENHANCED VISUALS
    // ========================================================================
    '.cm-ySelection': {
        backgroundColor: 'rgba(250, 204, 21, 0.2) !important', // Yellow-400 opacity
        margin: '0 -1px',
    },
    '.cm-ySelectionCaret': {
        position: 'relative',
        borderLeft: '2px solid #facc15', // Yellow-400
        marginLeft: '-1px',
    },
    '.cm-ySelectionCaret::before': {
        content: 'none !important', // HIDDEN AS REQUESTED
        display: 'none !important',
    },
    '.cm-ySelectionCaret:hover::before': {
        display: 'none !important',
    },
    // Better: Show on active line or when cursor moves? No, allow simpler UI:
    // Just make it very subtle or only show if it's "Active".
    // User requested "nothing like that on first line".
    // Let's try: ONLY show if we are hovering the EDITOR or the CARET.
    '.cm-content:hover .cm-ySelectionCaret::before': {
        display: 'none !important',
    },

    // Fallback/Legacy classes
    '.yRemoteSelection': {
        backgroundColor: 'rgba(250, 204, 21, 0.2)',
    },
    '.yRemoteSelectionHead': {
        position: 'absolute',
        borderLeft: '2px solid #facc15',
        height: '100%',
        boxSizing: 'border-box',
        zIndex: '10',
    },
    '.yRemoteSelectionHead::after': {
        position: 'absolute',
        content: 'attr(data-name)',
        top: '-1.6em',
        left: '-2px',
        background: '#facc15',
        color: '#000000',
        fontSize: '0.70rem',
        padding: '2px 6px',
        borderRadius: '4px',
        pointerEvents: 'none',
        whiteSpace: 'nowrap',
        fontWeight: '700',
        boxShadow: '0 2px 4px rgba(0,0,0,0.2)',
    }
}, { dark: true });


// ============================================
// HIGHLIGHTING STYLE
// ============================================
const onyxHighlighting = HighlightStyle.define([
    { tag: tags.heading1, color: '#e4e4e7', fontWeight: '800', fontSize: '2.25em' },
    { tag: tags.heading2, color: '#e4e4e7', fontWeight: '700', fontSize: '1.75em' },
    { tag: tags.heading3, color: '#e4e4e7', fontWeight: '600', fontSize: '1.5em' },
    { tag: tags.heading4, color: '#e4e4e7', fontWeight: '600', fontSize: '1.25em' },
    { tag: tags.heading5, color: '#e4e4e7', fontWeight: '600', fontSize: '1.1em' },
    { tag: tags.heading6, color: '#e4e4e7', fontWeight: '600', fontSize: '1em' },
    { tag: tags.strong, color: '#e4e4e7', fontWeight: '700' },
    { tag: tags.emphasis, fontStyle: 'italic', color: '#d4d4d8' },
    { tag: tags.link, color: '#a855f7', textDecoration: 'underline' },
    { tag: tags.url, color: '#71717a' },
    { tag: tags.quote, color: '#a1a1aa', borderLeft: '2px solid #3f3f46', fontStyle: 'italic' },
    { tag: tags.monospace, color: '#f472b6', backgroundColor: 'rgba(244, 114, 182, 0.1)', borderRadius: '3px', padding: '0 2px' }, // Inline code (Pink)
    { tag: tags.comment, color: '#52525b', fontStyle: 'italic' },
    { tag: tags.list, color: '#a1a1aa' },
    { tag: tags.processingInstruction, color: '#71717a' }, // Math delimiters
]);

// Markdown Styling Plugin
const getMarkdownDecorations = (showDividers: boolean) => ViewPlugin.fromClass(class {
    decorations: DecorationSet;
    constructor(view: EditorView) {
        this.decorations = this.compute(view);
    }
    update(u: ViewUpdate) {
        if (u.docChanged || u.viewportChanged || u.selectionSet) this.decorations = this.compute(u.view);
    }
    compute(view: EditorView): DecorationSet {
        const widgets = [];
        const selection = view.state.selection.main;

        for (const { from, to } of view.visibleRanges) {
            for (let pos = from; pos <= to;) {
                const line = view.state.doc.lineAt(pos);
                const text = line.text.trim();
                const isCursorOnLine = selection.head >= line.from && selection.head <= line.to;

                // Blockquote
                if (text.startsWith('>')) {
                    widgets.push(Decoration.line({ class: 'cm-blockquote-line' }).range(line.from));
                }
                // Divider (---, ***, ___)
                else if (showDividers && /^(\*\*\*|---|___)$/.test(text)) {
                    // Always add base class (Layout)
                    let className = 'cm-hr-line';

                    // If cursor is on line, add active modifier (Visibility)
                    if (isCursorOnLine) {
                        className += ' cm-hr-active';
                    }

                    widgets.push(Decoration.line({ class: className }).range(line.from));
                }

                pos = line.to + 1;
            }
        }
        return Decoration.set(widgets);
    }
}, {
    decorations: v => v.decorations
});

// Helper to wrap selection or insert template
function wrapSelection(view: EditorView, wrapper: string) {
    const { state, dispatch } = view;
    const updates = state.selection.ranges.map(range => {
        if (range.empty) {
            // Insert wrapper around cursor
            // If $$ (Block), add newlines. If $ (Inline), just wrap.
            if (wrapper === '$$') {
                return {
                    range,
                    changes: { from: range.from, insert: "$$\n\n$$" },
                    // Move cursor to middle line
                    selection: { anchor: range.from + 3 }
                };
            } else {
                return {
                    range,
                    changes: { from: range.from, insert: `${wrapper}${wrapper}` },
                    // Move cursor to middle
                    selection: { anchor: range.from + wrapper.length }
                };
            }
        }
        // Wrap existing selection
        return {
            range,
            changes: [
                { from: range.from, insert: wrapper },
                { from: range.to, insert: wrapper }
            ]
        };
    });

    if (updates.length > 0) {
        // Calculate new selection for the FIRST range (primary cursor)
        const primaryUpdate = updates[0];
        let newSelection = undefined;

        if (primaryUpdate.selection) {
            newSelection = primaryUpdate.selection;
        } else {
            // For wrapped text, select the wrapped text? Or just cursor at end?
            // Standard is to select the text inside.
            // We inserted wrapper at from and to.
            // Original: [from, to] -> New: [from+len, to+len]
            // We want selection to assume the whole range?
            // Let's just put cursor at end of wrapper for now to be safe, or keep selection.
            // To keep selection: anchor += len, head += len
            newSelection = {
                anchor: primaryUpdate.range.from + wrapper.length,
                head: primaryUpdate.range.to + wrapper.length
            };
        }

        dispatch(state.update({
            changes: updates.flatMap(u => u.changes),
            scrollIntoView: true,
            selection: newSelection
        }));
        return true;
    }
    return false;
}


interface EditorProps {
    activeNoteId: string | null;
}

export default function Editor({ activeNoteId }: EditorProps) {
    const { updateFile } = useSync();
    const settings = useSettings();
    const {
        fontFamily, fontSize, lineHeight, showMath, showDividers,
        lineNumbers: prefLineNumbers, wordWrap: prefWordWrap, highlightActiveLine: prefHighlightActiveLine
    } = settings;

    // DEMO MODE OVERRIDES
    const isDemo = import.meta.env.VITE_DEMO_MODE === 'true' || import.meta.env.VITE_DEMO_MODE === true;
    const effectiveLineNumbers = isDemo ? false : prefLineNumbers;

    // Force clean font in demo mode, otherwise respect user setting
    const finalFontFamily = isDemo ? 'Inter, system-ui, sans-serif' : (fontFamily === 'System' ? 'system-ui' : fontFamily);

    // UI State
    const [title, setTitle] = useState('');
    const [zoom, setZoom] = useState(1);
    const [status, setStatus] = useState<'connecting' | 'connected' | 'offline'>('connecting');
    const [mathMenuData, setMathMenuData] = useState<MathMenuState | null>(null);

    // Refs
    const editorRef = useRef<HTMLDivElement>(null);
    const viewRef = useRef<EditorView | null>(null);
    const [showFindWidget, setShowFindWidget] = useState(false);
    const [findFocusSignal, setFindFocusSignal] = useState(0);

    // Yjs Refs - Single Connection per Note
    const providerRef = useRef<HocuspocusProvider | null>(null);
    const yDocRef = useRef<Y.Doc | null>(null);

    // Mirror Refs
    const lastSavedTitleRef = useRef<string | null>(null);
    const lastWriteTimeRef = useRef<number>(0);


    // ============================================
    // LOAD NOTE & YJS SETUP
    // ============================================
    useEffect(() => {
        if (!activeNoteId) {
            if (viewRef.current) {
                viewRef.current.destroy();
                viewRef.current = null;
            }
            setTitle('');
            return;
        }

        console.log("Initializing Yjs for Note:", activeNoteId);
        setStatus('connecting');
        setTitle('');

        if (viewRef.current) {
            viewRef.current.destroy();
            viewRef.current = null;
        }

        // PERSISTENT FILE MAPPING
        const storedFilename = localStorage.getItem(`mirror-filename-${activeNoteId}`);
        const storedLastWrite = localStorage.getItem(`mirror-lastwrite-${activeNoteId}`);
        lastSavedTitleRef.current = storedFilename;
        lastWriteTimeRef.current = storedLastWrite ? parseInt(storedLastWrite, 10) : 0;

        const doc = new Y.Doc();
        yDocRef.current = doc;

        const persistence = new IndexeddbPersistence(`onyx-note-${activeNoteId}`, doc);

        // Wait for IndexedDB to sync before creating editor
        persistence.on('synced', async () => {
            console.log('[Editor] IndexedDB synced for note:', activeNoteId);

            // STARTUP SYNC: Check if mirror file is newer than our last write
            if (settings.mirrorEnabled && storedFilename && storedLastWrite) {
                try {
                    let basePath = settings.mirrorPath;
                    if (!basePath) {
                        const docs = await documentDir();
                        basePath = await join(docs, 'Onyx Notes');
                    }
                    const fullPath = await join(basePath, `${storedFilename}.md`);

                    if (await exists(fullPath)) {
                        const metadata = await stat(fullPath);
                        const diskMtime = metadata.mtime ? new Date(metadata.mtime).getTime() : 0;
                        const lastWrite = parseInt(storedLastWrite, 10);

                        if (diskMtime > lastWrite + 100) {
                            console.log('[Mirror] Startup sync: File is newer! Loading from disk...');
                            const newContent = await readTextFile(fullPath);

                            doc.transact(() => {
                                const yText = doc.getText('codemirror');
                                yText.delete(0, yText.length);
                                yText.insert(0, newContent);
                            });

                            lastWriteTimeRef.current = diskMtime;
                            localStorage.setItem(`mirror-lastwrite-${activeNoteId}`, diskMtime.toString());
                            console.log('[Mirror] Startup sync: Content loaded from file.');
                        }
                    }
                } catch (e) {
                    console.warn('[Mirror] Startup sync failed:', e);
                }
            }

            // Only create offline editor if not already created and not logged in
            import('../../lib/pocketbase').then(async ({ pb }) => {
                if (!pb.authStore.isValid && editorRef.current && !viewRef.current) {
                    console.log('[Editor] Creating offline editor after sync');
                    const text = doc.getText('codemirror');
                    const state = EditorState.create({
                        doc: text.toString(),
                        extensions: [
                            EditorView.editable.of(true),
                            EditorState.allowMultipleSelections.of(true),
                            history(),
                            drawSelection(),
                            ...(prefHighlightActiveLine ? [cmHighlightActiveLine()] : []),
                            ...(prefWordWrap ? [EditorView.lineWrapping] : []),
                            autoPairs,
                            hideMarkdown,
                            EditorView.updateListener.of((update) => {
                                if (update.docChanged) {
                                    const transaction = update.transactions.find(tr => tr.docChanged);
                                    if (transaction) {
                                        transaction.changes.iterChanges((fromA, toA, _fromB, _toB, inserted) => {
                                            text.delete(fromA, toA - fromA);
                                            text.insert(fromA, inserted.toString());
                                        });
                                    }
                                }
                            }),
                            keymap.of([
                                { key: 'Mod-f', run: () => { setShowFindWidget(true); setFindFocusSignal(prev => prev + 1); return true; } },
                                { key: 'Mod-b', run: (view) => { wrapSelection(view, '**'); return true; } },
                                { key: 'Mod-i', run: (view) => { wrapSelection(view, '*'); return true; } },
                                { key: 'Mod-u', run: (view) => { wrapSelection(view, '__'); return true; } },
                                { key: 'Mod-`', run: (view) => { wrapSelection(view, '`'); return true; } },
                                { key: 'Mod-m', run: (view) => { wrapSelection(view, '$'); return true; } },
                                {
                                    key: 'Mod-Shift-m', run: (view) => {
                                        const range = view.state.selection.main;
                                        view.dispatch({
                                            changes: { from: range.from, to: range.to, insert: '$$$$' },
                                            selection: { anchor: range.from + 2 }
                                        });
                                        return true;
                                    }
                                },
                                ...defaultKeymap,
                                ...historyKeymap,
                                indentWithTab,
                                ...foldKeymap,
                                ...markdownKeymap,
                                ...mathMenuKeymap
                            ]),
                            markdown({ base: markdownLanguage, codeLanguages: languages }),
                            syntaxHighlighting(onyxHighlighting),
                            codeFolding(),
                            ...(effectiveLineNumbers ? [foldGutter(),] : []),
                            ...(effectiveLineNumbers ? [cmLineNumbers()] : []),
                            placeholder('Start typing... (Changes save locally)'),
                            onyxTheme,
                            search({ top: true }),
                            ...(showMath ? [mathLivePreview, mathTooltip, mathMenuPlugin, mathMenuStateField, mathAutoReplace] : []),
                            EditorView.updateListener.of((update) => {
                                const nextState = update.state.field(mathMenuStateField, false);
                                setMathMenuData(nextState ? { ...nextState } : null);
                            }),
                            getMarkdownDecorations(showDividers),
                        ],
                    });

                    const view = new EditorView({ state, parent: editorRef.current });
                    viewRef.current = view;
                    setStatus('offline');
                }
            });
        });

        // 1. WebSocket Provider
        const wsUrl = import.meta.env.VITE_WS_URL || 'ws://localhost:1234';

        import('../../lib/pocketbase').then(async ({ pb }) => {
            if (!pb.authStore.isValid) {
                console.warn('[Editor] No auth token, operating in offline mode');
                return;
            }

            const token = pb.authStore.token;
            const userId = pb.authStore.model?.id;

            if (!token || !userId) {
                setStatus('offline');
                return;
            }

            const roomName = `user-${userId}-note-${activeNoteId}`;
            const provider = new HocuspocusProvider({
                url: wsUrl,
                name: roomName,
                document: doc,
                token: token,
                onStatus: ({ status }) => {
                    setStatus(status as any);
                },
                onAwarenessUpdate: () => {
                }
            });
            providerRef.current = provider;

            const userColor = '#' + Math.floor(Math.random() * 16777215).toString(16);
            const userName = pb.authStore.model?.email?.split('@')[0] || 'User';
            if (provider.awareness) {
                provider.awareness.setLocalStateField('user', { name: userName, color: userColor });
            }

            if (editorRef.current && !viewRef.current) {
                const state = EditorState.create({
                    doc: doc.getText('codemirror').toString(),
                    extensions: [
                        EditorView.editable.of(true),
                        EditorState.allowMultipleSelections.of(true),
                        history(),
                        drawSelection(),
                        ...(prefHighlightActiveLine ? [cmHighlightActiveLine()] : []),
                        ...(prefWordWrap ? [EditorView.lineWrapping] : []),
                        autoPairs,
                        hideMarkdown,
                        yCollab(doc.getText('codemirror'), provider.awareness, { undoManager: false }),
                        keymap.of([
                            { key: 'Mod-f', run: () => { setShowFindWidget(true); setFindFocusSignal(prev => prev + 1); return true; } },
                            { key: 'Mod-b', run: (view) => { wrapSelection(view, '**'); return true; } },
                            { key: 'Mod-i', run: (view) => { wrapSelection(view, '*'); return true; } },
                            { key: 'Mod-u', run: (view) => { wrapSelection(view, '__'); return true; } }, // Added Underline
                            { key: 'Mod-`', run: (view) => { wrapSelection(view, '`'); return true; } },
                            { key: 'Mod-m', run: (view) => { wrapSelection(view, '$'); return true; } },
                            { key: 'Mod-Shift-m', run: (view) => { wrapSelection(view, '$$'); return true; } },
                            ...defaultKeymap,
                            ...historyKeymap,
                            indentWithTab,
                            ...foldKeymap,
                            ...markdownKeymap,
                            ...mathMenuKeymap
                        ]),
                        markdown({ base: markdownLanguage, codeLanguages: languages }),
                        syntaxHighlighting(onyxHighlighting),
                        codeFolding(),
                        ...(prefLineNumbers ? [foldGutter(),] : []),
                        ...(prefLineNumbers ? [cmLineNumbers()] : []),
                        placeholder('Start typing... (Changes save instantly)'),
                        onyxTheme,
                        search({ top: true }),
                        ...(showMath ? [mathLivePreview, mathTooltip, mathMenuPlugin, mathMenuStateField, mathAutoReplace] : []),
                        EditorView.updateListener.of((update) => {
                            const nextState = update.state.field(mathMenuStateField, false);
                            setMathMenuData(nextState ? { ...nextState } : null);
                        }),
                        getMarkdownDecorations(showDividers),
                    ],
                });

                const view = new EditorView({ state, parent: editorRef.current });
                viewRef.current = view;
            }
        });

        // 4. Bind Title Update
        const metaMap = doc.getMap('meta');
        const updateTitleFromMap = () => {
            const newTitle = metaMap.get('title') as string;
            if (newTitle !== undefined) {
                setTitle(newTitle || '');
            }
        };
        metaMap.observe(updateTitleFromMap);
        updateTitleFromMap();

        // 5. LOCAL MIRROR LOGIC (Phase 19)
        let mirrorTimeout: ReturnType<typeof setTimeout> | null = null;
        let mirrorObserver: (() => void) | null = null;

        if (settings.mirrorEnabled) {
            const text = doc.getText('codemirror');

            const handleMirrorSave = async () => {
                const currentTitle = metaMap.get('title') as string || 'Untitled';
                const safeTitle = currentTitle.replace(/[^a-z0-9\u00a0-\uffff\-_\. ]/gi, '_').trim();

                console.log('[Mirror] Starting save process for:', safeTitle || 'Untitled');
                let basePath = settings.mirrorPath;
                if (!basePath) {
                    try {
                        const docs = await documentDir();
                        basePath = await join(docs, 'Onyx Notes');
                    } catch (e) {
                        console.error('[Mirror] Failed to resolve docs dir:', e);
                        return;
                    }
                }

                try {
                    const dirExists = await exists(basePath);
                    if (!dirExists) {
                        await mkdir(basePath, { recursive: true });
                    }

                    // RENAME HANDLING: Check if title changed
                    // Use the actual file basename (including 'Untitled' fallback)
                    let fileBasename = safeTitle || 'Untitled';

                    // TITLE CONFLICT DETECTION: Check if this filename belongs to a different note
                    // AND the file actually exists on disk (handles external deletions)
                    let hasConflict = false;
                    for (let i = 0; i < localStorage.length; i++) {
                        const key = localStorage.key(i);
                        if (key && key.startsWith('mirror-filename-') && !key.endsWith(activeNoteId)) {
                            const otherFilename = localStorage.getItem(key);
                            if (otherFilename === fileBasename) {
                                // Also check if that file actually exists
                                const otherFilePath = await join(basePath, `${otherFilename}.md`);
                                if (await exists(otherFilePath)) {
                                    hasConflict = true;
                                    break;
                                }
                            }
                        }
                    }

                    if (hasConflict) {
                        // Find the next available number suffix (1, 2, 3, etc.)
                        let suffix = 1;
                        let candidateName = `${fileBasename} ${suffix}`;

                        // Check if this numbered filename exists on disk OR in localStorage
                        while (true) {
                            const candidatePath = await join(basePath, `${candidateName}.md`);
                            const fileExists = await exists(candidatePath);

                            // Also check localStorage for this candidate
                            let lsConflict = false;
                            for (let i = 0; i < localStorage.length; i++) {
                                const key = localStorage.key(i);
                                if (key && key.startsWith('mirror-filename-') && !key.endsWith(activeNoteId)) {
                                    if (localStorage.getItem(key) === candidateName) {
                                        lsConflict = true;
                                        break;
                                    }
                                }
                            }

                            if (!fileExists && !lsConflict) break;
                            suffix++;
                            candidateName = `${fileBasename} ${suffix}`;
                        }

                        fileBasename = candidateName;
                        console.log('[Mirror] Title conflict detected! Using unique filename:', fileBasename);
                    }

                    const fileName = `${fileBasename}.md`;

                    if (lastSavedTitleRef.current !== null && lastSavedTitleRef.current !== fileBasename) {
                        const oldFileName = `${lastSavedTitleRef.current}.md`;
                        const oldFullPath = await join(basePath, oldFileName);
                        const newFullPath = await join(basePath, fileName);

                        console.log('[Mirror] Detecting rename. Old:', oldFileName, 'New:', fileName);
                        try {
                            if (await exists(oldFullPath)) {
                                // Use atomic rename to avoid trash clutter and improve performance
                                await rename(oldFullPath, newFullPath);
                                console.log('[Mirror] Renamed file:', oldFileName, '->', fileName);
                            }
                        } catch (e) {
                            console.warn('[Mirror] Failed to rename file, falling back to write:', e);
                        }
                    }

                    const fullPath = await join(basePath, fileName);
                    const content = text.toString();

                    await writeTextFile(fullPath, content);

                    // Update last write time to prevent self-reload loop
                    try {
                        const metadata = await stat(fullPath);
                        lastWriteTimeRef.current = metadata.mtime ? new Date(metadata.mtime).getTime() : Date.now();
                        console.log('[Mirror] Saved and updated write time:', lastWriteTimeRef.current);
                    } catch (e) {
                        lastWriteTimeRef.current = Date.now();
                    }

                    console.log('[Mirror] Successfully saved to:', fullPath);

                    // PERSISTENT FILE MAPPING: Update localStorage with filename and lastWriteTime
                    localStorage.setItem(`mirror-filename-${activeNoteId}`, fileBasename);
                    localStorage.setItem(`mirror-lastwrite-${activeNoteId}`, lastWriteTimeRef.current.toString());

                    // Update in-memory ref too
                    lastSavedTitleRef.current = fileBasename;

                } catch (e) {
                    console.error('[Mirror] Save failed with error:', e);
                }
            };

            mirrorObserver = () => {
                if (mirrorTimeout) clearTimeout(mirrorTimeout);
                mirrorTimeout = setTimeout(handleMirrorSave, 1000);
            };

            text.observe(mirrorObserver);
            console.log('[Mirror] Observer attached for note:', activeNoteId);

            // Trigger immediate save to ensure file exists (even if empty/untitled)
            handleMirrorSave();

            // Observe title changes to trigger rename immediately
            const handleTitleChange = () => {
                if (mirrorTimeout) clearTimeout(mirrorTimeout);
                mirrorTimeout = setTimeout(handleMirrorSave, 1000);
            };
            metaMap.observe(handleTitleChange);


            // 6. TWO-WAY SYNC (Watch for external changes on focus)
            const checkExternalChanges = async () => {
                if (!settings.mirrorPath || !settings.mirrorEnabled) return;

                const currentTitle = metaMap.get('title') as string || 'Untitled';
                const safeTitle = currentTitle.replace(/[^a-z0-9\u00a0-\uffff\-_\. ]/gi, '_').trim();
                const fileName = `${safeTitle || 'Untitled'}.md`;

                try {
                    let basePath = settings.mirrorPath;
                    if (!basePath) {
                        const docs = await documentDir();
                        basePath = await join(docs, 'Onyx Notes');
                    }
                    const fullPath = await join(basePath, fileName);

                    if (await exists(fullPath)) {
                        const metadata = await stat(fullPath);
                        const diskMtime = metadata.mtime ? new Date(metadata.mtime).getTime() : 0;

                        // If disk file is newer than our last write, reload
                        // Small buffer (100ms) to avoid filesystem timing jitter
                        if (lastWriteTimeRef.current && diskMtime > (lastWriteTimeRef.current + 100)) {
                            console.log('[Mirror] External change detected! Reloading from disk...');
                            const newContent = await readTextFile(fullPath);

                            // Update Yjs doc transactionally
                            doc.transact(() => {
                                const yText = doc.getText('codemirror');
                                yText.delete(0, yText.length);
                                yText.insert(0, newContent);
                            });

                            // Update our last write time so we don't reload again immediately
                            lastWriteTimeRef.current = diskMtime;
                            console.log('[Mirror] Reload complete.');
                        }
                    }
                } catch (e) {
                    console.warn('[Mirror] Failed to check external changes:', e);
                }
            };

            window.addEventListener('focus', checkExternalChanges);
            // Also check periodically if window is visible? No, focus is standard efficient way.

            // Clean up listener
            return () => {
                window.removeEventListener('focus', checkExternalChanges);
                // ... other cleanup handled by parent return
            };
        }

        return () => {
            console.log("Cleaning up Yjs for Note:", activeNoteId);
            if (viewRef.current) {
                viewRef.current.destroy();
                viewRef.current = null;
            }
            if (providerRef.current) providerRef.current.destroy();

            // Cleanup mirror observer
            if (mirrorObserver) {
                doc.getText('codemirror').unobserve(mirrorObserver);
            }
            if (mirrorTimeout) clearTimeout(mirrorTimeout);

            doc.destroy();
            persistence.destroy();
        };
    }, [activeNoteId, settings.mirrorEnabled, settings.mirrorPath]);

    // Note: Mirror logic is now integrated into the main Yjs useEffect above


    // Handle Title Change
    const handleTitleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
        const newTitle = e.target.value;
        setTitle(newTitle);

        if (activeNoteId && yDocRef.current) {
            // 1. Update Y.Doc meta (propagate to other editors of this note)
            yDocRef.current.getMap('meta').set('title', newTitle);

            // 2. Update File List (via Context)
            updateFile(activeNoteId, { title: newTitle });
        }
    };

    // Zoom Handler
    useEffect(() => {
        const handleWheel = (e: WheelEvent) => {
            if (e.ctrlKey) {
                e.preventDefault();
                setZoom(prev => Math.max(0.5, Math.min(2, prev + (e.deltaY > 0 ? -0.1 : 0.1))));
            }
        };
        window.addEventListener('wheel', handleWheel, { passive: false });
        return () => window.removeEventListener('wheel', handleWheel);
    }, []);

    if (!activeNoteId) {
        return (
            <div className="flex-1 flex items-center justify-center text-zinc-600 select-none">
                <div className="text-center">
                    <p className="text-lg font-medium mb-1">No Page Selected</p>
                    <p className="text-sm">Select a page from the sidebar or create a new one.</p>
                </div>
            </div>
        );
    }

    return (
        <div className="flex flex-col h-full bg-zinc-950 relative group/editor">
            {/* Find Widget */}
            {showFindWidget && viewRef.current && (
                <div className="absolute top-4 right-8 z-50">
                    <FindWidget
                        view={viewRef.current}
                        onClose={() => setShowFindWidget(false)}
                        focusSignal={findFocusSignal}
                    />
                </div>
            )}

            {/* Math Menu Overlay */}
            {mathMenuData && (
                <MathMenu
                    visible={mathMenuData.isOpen}
                    position={{ x: mathMenuData.x, y: mathMenuData.y }}
                    options={mathMenuData.filteredOptions}
                    selectedIndex={mathMenuData.selectedIndex}
                    onSelect={(cmd) => {
                        const view = viewRef.current;
                        if (!view) return;

                        const main = view.state.selection.main;
                        const line = view.state.doc.lineAt(main.from);
                        const lineText = line.text;
                        const relPos = main.from - line.from;

                        let start = relPos - 1;
                        while (start >= 0) {
                            if (lineText[start] === '\\') break;
                            start--;
                        }

                        if (start !== -1) {
                            const from = line.from + start;
                            const hasArgs = cmd.endsWith("{}");
                            const insertText = hasArgs ? cmd : cmd + " ";

                            view.dispatch({
                                changes: { from: from, to: main.from, insert: insertText },
                                selection: { anchor: from + insertText.length - (hasArgs ? 1 : 0) }
                            });
                            view.focus();
                        }
                    }}
                />
            )}

            {/* Title Input */}
            <div className="px-12 pt-8 pb-4 shrink-0">
                <input
                    type="text"
                    value={title}
                    onChange={handleTitleChange}
                    placeholder="Untitled"
                    className="w-full bg-transparent text-4xl font-bold text-zinc-100 placeholder-zinc-700 outline-none border-none p-0 pb-2 leading-normal"
                />
            </div>

            {/* Editor Container */}
            <div
                ref={editorRef}
                className="flex-1 w-full h-full overflow-hidden focus:outline-none"
                style={{
                    fontSize: `${zoom * (fontSize / 16)}rem`,
                    fontFamily: finalFontFamily,
                    lineHeight: lineHeight
                }}
            />

            {/* Status Bar */}
            {/* Status Bar - Hidden in Demo Mode */}
            {!import.meta.env.VITE_DEMO_MODE && (
                <div className="h-6 bg-zinc-900 border-t border-zinc-800 flex items-center px-4 justify-between text-[10px] text-zinc-500 select-none">
                    <div className="flex items-center gap-2">
                        <span>{status === 'connected' ? 'Synced' : status}</span>
                    </div>
                    <div>
                        {/* Word count or cursor position could go here */}
                    </div>
                </div>
            )}


        </div>
    );
}
