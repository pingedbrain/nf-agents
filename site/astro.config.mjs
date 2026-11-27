// @ts-check
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

export default defineConfig({
  site: 'https://pingedbrain.github.io',
  base: '/nf-agents',
  trailingSlash: 'always',
  integrations: [
    starlight({
      title: 'nf-agents docs',
      description: 'Documentation for nf-agents: agent modules and tools for Nextflow pipelines.',
      customCss: ['./src/styles/tokens.css', './src/styles/starlight.css'],
      head: [
        { tag: 'meta', attrs: { property: 'og:image', content: 'https://pingedbrain.github.io/nf-agents/og.png' } },
        { tag: 'meta', attrs: { name: 'twitter:card', content: 'summary_large_image' } },
      ],
      components: {
        Footer: './src/components/StarlightFooter.astro',
      },
      sidebar: [
        { label: 'Getting started', slug: 'docs' },
        { label: 'Architecture', slug: 'docs/architecture' },
        { label: 'nf-lineage-mcp', slug: 'docs/nf-lineage-mcp' },
        { label: 'nf-llm-replay', slug: 'docs/nf-llm-replay' },
        { label: 'Roadmap', slug: 'docs/roadmap' },
      ],
    }),
  ],
});
