import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, mkdir, writeFile, readFile, access } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { transform, rewriteLink, sync } from './sync-docs.mjs';

const BASE = '/baqueano';

test('first H1 becomes the frontmatter title and is removed from the body', () => {
  const out = transform('# Local demo\n\nBody text.\n', { src: 'docs/demo.md', base: BASE });
  assert.match(out, /^---\ntitle: "Local demo"\n/);
  assert.match(out, /editUrl: "https:\/\/github\.com\/pingedbrain\/baqueano\/edit\/main\/docs\/demo\.md"/);
  assert.equal((out.match(/^# /gm) ?? []).length, 0);
  assert.match(out, /Body text\./);
});

test('missing H1 throws and names the file', () => {
  assert.throws(
    () => transform('Intro first\n\n# Later\n', { src: 'docs/architecture.md', base: BASE }),
    /docs\/architecture\.md/,
  );
});

test('links to mapped docs resolve to site routes, keeping anchors', () => {
  assert.equal(rewriteLink('architecture.md', 'docs/demo.md', BASE), '/baqueano/docs/architecture/');
  assert.equal(rewriteLink('../../ROADMAP.md', 'crates/baqueano-mcp/README.md', BASE), '/baqueano/docs/roadmap/');
  assert.equal(rewriteLink('../../docs/demo.md', 'crates/baqueano-mcp/README.md', BASE), '/baqueano/docs/');
  assert.equal(
    rewriteLink('architecture.md#overview', 'docs/demo.md', BASE),
    '/baqueano/docs/architecture/#overview',
  );
});

test('links to other repo files become GitHub blob URLs, directories become tree URLs', () => {
  assert.equal(
    rewriteLink('../crates/baqueano-lineage/README.md#fixtures', 'docs/demo.md', BASE),
    'https://github.com/pingedbrain/baqueano/blob/main/crates/baqueano-lineage/README.md#fixtures',
  );
  assert.equal(
    rewriteLink('../crates/', 'docs/demo.md', BASE),
    'https://github.com/pingedbrain/baqueano/tree/main/crates/',
  );
});

test('parent-relative issue links resolve to the repository root', () => {
  assert.equal(
    rewriteLink('../../issues?q=is%3Aopen', 'crates/baqueano-mcp/README.md', BASE),
    'https://github.com/pingedbrain/baqueano/issues?q=is%3Aopen',
  );
});

test('absolute URLs, fragments and mailto links are kept', () => {
  assert.equal(rewriteLink('https://example.com/a', 'docs/demo.md', BASE), 'https://example.com/a');
  assert.equal(rewriteLink('#setup', 'docs/demo.md', BASE), '#setup');
  assert.equal(rewriteLink('mailto:a@example.com', 'docs/demo.md', BASE), 'mailto:a@example.com');
});

test('code fences and inline code are left untouched', () => {
  const md = '# T\n\n`[x](docs/demo.md)`\n\n```md\n[y](docs/demo.md)\n```\n\n[z](docs/demo.md)\n';
  const out = transform(md, { src: 'ROADMAP.md', base: BASE });
  assert.match(out, /`\[x\]\(docs\/demo\.md\)`/);
  assert.match(out, /```md\n\[y\]\(docs\/demo\.md\)\n```/);
  assert.match(out, /\[z\]\(\/baqueano\/docs\/\)/);
});

test('relative images throw, external images are allowed', () => {
  assert.throws(
    () => transform('# T\n\n![logo](logo.png)\n', { src: 'docs/demo.md', base: BASE }),
    /docs\/demo\.md.*relative image/,
  );
  assert.throws(
    () => transform('# T\n\n<img src="a.svg" alt="">\n', { src: 'docs/demo.md', base: BASE }),
    /relative image/,
  );
  assert.doesNotThrow(() => transform('# T\n\n![x](https://example.com/x.png)\n', { src: 'docs/demo.md', base: BASE }));
});

test('sync fails naming a missing source', async () => {
  const repoRoot = await mkdtemp(join(tmpdir(), 'nf-site-repo-'));
  const siteRoot = await mkdtemp(join(tmpdir(), 'nf-site-out-'));
  await assert.rejects(sync({ repoRoot, siteRoot }), /missing source: docs\/demo\.md/);
});

test('sync writes docs pages and copies brand assets', async () => {
  const repoRoot = await mkdtemp(join(tmpdir(), 'nf-site-repo-'));
  const siteRoot = await mkdtemp(join(tmpdir(), 'nf-site-out-'));
  const files = {
    'docs/demo.md': '# Local demo\n\nRun it.\n',
    'docs/architecture.md': '# Architecture\n',
    'crates/baqueano-mcp/README.md': '# baqueano-mcp\n',
    'docs/examples/baqueano-replay/README.md': '# baqueano-replay\n',
    'ROADMAP.md': '# Roadmap\n',
    'brand/logo/favicon.svg': '<svg/>',
    'brand/logo/mark-light.svg': '<svg id="l"/>',
  };
  for (const [rel, text] of Object.entries(files)) {
    await mkdir(join(repoRoot, rel, '..'), { recursive: true });
    await writeFile(join(repoRoot, rel), text);
  }
  await sync({ repoRoot, siteRoot });
  const index = await readFile(join(siteRoot, 'src/content/docs/docs/index.md'), 'utf8');
  assert.match(index, /title: "Local demo"/);
  await readFile(join(siteRoot, 'src/content/docs/docs/roadmap.md'), 'utf8');
  await readFile(join(siteRoot, 'src/generated/brand/mark-light.svg'), 'utf8');
  await access(join(siteRoot, 'public/favicon.svg'));
});
