import { defineConfig } from 'astro/config';
import mdx from '@astrojs/mdx';
import tailwind from '@astrojs/tailwind';

import { remarkDetails } from './plugins/remark-details.mjs';
import { remarkYouTube } from './plugins/remark-youtube.mjs';
import { rehypeTerminal } from './plugins/rehype-terminal.mjs';

// https://astro.build/config
export default defineConfig({
  site: 'https://nvme-git.github.io',
  base: '/rocky/',
  trailingSlash: 'always',
  outDir: './dist',
  integrations: [
    mdx(),
    tailwind({ applyBaseStyles: false }),
  ],
  markdown: {
    syntaxHighlight: 'shiki',
    shikiConfig: {
      theme: 'github-dark',
      wrap: true,
    },
    remarkPlugins: [remarkDetails, remarkYouTube],
    rehypePlugins: [rehypeTerminal],
  },
  vite: {
    server: { fs: { allow: ['..'] } },
  },
});
