/**
 * EditorV2 - CodeMirror 6 based editor for Onyx
 * 
 * fixes:
 * - Instant title update (Sidebar/Tab bar updates immediately)
 * - Safe saving (locks during load)
 * - Fixes content disappearing bug
 */

import React, { useState, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { EditorView, keymap, placeholder, drawSelection, highlightActiveLine } from '@codemirror/view';
import { EditorState, Extension } from '@codemirror/state';
import { markdown, markdownLanguage } from '@codemirror/lang-markdown';
import { languages } from '@codemirror/language-data';
import { defaultKeymap, history, historyKeymap, indentWithTab } from '@codemirror/commands';
import { HighlightStyle, syntaxHighlighting } from '@codemirror/language';
import { tags } from '@lezer/highlight';
import { hideMarkdown } from './extensions/hideMarkdown';

// ============================================
// ONYX DARK THEME
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
    '.cm-selectionBackground, &.cm-focused .cm-selectionBackground': {
        backgroundColor: 'rgba(168, 85, 247, 0.3)',
    },
    '.cm-activeLine': {
        backgroundColor: 'transparent',
    },
    '.cm-gutters': {
        display: 'none',
    },
    '.cm-line': {
        padding: '0.125rem 0',
    },
}, { dark: true });

// Syntax highlighting
const onyxHighlighting = HighlightStyle.define([
    { tag: tags.heading1, fontSize: '2.5rem', fontWeight: '800', color: '#f4f4f5', letterSpacing: '-0.025em' },
    { tag: tags.heading2, fontSize: '2rem', fontWeight: '700', color: '#f4f4f5', letterSpacing: '-0.025em' },
    { tag: tags.heading3, fontSize: '1.5rem', fontWeight: '600', color: '#f4f4f5' },
    { tag: tags.heading4, fontSize: '1.25rem', fontWeight: '600', color: '#e4e4e7' },
    { tag: tags.strong, fontWeight: '700', color: '#e4e4e7' },
    { tag: tags.emphasis, fontStyle: 'italic', color: '#d4d4d8' },
    { tag: tags.strikethrough, textDecoration: 'line-through', color: '#71717a' },
    { tag: tags.link, color: '#a855f7', textDecoration: 'underline' },
    { tag: tags.url, color: '#6b7280' },
    { tag: tags.monospace, fontFamily: 'ui-monospace, monospace', color: '#c084fc', backgroundColor: 'rgba(39, 39, 42, 0.5)' },
    { tag: tags.quote, color: '#71717a', fontStyle: 'italic' },
    { tag: tags.processingInstruction, color: '#c084fc', fontFamily: 'ui-monospace, monospace' },
]);

// ============================================
// HELPER: Wrap Selection with Markdown
// ============================================
function wrapSelection(view: EditorView, wrapper: string) {
    const { from, to } = view.state.selection.main;
    const selectedText = view.state.sliceDoc(from, to);

    view.dispatch({
        changes: { from, to, insert: `${wrapper}${selectedText}${wrapper}` },
        selection: { anchor: from + wrapper.length, head: to + wrapper.length }
    });
}

// ============================================
// EDITOR COMPONENT
// ============================================
interface EditorV2Props {
    activeNoteId: number | null;
    onSave: () => void;
}

export default function EditorV2({ activeNoteId, onSave }: EditorV2Props) {
    // UI State
    const [title, setTitle] = useState('');
    const [zoom, setZoom] = useState(1);

    // Refs for mutable state
    const editorRef = useRef<HTMLDivElement>(null);
    const viewRef = useRef<EditorView | null>(null);
    const titleRef = useRef<HTMLInputElement>(null);

    // Data refs - SINGLE SOURCE OF TRUTH
    const noteIdRef = useRef<number | null>(null);
    const titleValueRef = useRef<string>('');
    const contentValueRef = useRef<string>('');
    const saveTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
    const isLoadingRef = useRef(false);

    // ============================================
    // SAVE FUNCTION
    // ============================================
    const saveToDatabase = async (instantCallback = false) => {
        // LOCK: Don't save while loading
        if (isLoadingRef.current) return;

        const id = noteIdRef.current;
        if (!id || id !== activeNoteId) return;

        const titleVal = titleValueRef.current;
        const contentVal = contentValueRef.current;

        // Safeguard: Don't save empty content (prevents accidental data loss)
        if (contentVal.length === 0) {
            // Still save title
            try {
                await invoke('update_note', {
                    id,
                    title: titleVal,
                    content: JSON.stringify([{ id: 'main', type: 'p', content: '' }])
                });
                if (instantCallback) onSave();
            } catch (e) {
                console.error('Save failed:', e);
            }
            return;
        }

        try {
            await invoke('update_note', {
                id,
                title: titleVal,
                content: JSON.stringify([{ id: 'main', type: 'p', content: contentVal }])
            });
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
            // Clear editor
            if (viewRef.current) {
                viewRef.current.dispatch({
                    changes: { from: 0, to: viewRef.current.state.doc.length, insert: '' }
                });
            }
            return;
        }

        // Skip if already loaded (prevents re-fetching on small updates)
        // But we must be careful: if onSave triggered a re-render from parent, we don't want to reload
        if (noteIdRef.current === activeNoteId) {
            return;
        }

        isLoadingRef.current = true;

        invoke<any>('get_note_content', { id: activeNoteId }).then(data => {
            if (!data) {
                isLoadingRef.current = false;
                return;
            }

            // 1. Parse content
            let content = '';
            try {
                // Handle both JSON format and raw string
                if (data.content && data.content.trim().startsWith('[')) {
                    const parsed = JSON.parse(data.content);
                    if (Array.isArray(parsed)) {
                        content = parsed.map((b: any) => b.content || '').join('\n\n');
                    } else {
                        content = data.content || '';
                    }
                } else {
                    content = data.content || '';
                }
            } catch (e) {
                console.warn('Content parse fallback:', e);
                content = data.content || '';
            }

            // 2. Update refs immediately (Single Source of Truth)
            noteIdRef.current = activeNoteId;
            titleValueRef.current = data.title || '';
            contentValueRef.current = content; // CRITICAL: Set this before any saves occur!

            // 3. Update UI
            setTitle(data.title || '');

            // Update Editor
            if (viewRef.current) {
                // We use a transaction that doesn't trigger our own update listener logic if possible, 
                // but our listener checks isLoadingRef, so it's safe.
                viewRef.current.dispatch({
                    changes: { from: 0, to: viewRef.current.state.doc.length, insert: content }
                });
            }

            isLoadingRef.current = false;

            // Focus title if empty (User preference)
            if (!data.title) {
                setTimeout(() => titleRef.current?.focus(), 50);
            }
        }).catch(e => {
            console.error('Load failed:', e);
            isLoadingRef.current = false;
        });
    }, [activeNoteId]);

    // ============================================
    // INITIALIZE CODEMIRROR
    // ============================================
    useEffect(() => {
        if (!editorRef.current) return;

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

            keymap.of([
                // Formatting shortcuts (Highest Priority)
                { key: 'Ctrl-b', run: (view) => { wrapSelection(view, '**'); return true; } },
                { key: 'Ctrl-i', run: (view) => { wrapSelection(view, '*'); return true; } },
                { key: 'Ctrl-`', run: (view) => { wrapSelection(view, '`'); return true; } },
                { key: 'Ctrl-m', run: (view) => { wrapSelection(view, '$'); return true; } },
                { key: 'Ctrl-Shift-m', run: (view) => { wrapSelection(view, '$'); return true; } },

                // Disable Comment shortcut (Ctrl-/)
                { key: 'Ctrl-/', run: () => true },

                // Mac equivalents
                { key: 'Cmd-b', run: (view) => { wrapSelection(view, '**'); return true; } },
                { key: 'Cmd-i', run: (view) => { wrapSelection(view, '*'); return true; } },
                { key: 'Cmd-`', run: (view) => { wrapSelection(view, '`'); return true; } },
                { key: 'Cmd-m', run: (view) => { wrapSelection(view, '$'); return true; } },

                ...defaultKeymap,
                ...historyKeymap,
                indentWithTab,
            ]),

            markdown({
                base: markdownLanguage,
                codeLanguages: languages,
            }),

            onyxTheme,
            syntaxHighlighting(onyxHighlighting),
            hideMarkdown,

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
    }, [activeNoteId]); // Re-run when note changes (editor div only exists when note selected)

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
            {/* Title */}
            <div className="px-8 pt-8 pb-4">
                <input
                    ref={titleRef}
                    data-title-input
                    type="text"
                    value={title}
                    onChange={handleTitleChange}
                    onKeyDown={handleTitleKeyDown}
                    placeholder="Untitled"
                    className="w-full bg-transparent text-zinc-100 text-4xl font-black tracking-tight outline-none placeholder:text-zinc-700"
                />
            </div>

            {/* CodeMirror Editor */}
            <div
                ref={editorRef}
                className="flex-1 overflow-auto cursor-text"
                onClick={() => viewRef.current?.focus()}
            />
        </main>
    );
}
