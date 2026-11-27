// Renders og/og.svg to public/og.png (1200x630). Run via `npm run og`, which installs
// @resvg/resvg-js and wawoff2 with --no-save first, so neither enters the lockfile.
// resvg cannot read WOFF2, so the fontsource WOFF2 files are decoded to temporary TTFs.
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { Resvg } from '@resvg/resvg-js';
import { decompress } from 'wawoff2';

const siteRoot = resolve(import.meta.dirname, '..');
const FONTS = [
  '@fontsource-variable/inter/files/inter-latin-wght-normal.woff2',
  '@fontsource-variable/jetbrains-mono/files/jetbrains-mono-latin-wght-normal.woff2',
];

const svg = await readFile(join(siteRoot, 'og/og.svg'), 'utf8');
const tmp = await mkdtemp(join(tmpdir(), 'og-'));
try {
  const fontFiles = [];
  for (const [i, rel] of FONTS.entries()) {
    const ttf = await decompress(await readFile(join(siteRoot, 'node_modules', rel)));
    const path = join(tmp, `font-${i}.ttf`);
    await writeFile(path, ttf);
    fontFiles.push(path);
  }
  const png = new Resvg(svg, {
    fitTo: { mode: 'width', value: 1200 },
    font: { fontFiles, loadSystemFonts: false, defaultFontFamily: 'Inter' },
  }).render().asPng();
  await writeFile(join(siteRoot, 'public/og.png'), png);
  console.log(`wrote public/og.png (${png.length} bytes)`);
} finally {
  await rm(tmp, { recursive: true, force: true });
}
