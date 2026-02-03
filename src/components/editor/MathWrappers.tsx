import React, { Suspense } from 'react';

// Preload function - call this when user types $$ or uses /math
export const preloadMath = () => {
    // Trigger the dynamic import early so it's cached
    import('react-katex');
};

// Lazy load the components
const InlineMathComponent = React.lazy(() => import('react-katex').then(module => ({ default: module.InlineMath })));
const BlockMathComponent = React.lazy(() => import('react-katex').then(module => ({ default: module.BlockMath })));

// Load CSS
import 'katex/dist/katex.min.css';

interface MathProps {
    math: string;
}

export const LazyInlineMath = ({ math }: MathProps) => (
    <Suspense fallback={<span className="text-purple-400 opacity-50 font-mono text-xs">...</span>}>
        <InlineMathComponent math={math} />
    </Suspense>
);

export const LazyBlockMath = ({ math }: MathProps) => (
    <Suspense fallback={<div className="h-12 w-full bg-zinc-800/20 animate-pulse rounded-lg flex items-center justify-center text-zinc-600 font-mono text-xs">Loading Math...</div>}>
        <BlockMathComponent math={math} />
    </Suspense>
);
