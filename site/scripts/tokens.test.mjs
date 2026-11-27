import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { contrastRatio, parseTokenSets } from './tokens.mjs';

const here = dirname(fileURLToPath(import.meta.url));
const css = await readFile(join(here, '..', 'src', 'styles', 'tokens.css'), 'utf8');
const sets = parseTokenSets(css);

// Text on background pairs that the landing page and docs actually use.
// --nf-faint is deliberately absent: it only colours the decorative "nf-" prefix.
const TEXT_PAIRS = [
  ['--nf-ink', '--nf-bg'],
  ['--nf-ink', '--nf-surface'],
  ['--nf-muted', '--nf-bg'],
  ['--nf-muted', '--nf-surface'],
  ['--nf-accent', '--nf-bg'],
  ['--nf-accent', '--nf-surface'],
  ['--nf-accent-strong', '--nf-bg'],
  ['--nf-accent-strong', '--nf-surface'],
];

test('contrastRatio matches WCAG reference values', () => {
  assert.ok(Math.abs(contrastRatio('#000000', '#FFFFFF') - 21) < 0.01);
  const grey = contrastRatio('#767676', '#FFFFFF');
  assert.ok(grey >= 4.5 && grey < 4.6, `got ${grey}`);
});

test('light, dark media and dark data-theme sets define the same tokens', () => {
  const keys = (s) => Object.keys(s).sort().join(',');
  assert.equal(keys(sets.darkMedia), keys(sets.light));
  assert.equal(keys(sets.darkAttr), keys(sets.light));
});

for (const setName of ['light', 'darkMedia', 'darkAttr']) {
  test(`text pairs meet 4.5:1 in the ${setName} set`, () => {
    const tokens = sets[setName];
    for (const [fg, bg] of TEXT_PAIRS) {
      assert.ok(tokens[fg] && tokens[bg], `${setName} lacks ${fg} or ${bg}`);
      const ratio = contrastRatio(tokens[fg], tokens[bg]);
      assert.ok(ratio >= 4.5, `${setName}: ${fg} on ${bg} is ${ratio.toFixed(2)}:1`);
    }
  });
}
