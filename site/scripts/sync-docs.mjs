// Copies the repository docs and brand assets into gitignored site paths.
// Runs before every build (prebuild). Pure helpers are exported for node:test.
import { readFile, writeFile, mkdir, copyFile, readdir, rm } from 'node:fs/promises';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const REPO = 'https://github.com/pingedbrain/baqueano/';

export const SOURCES = [
  { src: 'docs/demo.md', slug: '' },
  { src: 'docs/architecture.md', slug: 'architecture' },
  { src: 'crates/baqueano-mcp/README.md', slug: 'baqueano-mcp' },
  { src: 'docs/examples/baqueano-replay/README.md', slug: 'baqueano-replay' },
  { src: 'ROADMAP.md', slug: 'roadmap' },
];

const SCHEME = /^[a-z][a-z0-9+.-]*:/i;

function routeFor(slug, base) {
  return slug === '' ? `${base}/docs/` : `${base}/docs/${slug}/`;
}

// Resolve a link found in `src` (a repo-relative path). `..` is clipped at the
// repository root, so `../../issues` in crates/... points at the repo's issues page.
export function rewriteLink(target, src, base = '/baqueano') {
  if (target.startsWith('#') || SCHEME.test(target) || target.startsWith('//')) return target;
  const hashAt = target.indexOf('#');
  const hash = hashAt === -1 ? '' : target.slice(hashAt);
  const pathAndQuery = hashAt === -1 ? target : target.slice(0, hashAt);
  const resolved = new URL(pathAndQuery, REPO + src);
  const rel = resolved.pathname.replace(/^\/pingedbrain\/baqueano\//, '');
  const mapped = SOURCES.find((s) => s.src === rel);
  if (mapped && !resolved.search) return routeFor(mapped.slug, base) + hash;
  const isDir = pathAndQuery.endsWith('/');
  const isFile = /\.[A-Za-z0-9]+$/.test(rel);
  const prefix = isDir ? `${REPO}tree/main/` : isFile ? `${REPO}blob/main/` : REPO;
  return `${prefix}${rel}${resolved.search}${hash}`;
}

// Split on fenced code blocks; yield [text, isCode] pairs.
function rewriteBody(body, src, base) {
  const lines = body.split('\n');
  let inFence = false;
  return lines
    .map((line) => {
      if (/^\s*(```|~~~)/.test(line)) {
        inFence = !inFence;
        return line;
      }
      if (inFence) return line;
      return line
        .split(/(`[^`]*`)/)
        .map((part, i) =>
          i % 2 === 1
            ? part
            : part.replace(/\]\(([^)\s]+)((?:\s+"[^"]*")?)\)/g, (_, t, title) => `](${rewriteLink(t, src, base)}${title})`),
        )
        .join('');
    })
    .join('\n');
}

function assertNoRelativeImages(body, src) {
  const md = /!\[[^\]]*\]\(\s*([^)\s]+)/g;
  const html = /<img\b[^>]*\bsrc=["']([^"']+)["']/gi;
  for (const re of [md, html]) {
    for (const m of body.matchAll(re)) {
      if (!SCHEME.test(m[1]) && !m[1].startsWith('//')) {
        throw new Error(`${src}: relative image ${m[1]} is not supported; link to an absolute URL`);
      }
    }
  }
}

export function transform(md, { src, base = '/baqueano' }) {
  const lines = md.split('\n');
  const first = lines.findIndex((l) => l.trim() !== '');
  const h1 = first === -1 ? null : /^# (.+?)\s*$/.exec(lines[first]);
  if (!h1) throw new Error(`${src}: first non-empty line must be an H1 ("# Title")`);
  const title = h1[1];
  const body = lines.slice(first + 1).join('\n').replace(/^\n+/, '');
  assertNoRelativeImages(body, src);
  const rewritten = rewriteBody(body, src, base);
  const editUrl = `${REPO}edit/main/${src}`;
  return `---\ntitle: ${JSON.stringify(title)}\neditUrl: ${JSON.stringify(editUrl)}\n---\n\n${rewritten}`;
}

export async function sync({ repoRoot, siteRoot, base = '/baqueano' }) {
  const docsDir = join(siteRoot, 'src/content/docs/docs');
  const brandDir = join(siteRoot, 'src/generated/brand');
  await rm(docsDir, { recursive: true, force: true });
  await rm(brandDir, { recursive: true, force: true });
  await mkdir(docsDir, { recursive: true });
  for (const { src, slug } of SOURCES) {
    let md;
    try {
      md = await readFile(join(repoRoot, src), 'utf8');
    } catch {
      throw new Error(`missing source: ${src}`);
    }
    const out = transform(md, { src, base });
    await writeFile(join(docsDir, `${slug || 'index'}.md`), out);
  }
  const logoDir = join(repoRoot, 'brand/logo');
  await mkdir(brandDir, { recursive: true });
  for (const f of (await readdir(logoDir)).filter((n) => n.endsWith('.svg'))) {
    await copyFile(join(logoDir, f), join(brandDir, f));
  }
  await mkdir(join(siteRoot, 'public'), { recursive: true });
  await copyFile(join(logoDir, 'favicon.svg'), join(siteRoot, 'public/favicon.svg'));
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  const siteRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
  await sync({ repoRoot: resolve(siteRoot, '..'), siteRoot });
}
