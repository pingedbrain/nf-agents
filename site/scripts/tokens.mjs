// Parse src/styles/tokens.css into its three colour sets and compute WCAG contrast.

const BLOCKS = {
  light: /:root\s*\{([^}]*)\}/,
  darkMedia: /:root:not\(\[data-theme='light'\]\)\s*\{([^}]*)\}/,
  darkAttr: /:root\[data-theme='dark'\]\s*\{([^}]*)\}/,
};

function vars(body) {
  const out = {};
  for (const m of body.matchAll(/--([\w-]+)\s*:\s*(#[0-9a-fA-F]{6})\s*;/g)) {
    out[`--${m[1]}`] = m[2].toUpperCase();
  }
  return out;
}

export function parseTokenSets(css) {
  const sets = {};
  for (const [name, re] of Object.entries(BLOCKS)) {
    const m = re.exec(css);
    if (!m) throw new Error(`tokens.css: no block for ${name}`);
    sets[name] = vars(m[1]);
  }
  return sets;
}

function luminance(hex) {
  const channel = (i) => {
    const c = parseInt(hex.slice(i, i + 2), 16) / 255;
    return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * channel(1) + 0.7152 * channel(3) + 0.0722 * channel(5);
}

export function contrastRatio(a, b) {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (hi + 0.05) / (lo + 0.05);
}
