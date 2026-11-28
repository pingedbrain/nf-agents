// @ts-check
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

export default defineConfig({
  site: 'https://pingedbrain.github.io',
  base: '/baqueano',
  trailingSlash: 'always',
  integrations: [
    starlight({
      title: 'baqueano docs',
      description: 'Documentation for baqueano: agent modules and tools for Nextflow pipelines.',
      customCss: ['./src/styles/tokens.css', './src/styles/starlight.css'],
      head: [
        { tag: 'meta', attrs: { property: 'og:image', content: 'https://pingedbrain.github.io/baqueano/og.png' } },
        { tag: 'meta', attrs: { name: 'twitter:card', content: 'summary_large_image' } },
      ],
      components: {
        Footer: './src/components/StarlightFooter.astro',
      },
      sidebar: [
        { label: 'Getting started', slug: 'docs' },
        { label: 'Architecture', slug: 'docs/architecture' },
        { label: 'baqueano-mcp', slug: 'docs/baqueano-mcp' },
        { label: 'baqueano-replay', slug: 'docs/baqueano-replay' },
        { label: 'Roadmap', slug: 'docs/roadmap' },
      ],
    }),
  ],
});
