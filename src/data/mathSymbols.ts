export interface MathSymbol {
    cmd: string;      // The LaTeX command (e.g., \alpha)
    name: string;     // The display name
    keywords: string; // Search keywords
    trigger?: string; // Optional: specific text that auto-converts (e.g. "alpha")
}

export const MATH_SYMBOLS: MathSymbol[] = [
    // --- GREEK LOWER (Auto-Triggers enabled) ---
    { cmd: "\\alpha", name: "Alpha", keywords: "a greek", trigger: "alpha" },
    { cmd: "\\beta", name: "Beta", keywords: "b greek", trigger: "beta" },
    { cmd: "\\gamma", name: "Gamma", keywords: "g greek", trigger: "gamma" },
    { cmd: "\\delta", name: "Delta", keywords: "d greek", trigger: "delta" },
    { cmd: "\\epsilon", name: "Epsilon", keywords: "e greek", trigger: "epsilon" },
    { cmd: "\\zeta", name: "Zeta", keywords: "z greek", trigger: "zeta" },
    { cmd: "\\eta", name: "Eta", keywords: "h greek", trigger: "eta" },
    { cmd: "\\theta", name: "Theta", keywords: "t angle greek", trigger: "theta" },
    { cmd: "\\iota", name: "Iota", keywords: "i greek", trigger: "iota" },
    { cmd: "\\kappa", name: "Kappa", keywords: "k greek", trigger: "kappa" },
    { cmd: "\\lambda", name: "Lambda", keywords: "l greek", trigger: "lambda" },
    { cmd: "\\mu", name: "Mu", keywords: "m greek", trigger: "mu" },
    { cmd: "\\nu", name: "Nu", keywords: "n greek", trigger: "nu" },
    { cmd: "\\xi", name: "Xi", keywords: "x greek", trigger: "xi" },
    { cmd: "\\pi", name: "Pi", keywords: "p circle greek", trigger: "pi" },
    { cmd: "\\rho", name: "Rho", keywords: "r greek", trigger: "rho" },
    { cmd: "\\sigma", name: "Sigma", keywords: "s standard deviation greek", trigger: "sigma" },
    { cmd: "\\tau", name: "Tau", keywords: "t greek", trigger: "tau" },
    { cmd: "\\upsilon", name: "Upsilon", keywords: "u greek", trigger: "upsilon" },
    { cmd: "\\phi", name: "Phi", keywords: "f greek", trigger: "phi" },
    { cmd: "\\chi", name: "Chi", keywords: "c greek", trigger: "chi" },
    { cmd: "\\psi", name: "Psi", keywords: "p greek", trigger: "psi" },
    { cmd: "\\omega", name: "Omega", keywords: "w greek", trigger: "omega" },

    // --- GREEK UPPER ---
    { cmd: "\\Gamma", name: "Gamma (Upper)", keywords: "G Greek" },
    { cmd: "\\Delta", name: "Delta (Upper)", keywords: "D change Greek" },
    { cmd: "\\Theta", name: "Theta (Upper)", keywords: "T Greek" },
    { cmd: "\\Lambda", name: "Lambda (Upper)", keywords: "L Greek" },
    { cmd: "\\Xi", name: "Xi (Upper)", keywords: "X Greek" },
    { cmd: "\\Pi", name: "Pi (Upper)", keywords: "P product Greek" },
    { cmd: "\\Sigma", name: "Sigma (Upper)", keywords: "S sum Greek" },
    { cmd: "\\Phi", name: "Phi (Upper)", keywords: "F Greek" },
    { cmd: "\\Psi", name: "Psi (Upper)", keywords: "P Greek" },
    { cmd: "\\Omega", name: "Omega (Upper)", keywords: "W resistance Greek" },

    // --- OPERATIONS (Triggers for common ones) ---
    { cmd: "\\cdot", name: "Dot Product", keywords: "multiply point", trigger: "cdot" },
    { cmd: "\\times", name: "Cross Product", keywords: "multiply x", trigger: "times" },
    { cmd: "\\div", name: "Division", keywords: "divide", trigger: "div" },
    { cmd: "\\pm", name: "Plus Minus", keywords: "+-", trigger: "pm" },
    { cmd: "\\mp", name: "Minus Plus", keywords: "-+" },
    { cmd: "\\sqrt", name: "Square Root", keywords: "root radical", trigger: "sqrt" },
    { cmd: "\\frac", name: "Fraction", keywords: "division ratio" },
    { cmd: "^", name: "Power", keywords: "exponent superscript" },
    { cmd: "_", name: "Subscript", keywords: "underscore index" },

    // --- RELATIONS ---
    { cmd: "\\neq", name: "Not Equal", keywords: "!= unequal", trigger: "neq" },
    { cmd: "\\approx", name: "Approximate", keywords: "~ like", trigger: "approx" },
    { cmd: "\\equiv", name: "Equivalent", keywords: "=== identity", trigger: "equiv" },
    { cmd: "\\leq", name: "Less or Equal", keywords: "<=", trigger: "leq" },
    { cmd: "\\geq", name: "Greater or Equal", keywords: ">=", trigger: "geq" },
    { cmd: "\\ll", name: "Much Less", keywords: "<<" },
    { cmd: "\\gg", name: "Much Greater", keywords: ">>" },
    { cmd: "\\propto", name: "Proportional", keywords: "alpha relation" },

    // --- CALCULUS (Triggers enabled) ---
    { cmd: "\\int", name: "Integral", keywords: "calculus area", trigger: "int" },
    { cmd: "\\iint", name: "Double Integral", keywords: "calculus" },
    { cmd: "\\oint", name: "Contour Integral", keywords: "calculus loop" },
    { cmd: "\\sum", name: "Summation", keywords: "sigma add total", trigger: "sum" },
    { cmd: "\\prod", name: "Product", keywords: "pi multiply", trigger: "prod" },
    { cmd: "\\lim", name: "Limit", keywords: "calculus approach", trigger: "lim" },
    { cmd: "\\partial", name: "Partial Derivative", keywords: "d calculus diff", trigger: "partial" },
    { cmd: "\\nabla", name: "Nabla (Del)", keywords: "gradient vector", trigger: "nabla" },
    { cmd: "\\infty", name: "Infinity", keywords: "forever loop", trigger: "inf" },

    // --- LOGIC & SETS ---
    { cmd: "\\forall", name: "For All", keywords: "A logic every", trigger: "forall" },
    { cmd: "\\exists", name: "Exists", keywords: "E logic some", trigger: "exists" },
    { cmd: "\\nexists", name: "Not Exists", keywords: "logic none" },
    { cmd: "\\in", name: "Element Of", keywords: "set inside", trigger: "in" },
    { cmd: "\\notin", name: "Not Element", keywords: "set outside", trigger: "notin" },
    { cmd: "\\subset", name: "Subset", keywords: "contained" },
    { cmd: "\\cup", name: "Union", keywords: "U combine", trigger: "cup" },
    { cmd: "\\cap", name: "Intersection", keywords: "n overlap", trigger: "cap" },
    { cmd: "\\emptyset", name: "Empty Set", keywords: "null void", trigger: "empty" },
    { cmd: "\\therefore", name: "Therefore", keywords: "logic hence" },
    { cmd: "\\because", name: "Because", keywords: "logic since" },

    // --- ARROWS ---
    { cmd: "\\rightarrow", name: "Right Arrow", keywords: "-> next", trigger: "->" },
    { cmd: "\\leftarrow", name: "Left Arrow", keywords: "<- prev", trigger: "<-" },
    { cmd: "\\Rightarrow", name: "Right Double", keywords: "=> implies", trigger: "=>" },
    { cmd: "\\Leftarrow", name: "Left Double", keywords: "<= implies" },
    { cmd: "\\Leftrightarrow", name: "Equivalent Arrow", keywords: "<=> iff" },
    { cmd: "\\mapsto", name: "Maps To", keywords: "function" },
    { cmd: "\\uparrow", name: "Up Arrow", keywords: "up" },
    { cmd: "\\downarrow", name: "Down Arrow", keywords: "down" },

    // --- ACCENTS & DELIMITERS ---
    { cmd: "\\hat", name: "Hat", keywords: "unit vector" },
    { cmd: "\\bar", name: "Bar", keywords: "average mean" },
    { cmd: "\\vec", name: "Vector", keywords: "arrow" },
    { cmd: "\\dot", name: "Dot", keywords: "derivative time" },
    { cmd: "\\langle", name: "Left Angle", keywords: "< bracket" },
    { cmd: "\\rangle", name: "Right Angle", keywords: "> bracket" },

    // --- FUNCTIONS ---
    { cmd: "\\sin", name: "Sine", keywords: "trig", trigger: "sin" },
    { cmd: "\\cos", name: "Cosine", keywords: "trig", trigger: "cos" },
    { cmd: "\\tan", name: "Tangent", keywords: "trig", trigger: "tan" },
    { cmd: "\\ln", name: "Natural Log", keywords: "logarithm", trigger: "ln" },
    { cmd: "\\log", name: "Log", keywords: "logarithm", trigger: "log" },
];