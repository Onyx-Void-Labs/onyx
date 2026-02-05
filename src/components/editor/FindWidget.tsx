import React, { useState, useEffect, useRef } from 'react';
import { EditorView } from '@codemirror/view';
import { SearchQuery, findNext, findPrevious, replaceNext, replaceAll, setSearchQuery, openSearchPanel, closeSearchPanel } from '@codemirror/search';
import { Search, ArrowUp, ArrowDown, X, ChevronDown, ChevronUp, Replace, CaseSensitive, WholeWord, Regex, MousePointer2 } from 'lucide-react';
import { EditorState, EditorSelection } from '@codemirror/state';

interface FindWidgetProps {
    view: EditorView | null;
    onClose: () => void;
    focusSignal?: number; // Optional signal to force focus
    searchTick?: number; // External signal to re-scan
}

export const FindWidget: React.FC<FindWidgetProps> = ({ view, onClose, focusSignal = 0, searchTick = 0 }) => {
    const [isExpanded, setIsExpanded] = useState(false);
    const [searchText, setSearchText] = useState('');
    const [replaceText, setReplaceText] = useState('');

    // Options
    const [caseSensitive, setCaseSensitive] = useState(false);
    const [wholeWord, setWholeWord] = useState(false);
    const [isRegex, setIsRegex] = useState(false);

    const [matchCount, setMatchCount] = useState({ current: 0, total: 0 });
    const inputRef = useRef<HTMLInputElement>(null);

    // Focus Effect (Runs on Mount + Signal)
    useEffect(() => {
        // We use a small timeout to ensure the DOM is ready (animation frame)
        setTimeout(() => {
            if (inputRef.current) {
                inputRef.current.focus();
                inputRef.current.select(); // Select all text for quick replace
            }
        }, 50);
    }, [focusSignal]); // Trigger when signal changes (Ctrl+F pressed again)

    useEffect(() => {
        // Activate search state (highlights depend on panel being "open")
        // We hide the actual panel via CSS (.cm-panel { display: none !important })
        if (view) {
            openSearchPanel(view);
        }

        // Focus handled by focusSignal effect above for robustness

        return () => {
            if (view) closeSearchPanel(view);
        };
    }, [view]);

    // Helper: Count Matches & Current Position
    // We can pass an explicit 'targetMatch' if we just jumped to it, for instant feedback.
    const countMatches = (state: EditorState, query: SearchQuery, targetMatch?: { from: number, to: number }) => {
        let total = 0;
        let current = 0;

        const cursor = query.getCursor(state);
        let match = cursor.next();

        while (!match.done) {
            total++;

            // Priority 1: Check against explicit target (instant update during search/jump)
            if (targetMatch) {
                if (match.value.from === targetMatch.from && match.value.to === targetMatch.to) {
                    current = total;
                }
            }
            // Priority 2: Check against actual selection (updates on click/nav)
            // STRICT: Must be exact selection match.
            else if (match.value.from === state.selection.main.from && match.value.to === state.selection.main.to) {
                current = total;
            }

            match = cursor.next();
        }
        setMatchCount({ current, total });
    };

    // --- Search Logic ---
    const updateSearch = (term: string, useCase: boolean, useWord: boolean, useRegex: boolean) => {
        if (!view) return;

        // Dispatch CodeMirror Search Query
        const query = new SearchQuery({
            search: term,
            caseSensitive: useCase,
            literal: !useRegex,
            wholeWord: useWord,
            regexp: useRegex
        });

        view.dispatch({ effects: setSearchQuery.of(query) });

        // 2. Determine Jump Target & Dispatch
        // We do this BEFORE counting so we can pass the target to countMatches for instant feedback.
        let jumpTarget = null;

        if (term) {
            const currentFrom = view.state.selection.main.from;
            let cursor = query.getCursor(view.state, currentFrom);
            let match = cursor.next();

            // Wrap around
            if (match.done) {
                cursor = query.getCursor(view.state, 0);
                match = cursor.next();
            }

            if (!match.done) {
                jumpTarget = match.value;
                // Dispatch Jump
                view.dispatch({
                    selection: { anchor: match.value.from, head: match.value.to },
                    scrollIntoView: true,
                    effects: EditorView.scrollIntoView(match.value.from, { y: "center" })
                });
            }
        }

        // 3. Count Matches (Pass target if we jumped)
        countMatches(view.state, query, jumpTarget || undefined);

        // Count AFTER the jump (so we detect the new selection as "Current")
        // We defer slightly to ensure state is updated, or we rely on the searchTick.
        // But to be snappy, we check against the *new* state if possible, or just re-run.
        // Actually, since we dispatched a selection change, the EditorV2 will fire a searchTick.
        // BUT, that might be async.
        // Let's force a count checks assuming the jump happened if we found a match.
        // However, we don't have the "next state" easily here without `view.state` update.
        // So we rely on the tick.
        // If it's NOT updating, it means the tick isn't firing or the check is too strict.

    };

    // Effect: Handle Text/Option Changes (Query + Jump)
    useEffect(() => {
        updateSearch(searchText, caseSensitive, wholeWord, isRegex);
    }, [searchText, caseSensitive, wholeWord, isRegex, view]);

    // Effect: Handle Selection/Doc Updates (Count Only - No Jump)
    // This runs when the editor tells us something changed (tick)
    useEffect(() => {
        if (!view) return;
        // Don't re-dispatch query or jump. Just re-count based on current state.
        // We get the query from the state to ensure we match what's active.
        const query = new SearchQuery({
            search: searchText,
            caseSensitive,
            literal: !isRegex,
            wholeWord,
            regexp: isRegex
        });
        countMatches(view.state, query);
    }, [searchTick, view]); // Only run on tick (or view init)

    const handleNext = () => {
        if (view) {
            findNext(view);
            // Force a re-count after a microtask ensuring selection updated
            setTimeout(() => {
                const query = new SearchQuery({
                    search: searchText, caseSensitive, literal: !isRegex, wholeWord, regexp: isRegex
                });
                countMatches(view.state, query);
            }, 10); // Tiny delay to let CodeMirror update selection
        }
    };
    const handlePrev = () => {
        if (view) {
            findPrevious(view);
            setTimeout(() => {
                const query = new SearchQuery({
                    search: searchText, caseSensitive, literal: !isRegex, wholeWord, regexp: isRegex
                });
                countMatches(view.state, query);
            }, 10);
        }
    };
    const handleReplace = () => {
        if (view) {
            const query = new SearchQuery({
                search: searchText,
                caseSensitive: caseSensitive,
                literal: !isRegex,
                wholeWord: wholeWord,
                regexp: isRegex,
                replace: replaceText
            });
            view.dispatch({ effects: setSearchQuery.of(query) });
            replaceNext(view);
        }
    }
    const handleReplaceAll = () => {
        if (view) {
            const query = new SearchQuery({
                search: searchText,
                caseSensitive: caseSensitive,
                literal: !isRegex,
                wholeWord: wholeWord,
                regexp: isRegex,
                replace: replaceText
            });
            view.dispatch({ effects: setSearchQuery.of(query) });
            replaceAll(view);
        }
    }

    const handleSelectAll = () => {
        if (!view || !searchText) return;

        const query = new SearchQuery({
            search: searchText, caseSensitive, literal: !isRegex, wholeWord, regexp: isRegex
        });

        const cursor = query.getCursor(view.state);
        const ranges: any[] = [];
        let match = cursor.next();

        while (!match.done) {
            // User requested: "Backspace removes 1 char" -> Place cursors at the END of matches, don't select the whole word.
            ranges.push(EditorSelection.cursor(match.value.to));
            match = cursor.next();
        }

        if (ranges.length > 0) {
            view.dispatch({
                selection: EditorSelection.create(ranges),
                scrollIntoView: true
            });
            onClose(); // Close widget so user can edit freely
            view.focus();
        }
    };

    // Enter key
    const handleKeyDown = (e: React.KeyboardEvent) => {
        if (e.key === 'Enter') {
            if (e.shiftKey) handlePrev();
            else if (e.altKey) handleSelectAll(); // Alt+Enter shortcut
            else handleNext();
        } else if (e.key === 'Escape') {
            onClose();
            view?.focus();
        }
    };

    return (
        <div className="absolute top-1 right-2 z-50 flex flex-col gap-2 w-[420px] animate-in slide-in-from-top-2 fade-in duration-200">
            {/* Main Bar */}
            <div className={`
                flex flex-col bg-zinc-900/95 backdrop-blur-xl border border-zinc-800 shadow-2xl rounded-xl overflow-hidden
                transition-all duration-300 ease-spring
                ${isExpanded ? 'scale-100' : 'scale-95'}
            `}>
                {/* Header / Title Area */}
                <div className="p-4 bg-zinc-950/50 backdrop-blur-sm flex items-center justify-between shrink-0 h-14 z-10 w-full border-b border-transparent">
                    {/* Title */}
                    <div className="flex-1 mr-4 text-zinc-500">
                        <Search size={16} />
                    </div>
                    <input
                        ref={inputRef}
                        type="text"
                        value={searchText}
                        onChange={e => setSearchText(e.target.value)}
                        onKeyDown={handleKeyDown}
                        placeholder="Find..."
                        className="flex-1 bg-transparent text-sm text-zinc-100 placeholder-zinc-600 outline-none h-8"
                    />

                    {/* Badge (Always Rendered for Layout Stability) */}
                    <div className={`text-xs font-mono text-zinc-500 px-2 min-w-[4rem] text-center whitespace-nowrap flex items-center justify-center transition-opacity duration-200 ${searchText ? 'opacity-100' : 'opacity-0'}`}>
                        {matchCount.total > 0
                            ? (matchCount.current > 0 ? `${matchCount.current} of ${matchCount.total}` : `${matchCount.total}`)
                            : '0'}
                    </div>

                    {/* Controls */}
                    <div className="flex items-center gap-1 border-l border-zinc-800 pl-1">
                        <button onClick={handlePrev} className="p-1.5 hover:bg-zinc-800 rounded-md text-zinc-400 hover:text-zinc-100 transition-colors">
                            <ArrowUp size={14} />
                        </button>
                        <button onClick={handleNext} className="p-1.5 hover:bg-zinc-800 rounded-md text-zinc-400 hover:text-zinc-100 transition-colors">
                            <ArrowDown size={14} />
                        </button>
                    </div>

                    <button
                        onClick={() => setIsExpanded(!isExpanded)}
                        className={`p-1.5 rounded-md transition-colors ml-1 ${isExpanded ? 'bg-zinc-800 text-zinc-100' : 'text-zinc-500 hover:bg-zinc-800 hover:text-zinc-300'}`}
                    >
                        {isExpanded ? <ChevronUp size={14} /> : <ChevronDown size={14} />}
                    </button>

                    <button onClick={onClose} className="p-1.5 hover:bg-red-500/10 hover:text-red-400 rounded-md text-zinc-500 transition-colors">
                        <X size={14} />
                    </button>
                </div>

                {/* Expanded Area */}
                {isExpanded && (
                    <div className="flex flex-col border-t border-zinc-800/50 bg-black/20 animate-in slide-in-from-top-2 fade-in">

                        {/* Row 2: Replace */}
                        <div className="flex items-center gap-2 p-2">
                            <div className="pl-2 text-zinc-500">
                                <Replace size={14} className="opacity-70" />
                            </div>
                            <input
                                type="text"
                                value={replaceText}
                                onChange={e => setReplaceText(e.target.value)}
                                placeholder="Replace with..."
                                className="flex-1 bg-transparent text-sm text-zinc-100 placeholder-zinc-600 outline-none h-8"
                            />
                            <div className="flex items-center gap-1">
                                <button onClick={handleReplace} className="p-1 px-2 hover:bg-zinc-800 rounded-md text-zinc-400 text-xs transition-colors">
                                    Replace
                                </button>
                                <button onClick={handleReplaceAll} className="p-1 px-2 hover:bg-zinc-800 rounded-md text-zinc-400 text-xs transition-colors">
                                    All
                                </button>
                            </div>
                        </div>

                        {/* Row 3: Toggles */}
                        <div className="flex items-start gap-1 p-2 pt-0 pb-3 px-3">
                            <ToggleBtn active={caseSensitive} onClick={() => setCaseSensitive(!caseSensitive)} icon={<CaseSensitive size={14} />} label="Match Case" />
                            <ToggleBtn active={wholeWord} onClick={() => setWholeWord(!wholeWord)} icon={<WholeWord size={14} />} label="Whole Word" />
                            <ToggleBtn active={isRegex} onClick={() => setIsRegex(!isRegex)} icon={<Regex size={14} />} label="Regex" />

                            <div className="w-[1px] h-4 bg-zinc-800 mx-1 self-center" />

                            <button
                                onClick={handleSelectAll}
                                title="Select All Matches (Alt+Enter)"
                                className="p-1.5 rounded-md flex items-center justify-center transition-all duration-200 text-zinc-500 hover:bg-purple-500/20 hover:text-purple-300 hover:shadow-[0_0_10px_rgba(168,85,247,0.2)]"
                            >
                                <MousePointer2 size={14} />
                            </button>
                        </div>
                    </div>
                )}
            </div>
        </div>
    );
};

const ToggleBtn = ({ active, onClick, icon, label }: { active: boolean, onClick: () => void, icon: React.ReactNode, label: string }) => (
    <button
        onClick={onClick}
        title={label}
        className={`
            p-1.5 rounded-md flex items-center justify-center transition-all duration-200
            ${active
                ? 'bg-purple-500/20 text-purple-300 ring-1 ring-purple-500/30 shadow-[0_0_10px_rgba(168,85,247,0.2)]'
                : 'text-zinc-500 hover:bg-zinc-800 hover:text-zinc-300'}
        `}
    >
        {icon}
    </button>
);
