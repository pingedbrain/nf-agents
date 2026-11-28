import { test } from 'node:test';
import assert from 'node:assert/strict';
import { checkCopy, checkHtml, checkOg, checkOgMeta, checkRoutes, pngSize } from './check-dist.mjs';

const FOOTER =
  '<footer data-site-footer><p>Not affiliated with or endorsed by Seqera. Nextflow is a registered trademark of Seqera Labs.</p></footer>';
const page = (body) => `<!doctype html><html><head><title>t</title></head><body>${body}${FOOTER}</body></html>`;
const files = new Set(['index.html', 'docs/index.html', 'favicon.svg']);

test('a clean page has no errors', () => {
  const html = page('<h1>baqueano</h1><a href="/baqueano/docs/">docs</a><img src="/baqueano/favicon.svg" alt="">');
  assert.deepEqual(checkHtml('index.html', html, files), []);
});

test('more than one h1 is reported', () => {
  const errors = checkHtml('index.html', page('<h1>a</h1><h1>b</h1>'), files);
  assert.ok(errors.some((e) => /h1/.test(e)), errors.join('\n'));
});

test('missing footer marker or disclaimer is reported', () => {
  const noMarker = '<!doctype html><html><body><h1>a</h1></body></html>';
  assert.ok(checkHtml('index.html', noMarker, files).some((e) => /footer/.test(e)));
  const noText = '<!doctype html><html><body><h1>a</h1><footer data-site-footer>x</footer></body></html>';
  assert.ok(checkHtml('index.html', noText, files).some((e) => /Seqera/.test(e)));
});

test('root-relative links without the /baqueano/ base are reported', () => {
  const errors = checkHtml('index.html', page('<h1>a</h1><a href="/docs/">d</a>'), files);
  assert.ok(errors.some((e) => /\/baqueano\//.test(e) && /\/docs\//.test(e)), errors.join('\n'));
});

test('a /baqueano/ link that resolves to no dist file is reported', () => {
  const errors = checkHtml('index.html', page('<h1>a</h1><a href="/baqueano/missing/">m</a>'), files);
  assert.ok(errors.some((e) => /missing/.test(e)), errors.join('\n'));
});

test('external script, stylesheet and image hosts are reported', () => {
  const html = page(
    '<h1>a</h1><script src="https://cdn.example.com/a.js"></script>' +
      '<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Inter">' +
      '<img src="https://example.org/i.png" alt="">',
  );
  const errors = checkHtml('index.html', html, files);
  assert.ok(errors.some((e) => /cdn\.example\.com/.test(e)), errors.join('\n'));
  assert.ok(errors.some((e) => /fonts\.googleapis/.test(e)), errors.join('\n'));
  assert.ok(errors.some((e) => /example\.org/.test(e)), errors.join('\n'));
});

test('external url() in inline styles is reported', () => {
  const html = page('<h1>a</h1><div style="background:url(https://example.net/x.png)"></div>');
  assert.ok(checkHtml('index.html', html, files).some((e) => /example\.net/.test(e)));
});

test('vendor logo assets are reported', () => {
  const html = page('<h1>a</h1><img src="/baqueano/nextflow-logo.svg" alt="">');
  assert.ok(checkHtml('index.html', html, files).some((e) => /nextflow/i.test(e)));
});

test('checkRoutes names each missing docs route', () => {
  const complete = new Set([
    'docs/index.html',
    'docs/architecture/index.html',
    'docs/baqueano-mcp/index.html',
    'docs/baqueano-replay/index.html',
    'docs/roadmap/index.html',
  ]);
  assert.deepEqual(checkRoutes(complete), []);
  const errors = checkRoutes(new Set(['docs/index.html']));
  assert.ok(errors.some((e) => /docs\/roadmap/.test(e)), errors.join('\n'));
});

test('pngSize reads width and height from the IHDR chunk', () => {
  const buf = Buffer.alloc(24);
  Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]).copy(buf, 0);
  buf.write('IHDR', 12, 'ascii');
  buf.writeUInt32BE(1200, 16);
  buf.writeUInt32BE(630, 20);
  assert.deepEqual(pngSize(buf), { width: 1200, height: 630 });
  assert.throws(() => pngSize(Buffer.from('not a png at all, sorry......')));
});

const OG_URL = 'https://pingedbrain.github.io/baqueano/og.png';
const pngOf = (width, height) => {
  const buf = Buffer.alloc(24);
  Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]).copy(buf, 0);
  buf.write('IHDR', 12, 'ascii');
  buf.writeUInt32BE(width, 16);
  buf.writeUInt32BE(height, 20);
  return buf;
};

test('og:image meta with the /baqueano/ base passes', () => {
  const html = page(`<h1>a</h1><meta property="og:image" content="${OG_URL}">`);
  assert.deepEqual(checkOgMeta('index.html', html), []);
});

test('missing og:image meta is reported', () => {
  const errors = checkOgMeta('index.html', page('<h1>a</h1>'));
  assert.ok(errors.some((e) => /og:image/.test(e)), errors.join('\n'));
});

test('og:image without the /baqueano/ base is reported', () => {
  const html = page('<h1>a</h1><meta property="og:image" content="https://pingedbrain.github.io/og.png">');
  assert.ok(checkOgMeta('index.html', html).some((e) => /og:image/.test(e)));
});

test('OG image must exist and be 1200x630', () => {
  assert.ok(checkOg(new Set(['index.html']), undefined).some((e) => /og\.png/.test(e)));
  assert.ok(checkOg(new Set(['og.png']), pngOf(800, 600)).some((e) => /1200x630/.test(e)));
  assert.deepEqual(checkOg(new Set(['og.png']), pngOf(1200, 630)), []);
});

test('copy: the old project name and the docker one-liner are reported on any page', () => {
  for (const bad of ['nf-agents', 'NF-Agents', 'Nf-agents', 'docker run ghcr.io/x/baqueano-demo']) {
    assert.ok(checkCopy('docs/index.html', page(`<p>${bad}</p>`)).length > 0, bad);
  }
  assert.deepEqual(checkCopy('docs/index.html', page('<p>nf-agents-fixture</p>')), []);
  assert.deepEqual(checkCopy('docs/roadmap/index.html', page('<p>nf-core/sarek on AWS Batch</p>')), []);
});

test('copy: the landing needs the real demo and no sarek example', () => {
  const ok = page('<pre>git clone x &amp;&amp; demo/run.sh</pre><pre>params.greeting</pre>');
  assert.deepEqual(checkCopy('index.html', ok), []);
  assert.ok(checkCopy('index.html', page('<pre>demo/run.sh params.greeting sarek</pre>')).some((e) => /sarek/.test(e)));
  assert.ok(checkCopy('index.html', page('<p>hi</p>')).some((e) => /demo\/run\.sh/.test(e)));
});
