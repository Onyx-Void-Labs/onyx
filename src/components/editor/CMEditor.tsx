import React, { useCallback, useEffect, useRef } from 'react';
import { EditorView, keymap, placeholder, drawSelection, highlightActiveLine } from '@codemirror/view';
import { EditorState, Extension } from '@codemirror/state';
import { markdown, markdownLanguage } from '@codemirror/lang-markdown';
import { languages } from '@codemirror/language-data';
import { defaultKeymap, history, historyKeymap } from '@codemirror/commands';
import { HighlightStyle, syntaxHighlighting } from '@codemirror/language';
import { tags } from '@lezer/highlight';

// ============================================
// ONYX DARK THEME
// ============================================
const onyxTheme = EditorView.theme({
    '&': {
        backgroundColor: 'transparent',
        color: '#a1a1aa', // zinc-400
        fontSize: '1.125rem',
        fontFamily: 'inherit',
    },
    '.cm-content': {
        caretColor: '#a855f7', // purple-500
        padding: '0',
        lineHeight: '1.75',
    },
    '.cm-cursor': {
        borderLeftColor: '#a855f7',
        borderLeftWidth: '2px',
    },
    '.cm-selectionBackground, &.cm-focused .cm-selectionBackground': {
        backgroundColor: 'rgba(168, 85, 247, 0.3)', // purple with opacity
    },
    '.cm-activeLine': {
        backgroundColor: 'transparent',
    },
    '.cm-gutters': {
        display: 'none', // Hide line numbers for clean look
    },
    '.cm-line': {
        padding: '0',
    },
    // Markdown styling
    '.cm-header-1': {
        fontSize: '2.5rem',
        fontWeight: '800',
        color: '#f4f4f5', // zinc-100
        letterSpacing: '-0.025em',
    },
    '.cm-header-2': {
        fontSize: '2rem',
        fontWeight: '700',
        color: '#f4f4f5',
        letterSpacing: '-0.025em',
    },
    '.cm-header-3': {
        fontSize: '1.5rem',
        fontWeight: '600',
        color: '#f4f4f5',
    },
    '.cm-strong': {
        fontWeight: '700',
        color: '#e4e4e7', // zinc-200
    },
    '.cm-emphasis': {
        fontStyle: 'italic',
        color: '#d4d4d8', // zinc-300
    },
    '.cm-strikethrough': {
        textDecoration: 'line-through',
    },
    '.cm-link': {
        color: '#a855f7',
        textDecoration: 'underline',
    },
    '.cm-url': {
        color: '#6b7280',
    },
    '.cm-code, .cm-monospace': {
        fontFamily: 'ui-monospace, monospace',
        backgroundColor: 'rgba(39, 39, 42, 0.5)',
        padding: '0.125rem 0.375rem',
        borderRadius: '0.25rem',
        color: '#c084fc', // purple-400
    },
    '.cm-math': {
        color: '#c084fc',
        fontFamily: 'ui-monospace, monospace',
    },
}, { dark: true });

// Syntax highlighting
const onyxHighlighting = HighlightStyle.define([
    { tag: tags.heading1, class: 'cm-header-1' },
    { tag: tags.heading2, class: 'cm-header-2' },
    { tag: tags.heading3, class: 'cm-header-3' },
    { tag: tags.strong, class: 'cm-strong' },
    { tag: tags.emphasis, class: 'cm-emphasis' },
    { tag: tags.strikethrough, class: 'cm-strikethrough' },
    { tag: tags.link, class: 'cm-link' },
    { tag: tags.url, class: 'cm-url' },
    { tag: tags.monospace, class: 'cm-code' },
    { tag: tags.quote, color: '#71717a', fontStyle: 'italic' },
]);

// ============================================
// CODEMIRROR EDITOR COMPONENT
// ============================================
interface CMEditorProps {
    initialContent: string;
    onChange: (content: string) => void;
    onSave?: () => void;
    className?: string;
}

export const CMEditor: React.FC<CMEditorProps> = ({
    initialContent,
    onChange,
    onSave,
    className = ''
}) => {
    const editorRef = useRef<HTMLDivElement>(null);
    const viewRef = useRef<EditorView | null>(null);
    const contentRef = useRef<string>(initialContent);

    // Update handler
    const handleUpdate = useCallback((update: { state: EditorState; docChanged: boolean }) => {
        if (update.docChanged) {
            const newContent = update.state.doc.toString();
            contentRef.current = newContent;
            onChange(newContent);
        }
    }, [onChange]);

    // Save shortcut
    const saveKeymap = keymap.of([{
        key: 'Mod-s',
        run: () => {
            onSave?.();
            return true;
        }
    }]);

    // Initialize editor
    useEffect(() => {
        if (!editorRef.current) return;

        const extensions: Extension[] = [
            // Core
            history(),
            drawSelection(),
            highlightActiveLine(),
            EditorView.lineWrapping,

            // Keymaps
            keymap.of([
                ...defaultKeymap,
                ...historyKeymap,
            ]),
            saveKeymap,

            // Markdown
            markdown({
                base: markdownLanguage,
                codeLanguages: languages,
            }),

            // Theme
            onyxTheme,
            syntaxHighlighting(onyxHighlighting),

            // Placeholder
            placeholder("Start writing... Use # for headings, **bold**, *italic*, $math$"),

            // Update listener
            EditorView.updateListener.of(handleUpdate),
        ];

        const state = EditorState.create({
            doc: initialContent,
            extensions,
        });

        const view = new EditorView({
            state,
            parent: editorRef.current,
        });

        viewRef.current = view;

        // Cleanup
        return () => {
            view.destroy();
            viewRef.current = null;
        };
    }, []); // Only run once on mount

    // Update content when initialContent changes externally
    useEffect(() => {
        if (viewRef.current && initialContent !== contentRef.current) {
            const view = viewRef.current;
            view.dispatch({
                changes: {
                    from: 0,
                    to: view.state.doc.length,
                    insert: initialContent,
                }
            });
            contentRef.current = initialContent;
        }
    }, [initialContent]);

    return (
        <div
            ref={editorRef}
            className={`cm-editor-wrapper ${className}`}
            style={{ minHeight: '100%', width: '100%' }}
        />
    );
};

export default CMEditor;
