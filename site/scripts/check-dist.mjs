// Post-build checks on dist/. Prints every failure, then exits 1 if any.
// Usage: node scripts/check-dist.mjs [dist-dir]   (default: dist)
import { readFile, readdir } from 'node:fs/promises';
import { join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

const BASE = '/nf-agents/';
const SITE_HOST = 'pingedbrain.github.io';
const DISCLAIMER = 'Not affiliated with or endorsed by Seqera';
const DENYLIST = [/fonts\.googleapis/, /fonts\.gstatic/, /googletagmanager/, /google-analytics/, /plausible/];
const VENDOR_LOGO = /nextflow|seqera|\baws\b|azure|\bgcp\b|google-cloud/i;
const DOCS_ROUTES = [
  'docs/index.html',
  'docs/architecture/index.html',
  'docs/nf-lineage-mcp/index.html',
  'docs/nf-llm-replay/index.html',
  'docs/roadmap/index.html',
];

function resolvesInDist(value, files) {
  const path = value.slice(BASE.length).split(/[?#]/)[0];
  const candidates = path === '' || path.endsWith('/')
    ? [`${path}index.html`]
    : [path, `${path}/index.html`];
  return candidates.some((c) => files.has(c));
}

function hostOf(url) {
  const m = /^https?:\/\/([^/\s"'?#]+)/i.exec(url);
  return m ? m[1].toLowerCase() : null;
}

export function checkHtml(relPath, html, files) {
  const errors = [];
  const h1Count = (html.match(/<h1[\s>]/gi) ?? []).length;
  if (h1Count !== 1) errors.push(`${relPath}: expected exactly one <h1>, found ${h1Count}`);

  if (!html.includes('data-site-footer')) errors.push(`${relPath}: missing data-site-footer`);
  if (!html.includes(DISCLAIMER)) errors.push(`${relPath}: footer lacks the disclaimer "${DISCLAIMER}"`);

  for (const m of html.matchAll(/\s(href|src|srcset)\s*=\s*(?:"([^"]*)"|'([^']*)')/gi)) {
    const attr = m[1].toLowerCase();
    const raw = m[2] ?? m[3] ?? '';
    const values = attr === 'srcset' ? raw.split(',').map((s) => s.trim().split(/\s+/)[0]) : [raw];
    for (const v of values) {
      if (!v.startsWith('/') || v.startsWith('//')) continue;
      if (!v.startsWith(BASE)) {
        errors.push(`${relPath}: root-relative ${attr} "${v}" must start with ${BASE}`);
      } else if (!resolvesInDist(v, files)) {
        errors.push(`${relPath}: ${attr} "${v}" does not resolve to a dist file`);
      }
    }
  }

  for (const m of html.matchAll(/<(script|link|img|source)\b[^>]*>/gi)) {
    for (const a of m[0].matchAll(/\s(src|href|srcset)\s*=\s*["']([^"']*)["']/gi)) {
      const host = hostOf(a[2].split(',')[0].trim().split(/\s+/)[0]);
      if (host && host !== SITE_HOST) errors.push(`${relPath}: <${m[1].toLowerCase()}> loads from external host ${host}`);
    }
    for (const a of m[0].matchAll(/\s(?:src|href)\s*=\s*["']([^"']*)["']/gi)) {
      if (VENDOR_LOGO.test(a[1])) errors.push(`${relPath}: vendor logo reference "${a[1]}" is not allowed`);
    }
  }
  for (const m of html.matchAll(/<img\b[^>]*\balt\s*=\s*["']([^"']*)["']/gi)) {
    if (VENDOR_LOGO.test(m[1])) errors.push(`${relPath}: vendor logo name in alt "${m[1]}" is not allowed`);
  }
  for (const m of html.matchAll(/url\(\s*['"]?(https?:\/\/[^)'"\s]+)/gi)) {
    const host = hostOf(m[1]);
    if (host !== SITE_HOST) errors.push(`${relPath}: url() loads from external host ${host}`);
  }
  for (const re of DENYLIST) {
    if (re.test(html)) errors.push(`${relPath}: references ${re.source}`);
  }
  return errors;
}

export function checkText(relPath, text) {
  const errors = [];
  for (const re of DENYLIST) {
    if (re.test(text)) errors.push(`${relPath}: references ${re.source}`);
  }
  for (const m of text.matchAll(/url\(\s*['"]?(https?:\/\/[^)'"\s]+)/gi)) {
    const host = hostOf(m[1]);
    if (host !== SITE_HOST) errors.push(`${relPath}: url() loads from external host ${host}`);
  }
  return errors;
}

export function checkRoutes(files) {
  return DOCS_ROUTES.filter((r) => !files.has(r)).map((r) => `missing docs route: ${r}`);
}

export function pngSize(buf) {
  const sig = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];
  if (buf.length < 24 || sig.some((b, i) => buf[i] !== b) || buf.toString('ascii', 12, 16) !== 'IHDR') {
    throw new Error('not a PNG file');
  }
  return { width: buf.readUInt32BE(16), height: buf.readUInt32BE(20) };
}

async function walk(root, prefix = '') {
  const out = [];
  for (const entry of await readdir(join(root, prefix), { withFileTypes: true })) {
    const rel = prefix ? `${prefix}/${entry.name}` : entry.name;
    if (entry.isDirectory()) out.push(...(await walk(root, rel)));
    else out.push(rel);
  }
  return out;
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  const root = resolve(process.argv[2] ?? 'dist');
  const list = await walk(root);
  const files = new Set(list);
  const errors = [...checkRoutes(files)];
  let htmlCount = 0;
  for (const rel of list) {
    if (/\.(html)$/.test(rel)) {
      htmlCount += 1;
      errors.push(...checkHtml(rel, await readFile(join(root, rel), 'utf8'), files));
    } else if (/\.(css|js|mjs)$/.test(rel)) {
      errors.push(...checkText(rel, await readFile(join(root, rel), 'utf8')));
    }
  }
  if (errors.length) {
    for (const e of errors) console.error(e);
    console.error(`${errors.length} check(s) failed`);
    process.exitCode = 1;
  } else {
    console.log(`dist OK: ${htmlCount} html pages checked`);
  }
}
