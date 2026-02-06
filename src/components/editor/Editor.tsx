/**
 * EditorV2
 * Core CodeMirror 6 editor component for Onyx.
 * Handles markdown editing, theming, encryption, and custom highlighter extensions.
 */

import React, { useState, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
    EditorView,
    keymap,
    drawSelection,
    placeholder,
    ViewPlugin,
    Decoration,
    type DecorationSet,
    type ViewUpdate,
    highlightActiveLine
} from '@codemirror/view';
import { EditorState, Extension, Prec } from '@codemirror/state';
import { markdown, markdownLanguage, markdownKeymap } from '@codemirror/lang-markdown';
import { languages } from '@codemirror/language-data';
import { defaultKeymap, history, historyKeymap, indentWithTab } from '@codemirror/commands';
import { HighlightStyle, syntaxHighlighting, foldGutter, codeFolding, foldKeymap } from '@codemirror/language';
import { tags } from '@lezer/highlight';
import { hideMarkdown } from './extensions/hideMarkdown';
import { autoPairs } from './extensions/autoPairs';
import { bulletLists } from './extensions/bulletLists';
import { mathLivePreview } from './extensions/mathLivePreview';
import { mathTooltip } from './extensions/mathTooltip';
import { mathMenuPlugin, mathMenuKeymap, mathMenuStateField, MathMenuState } from './extensions/mathAutocomplete';
import { MathMenu } from './MathMenu';
import { FindWidget } from './FindWidget';

import { search } from '@codemirror/search';
// Yjs Imports
import * as Y from 'yjs';
// import { IndexeddbPersistence } from 'y-indexeddb'; // CAUSING DUPLICATION
import { yCollab } from 'y-codemirror.next';
import { usePocketBase } from '../../contexts/PocketBaseContext';

import 'katex/dist/katex.min.css'; // KaTeX CSS

// ... existing code ...



// ============================================
const onyxTheme = EditorView.theme({
    '&': {
        backgroundColor: 'transparent',
        color: '#a1a1aa',
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
const markdownDecorations = ViewPlugin.fromClass(class {
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
                else if (/^(\*\*\*|---|___)$/.test(text)) {
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

// Helper to wrap selection
function wrapSelection(view: EditorView, wrapper: string) {
    const { state, dispatch } = view;
    const updates = state.selection.ranges.map(range => {
        if (range.empty) return null;
        return {
            range,
            changes: [
                { from: range.from, insert: wrapper },
                { from: range.to, insert: wrapper }
            ]
        };
    }).filter(u => u !== null);

    if (updates.length > 0) {
        dispatch(state.update({
            changes: updates.flatMap(u => u!.changes),
            scrollIntoView: true,
            selection: { anchor: updates[0]!.range.from + wrapper.length } // Imperfect for multi-select but works for single
        }));
        return true;
    }
    return false;
}


interface EditorProps {
    activeNoteId: number | null;
    onSave: () => void;
    refreshTrigger?: number; // Prop to force re-check of content
}

export default function Editor({ activeNoteId, refreshTrigger = 0, onSave }: EditorProps) {
    // UI State
    const [title, setTitle] = useState('');
    const [zoom, setZoom] = useState(1);

    // Refs for mutable state
    const editorRef = useRef<HTMLDivElement>(null);
    const [mathMenuData, setMathMenuData] = useState<MathMenuState | null>(null);
    const [showFindWidget, setShowFindWidget] = useState(false);
    // Signal to force-focus the FindWidget (increments to trigger effect)
    const [findFocusSignal, setFindFocusSignal] = useState(0);
    // Signal for Live Search Updates (re-count matches on doc/selection change)
    const [searchTick, setSearchTick] = useState(0);
    const viewRef = useRef<EditorView | null>(null);
    const titleRef = useRef<HTMLInputElement>(null);

    // Data refs - SINGLE SOURCE OF TRUTH
    const noteIdRef = useRef<number | null>(null);
    const titleValueRef = useRef<string>('');
    const contentValueRef = useRef<string>('');
    const pbIdRef = useRef<string | null>(null); // Real PB ID
    const saveTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
    const isLoadingRef = useRef(false);

    // Sync State
    const [loadedId, setLoadedId] = useState<number | null>(null); // Prevents View Init race condition

    // Yjs State
    const yDocRef = useRef<Y.Doc | null>(null);
    const yProviderRef = useRef<any | null>(null);

    // PocketBase Sync
    const { pb, user, isConnected } = usePocketBase();


    // ============================================
    // POCKETBASE SYNC
    // ============================================
    useEffect(() => {
        if (!activeNoteId || !user || !isConnected || loadedId !== activeNoteId) return;

        const pbId = pbIdRef.current;
        if (!pbId) return; // If no PB ID yet, we can't subscribe. Save will create it.

        const syncRemote = async () => {
            try {
                // Subscribe to THIS note
                await pb.collection('notes').subscribe(pbId, (e) => {
                    if (e.action === 'update' && e.record.content) {
                        // Check if this update originated from US? 
                        // PocketBase doesn't easily tell us "who" sent it excluding us without custom headers.
                        // Simple check: If content matches current, ignore.
                        if (contentValueRef.current === e.record.content) return;

                        console.log("Remote update received!");
                        // Ideally: Update Yjs doc
                        if (yDocRef.current) {
                            const doc = yDocRef.current;
                            const yText = doc.getText('codemirror');
                            doc.transact(() => {
                                if (yText.toString() !== e.record.content) {
                                    yText.delete(0, yText.length);
                                    yText.insert(0, e.record.content);
                                }
                            });
                        }
                    }
                });
            } catch (err) {
                console.error("Failed to subscribe", err);
            }
        };

        syncRemote();

        return () => {
            if (activeNoteId && pbId) {
                pb.collection('notes').unsubscribe(pbId).catch(() => { });
            }
        };
    }, [activeNoteId, loadedId, user, isConnected, pb]);


    // ============================================
    // SAVE FUNCTION
    // ============================================
    const saveToDatabase = async (instantCallback = false) => {
        // LOCK: Don't save while loading
        if (isLoadingRef.current) return;

        const id = noteIdRef.current;
        if (!id || id !== activeNoteId) return;

        const titleVal = titleValueRef.current;
        const contentVal = contentValueRef.current; // Raw Text

        // Prepare Local Content (JSON wrapper)
        const contentJson = JSON.stringify([{
            id: 'main',
            type: 'p',
            content: contentVal
        }]);

        try {
            // 1. Local Save (Tauri)
            await invoke('update_note', {
                id,
                title: titleVal,
                content: contentJson
            });

            // 2. Cloud Save (PocketBase) - Fire and Forget
            if (user && isConnected) {
                let pbId = pbIdRef.current;

                // Lazy Load: If we don't have a PB ID yet, check if the Sync Engine assigned one in the background
                if (!pbId) {
                    try {
                        const result = await invoke<any>('get_note_content', { id });
                        if (result && result.pb_id) {
                            pbId = result.pb_id;
                            pbIdRef.current = pbId; // Update ref for next time
                            console.log("Lazy loaded PB ID:", pbId);
                        }
                    } catch (e) {
                        // Ignore lookup error
                    }
                }

                if (pbId) {
                    // Update Existing ONLY
                    // We let the Sync Hook handle creation of new notes to avoid race conditions/duplication.
                    pb.collection('notes').update(pbId, {
                        title: titleVal,
                        content: contentVal,
                    }).catch(async (e) => {
                        console.error("Cloud Sync Update Failed", e);
                    });
                }
            }

            if (instantCallback) onSave();
        } catch (e) {
            console.error('Save failed:', e);
        }
    };

    // Debounced save for content
    const scheduleSave = () => {
        if (saveTimerRef.current) clearTimeout(saveTimerRef.current);
        saveTimerRef.current = setTimeout(async () => {
            await saveToDatabase(true); // Call onSave after debounce
        }, 1000);
    };

    // ============================================
    // LOAD NOTE
    // ============================================
    useEffect(() => {
        if (!activeNoteId) {
            noteIdRef.current = null;
            setTitle('');
            setLoadedId(null);
            // Clear editor
            if (viewRef.current) {
                viewRef.current.destroy();
                viewRef.current = null;
            }
            return;
        }

        // Skip if already loaded (prevents re-fetching on small updates)
        // We use a simple check: if IDs match, we usually skip, UNLESS refreshTrigger changed (we can't easily track change direction here without ref,
        // but since we want to fix bugs, let's just ALLOW reloading if we clearly aren't sure).
        // Safest fix for "Notes Vanished": Just load it. The network/invoke overhead is local and tiny.
        // Optimization can come later.

        // However, we MUST NOT overwrite user work if they are typing and we spuriously reload.
        // But `activeNoteId` changing implies a real switch.
        // The only case is 'refreshTrigger' forcing a reload on the SAME note (Locking).

        if (noteIdRef.current === activeNoteId && typeof refreshTrigger !== 'undefined' && refreshTrigger === 0) {
            // If this is just a re-render and Trigger is 0, maybe skip?
            // But treating '0' as special is dangerous.
        }

        // SIMPLIFIED LOGIC:
        // Always load. The specific "skip" optimization was causing the "Same Note" bug.
        // We will just protect against overwriting dirty state if needed, but for now,
        // if activeNoteId changes, we LOAD.

        console.log("Loading note:", activeNoteId);

        isLoadingRef.current = true;
        setLoadedId(null); // Signals "Loading..."

        invoke<any>('get_note_content', { id: activeNoteId }).then(async (result) => {
            if (!result) {
                isLoadingRef.current = false;
                return;
            }

            // --- SECURITY CHECK DISABLED (Emergency Rollback) ---
            // let loadedContent = result.content;
            // let locked = false;

            // try {
            //     const parsed = JSON.parse(loadedContent);
            //     if (parsed.iv && parsed.salt && parsed.data) {
            //         locked = true;
            //     }
            // } catch (e) {
            //     // Not JSON, plain text
            // }

            // // Update Lock State
            // setIsLocked(false); // Force Unlocked

            // // if (locked) { ... } // Disabled Locked Flow

            let loadedContent = result.content;
            if (!loadedContent) loadedContent = "";

            // Normal Flow
            // Handles empty/null/undefined content safely
            if (!loadedContent) loadedContent = "";

            // 1. Parse content (if not locked)
            let content = '';
            try {
                // Handle both JSON format and raw string
                if (loadedContent.trim().startsWith('[')) {
                    const parsed = JSON.parse(loadedContent);
                    if (Array.isArray(parsed)) {
                        content = parsed.map((b: any) => b.content || '').join('\n\n');
                    } else {
                        content = loadedContent || '';
                    }
                } else {
                    content = loadedContent || '';
                }
            } catch (e) {
                content = loadedContent || '';
            }

            // 2. YJS INITIALIZATION
            // Cleanup old doc
            if (yDocRef.current) {
                yDocRef.current.destroy();
            }
            if (yProviderRef.current) {
                yProviderRef.current.destroy();
                yProviderRef.current = null;
            }

            const doc = new Y.Doc();
            const yText = doc.getText('codemirror');

            // Apply loaded content to Yjs (Single Source of Truth)
            // We strip existing content and insert new to ensure FS sync
            doc.transact(() => {
                if (yText.length > 0) yText.delete(0, yText.length);
                yText.insert(0, content);
            });

            // OFFLINE PERSISTENCE (Only for Unlocked Notes)
            // We use the ID as the room name for IndexedDB
            // SECURITY: Locked notes are memory-only.
            /* 
            {
                const provider = new IndexeddbPersistence(`onyx-note-${activeNoteId}`, doc);
                yProviderRef.current = provider;

                // If we have unsaved local changes in IndexedDB that are NEWER than FS, Yjs handles checking? 
                // Actually relying on FS as 'Master' for now is safer for phase 1.
                // So we overwrite Yjs with FS content above.
                // Persistence here mainly acts as a cache for the NEXT session/crash recovery.
            }
            */

            yDocRef.current = doc;

            // 3. Update refs immediately (Single Source of Truth)
            noteIdRef.current = activeNoteId;
            pbIdRef.current = result.pb_id || null;
            titleValueRef.current = result.title || '';
            contentValueRef.current = content;

            // 4. Update UI
            setTitle(result.title || '');

            // Update Editor
            // We rely on the NEW view creation to bind Yjs, so we force a re-mount essentially?
            // Actually, we need to pass the yText to the extensions.
            // Since `extensions` is defined in the dependencies of the *next* useEffect, 
            // updating `yDocRef.current` here isn't enough unless we trigger a re-render or re-init.

            // We'll trust the next useEffect to pick up `yDocRef.current`.
            // But wait, the next useEffect depends on `activeNoteId`.
            // So it runs concurrently? No, sequential often.

            // Hack: trigger a re-render to force editor re-init with new Ydoc
            // We can use a state dummy or just rely on the fact that we set refs.
            // Actually, let's move the View Init logic HERE or unify them. 
            // For now, let's stick to the current pattern:
            // 1. Load Data. 2. Set State. 3. Re-render triggers View Init.

            // To ensure the View Init effect picks up the new YDoc, we need to signal it.
            // `isLoadingRef` toggle might do it if it caused a render, but it uses ref.
            // `setTitle` causes a render.

            isLoadingRef.current = false;
            setLoadedId(activeNoteId);

            // Focus title if empty (User preference)
            if (!result.title) {
                setTimeout(() => titleRef.current?.focus(), 50);
            }
        }).catch(e => {
            console.error('Load failed:', e);
            isLoadingRef.current = false;
        });
    }, [activeNoteId]); // Removed refreshTrigger from dependency array to simplify for now, logic handled internally if needed

    // ============================================
    // INITIALIZE CODEMIRROR
    // ============================================
    useEffect(() => {
        if (!editorRef.current) return;

        // CRITICAL: Wait for Data Load before init
        if (loadedId !== activeNoteId) {
            if (viewRef.current) {
                viewRef.current.destroy();
                viewRef.current = null;
            }
            return;
        }

        // Clean up existing view if any (for HMR)
        if (viewRef.current) {
            viewRef.current.destroy();
            viewRef.current = null;
        }

        const extensions: Extension[] = [
            EditorView.editable.of(true),
            EditorState.allowMultipleSelections.of(true),
            history(),
            drawSelection(),
            highlightActiveLine(),
            EditorView.lineWrapping,
            markdownDecorations, // Add custom decorations
            autoPairs,
            hideMarkdown, // Hide markdown syntax

            keymap.of([
                {
                    key: 'Mod-f', run: () => {
                        setShowFindWidget(true);
                        setFindFocusSignal(prev => prev + 1); // Signal to focus
                        return true;
                    }
                }, // Ctrl+F

                // Formatting shortcuts (Highest Priority)
                { key: 'Ctrl-b', run: (view) => { wrapSelection(view, '**'); return true; } },
                { key: 'Ctrl-i', run: (view) => { wrapSelection(view, '*'); return true; } },
                { key: 'Ctrl-`', run: (view) => { wrapSelection(view, '`'); return true; } },
                { key: 'Ctrl-m', run: (view) => { wrapSelection(view, '$'); return true; } },
                { key: 'Ctrl-Shift-m', run: (view) => { wrapSelection(view, '$$'); return true; } },

                // Disable Comment shortcut (Ctrl-/)
                { key: 'Ctrl-/', run: () => true },

                // Mac equivalents
                { key: 'Cmd-b', run: (view) => { wrapSelection(view, '**'); return true; } },
                { key: 'Cmd-i', run: (view) => { wrapSelection(view, '*'); return true; } },
                { key: 'Cmd-`', run: (view) => { wrapSelection(view, '`'); return true; } },
                { key: 'Cmd-m', run: (view) => { wrapSelection(view, '$'); return true; } },
                { key: 'Cmd-Shift-m', run: (view) => { wrapSelection(view, '$$'); return true; } },

                ...defaultKeymap,
                ...historyKeymap,
                ...foldKeymap,
                indentWithTab,
                ...markdownKeymap,
            ]),

            markdown({
                base: markdownLanguage,
                codeLanguages: languages,
                extensions: [
                    { remove: ['SetextHeading'] } // Disable setext headers (text followed by ---)
                ]
            }),

            foldGutter({
                markerDOM: (open) => {
                    const el = document.createElement("span");
                    // Add class for Hybrid Visibility logic
                    if (!open) el.className = "cm-fold-closed";

                    if (open) {
                        el.innerHTML = `<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M6 9l6 6 6-6"/></svg>`; // Down
                    } else {
                        el.innerHTML = `<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M9 18l6-6-6-6"/></svg>`; // Right
                    }
                    return el;
                }
            }),
            codeFolding({
                placeholderDOM: (_view, onclick) => {
                    const el = document.createElement("span");
                    el.className = "cm-folded-badge";
                    el.textContent = "..."; // Content exists but hidden via CSS
                    el.onclick = onclick;
                    return el;
                }
            }),

            onyxTheme,
            syntaxHighlighting(onyxHighlighting),
            hideMarkdown,
            autoPairs, // Auto-close pairs: $ ` * ( [ {
            bulletLists, // Nested bullet styling

            // Math Support
            mathLivePreview,
            mathTooltip, // Re-enabled for real-time preview
            mathMenuStateField, // REQUIRED: Register the state field!
            mathMenuPlugin, // The Logic Plugin
            Prec.highest(keymap.of(mathMenuKeymap)), // The Keyboard Interceptor (Highest Priority)
            EditorView.updateListener.of((update) => {
                // Always check if the state field changed, even if doc didn't change (e.g. index update)
                const prevState = update.startState.field(mathMenuStateField, false);
                const nextState = update.state.field(mathMenuStateField, false);

                if (prevState !== nextState) { // Reference comparison works because we create new object on update
                    setMathMenuData(nextState ? { ...nextState } : null);
                }
            }),
            search({ top: true }), // Enable Search commands without default UI (hidden by CSS)

            // YJS COLLAB (The Magic)
            // Only add if we have a valid Doc
            ...(yDocRef.current ? [
                yCollab(yDocRef.current.getText('codemirror'), null, { undoManager: false }) // Disable default Yjs UndoManager to use CM6 history? Or use Yjs history?
                // Actually Yjs + CM6 usually requires `y-codemirror`'s own history handling or we keep CM6 history. 
                // `y-codemirror.next` binds well. Let's keep CM6 `history()` for local undo stack, it usually works fine for single user.
                // For multi-user, we need Yjs UndoManager. For Phase 2 (Local), CM6 history is safer UI-wise.
            ] : []),

            placeholder("Start writing...\n\nUse # for headings, **bold**, *italic*, `code`, $math$"),

            // Content change handler
            EditorView.updateListener.of(update => {
                // Only update if document ACTUALLY changed
                if (update.docChanged) {
                    // Update ref
                    contentValueRef.current = update.state.doc.toString();

                    // Only schedule save if NOT loading
                    if (!isLoadingRef.current) {
                        scheduleSave();
                    }
                }

                // LIVE SEARCH RE-INDEXING
                // If the doc changed OR selection changed, and search is open, triggers re-count.
                if ((update.docChanged || update.selectionSet) && showFindWidget) {
                    setSearchTick(prev => prev + 1);
                }

                // LIVE SEARCH RE-INDEXING
                // If the doc changed OR selection changed, and search is open, triggers re-count.
                if ((update.docChanged || update.selectionSet) && showFindWidget) {
                    setSearchTick(prev => prev + 1);
                }
            }),

            // Instant save on blur (when user switches tabs/clicks away)
            EditorView.domEventHandlers({
                blur: () => {
                    saveToDatabase(true);
                }
            }),
        ];

        const state = EditorState.create({
            doc: contentValueRef.current, // CRITICAL: Init with current content in case load finished first
            extensions,
        });

        const view = new EditorView({
            state,
            parent: editorRef.current,
        });

        viewRef.current = view;

        return () => {
            view.destroy();
            viewRef.current = null;
        };
    }, [activeNoteId, refreshTrigger, loadedId]); // Re-run when note changes or force refresh

    // ============================================
    // TITLE HANDLERS
    // ============================================
    const handleTitleChange = async (e: React.ChangeEvent<HTMLInputElement>) => {
        const newTitle = e.target.value;
        // Update UI
        setTitle(newTitle);
        // Update Ref
        titleValueRef.current = newTitle;

        // INSTANT SAVE FOR TITLE
        // We bypass debounce because user wants instant Sidebar updates
        // But we MUST check isLoadingRef to be safe
        if (!isLoadingRef.current && noteIdRef.current) {
            try {
                // Fire and forget - don't await to keep UI responsive
                saveToDatabase(true);
            } catch (e) {
                console.error("Instant title save error", e);
            }
        }
    };

    const handleTitleKeyDown = (e: React.KeyboardEvent) => {
        if (e.key === 'Enter') {
            e.preventDefault();
            viewRef.current?.focus();
        }
    };

    // ============================================
    // ZOOM
    // ============================================
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

    // ============================================
    // RENDER
    // ============================================
    if (!activeNoteId) {
        return (
            <main className="flex-1 flex items-center justify-center text-zinc-600 text-lg bg-zinc-950">
                <div className="text-center">
                    <div className="text-4xl mb-4 opacity-20">📝</div>
                    <p>Select a note or create a new one</p>
                    <p className="text-sm mt-2 text-zinc-700">Ctrl+P to search</p>
                </div>
            </main>
        );
    }

    return (
        <main
            className="flex-1 overflow-hidden bg-zinc-950 relative flex flex-col"
            style={{ transform: `scale(${zoom})`, transformOrigin: 'top left', width: `${100 / zoom}%`, height: `${100 / zoom}%` }}
        >


            {/* Find Widget Overlay */}
            {showFindWidget && (
                <FindWidget
                    view={viewRef.current}
                    onClose={() => setShowFindWidget(false)}
                    focusSignal={findFocusSignal}
                    searchTick={searchTick} // Pass the tick!
                />
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

                        // Re-calculate replacement range (Backwards from cursor to '\')
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

            {/* Header / Title Area */}
            <div className="p-4 bg-zinc-950/50 backdrop-blur-sm flex items-center justify-between shrink-0 h-14 z-10 w-full">
                {/* Title */}
                <div className="flex-1 mr-4">
                    <input
                        ref={titleRef}
                        data-title-input
                        type="text"
                        value={title}
                        onChange={handleTitleChange}
                        onKeyDown={handleTitleKeyDown}
                        placeholder="Untitled"
                        className="w-full bg-transparent text-zinc-100 text-2xl font-bold tracking-tight outline-none placeholder:text-zinc-700 font-display"
                    />
                </div>
            </div>

            {/* CodeMirror Editor */}
            {/* EMERGENCY ROLLBACK: Disabled UnlockScreen */}
            {/* {isLocked ? (
                <UnlockScreen 
                    onUnlock={handleUnlock}
                    title={title} 
                />
            ) : ( */}
            <div
                ref={editorRef}
                className="flex-1 w-full h-full overflow-hidden focus:outline-none"
                style={{
                    fontSize: `${zoom}px`,
                }}
            />
            {/* )} */}
        </main>
    );
}
