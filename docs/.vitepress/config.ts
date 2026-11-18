import { defineConfig } from 'vitepress'
import { slug } from 'github-slugger'

const repository = 'https://github.com/PlagueHO/beyond-gate'
const themeStudies = process.env.BEYOND_GATE_THEME_STUDIES === '1'

export default defineConfig({
  title: 'Beyond Gate',
  description: 'Native CLI and local MCP documentation for Beyond Gate.',
  base: '/beyond-gate/',
  appearance: themeStudies ? false : 'dark',
  outDir: themeStudies ? '.vitepress/studies' : '.vitepress/dist',
  vite: { define: { __DOCS_THEME_STUDIES__: JSON.stringify(themeStudies) } },
  lastUpdated: false,
  markdown: {
    anchor: { slugify: slug },
    headers: { slugify: slug },
    toc: { slugify: slug },
    config(markdown) {
      const renderLink = markdown.renderer.rules.link_open
      markdown.renderer.rules.link_open = (tokens, index, options, environment, renderer) => {
        const token = tokens[index]
        const href = token.attrGet('href')
        // These links target repository files outside the documentation site.
        if (href?.startsWith('../')) {
          token.attrSet('href', `${repository}/blob/main/${href.slice(3)}`)
        } else if (href?.startsWith('fixtures/')) {
          token.attrSet('href', `${repository}/blob/main/docs/${href}`)
        }
        return renderLink
          ? renderLink(tokens, index, options, environment, renderer)
          : renderer.renderToken(tokens, index, options)
      }
    },
  },
  themeConfig: {
    nav: [
      { text: 'Overview', link: '/' },
      { text: 'Installation', link: '/installation' },
      { text: 'CLI', link: '/cli' },
      { text: 'MCP', link: '/mcp' },
      { text: 'Development', link: '/development' },
    ],
    sidebar: [
      {
        text: 'Start here',
        items: [
          { text: 'Overview', link: '/' },
          { text: 'Installation', link: '/installation' },
          { text: 'Authentication', link: '/authentication' },
        ],
      },
      {
        text: 'Use Beyond Gate',
        items: [
          { text: 'CLI reference', link: '/cli' },
          { text: 'MCP reference', link: '/mcp' },
        ],
      },
      {
        text: 'Development',
        collapsed: false,
        items: [
          { text: 'Developer guide', link: '/development' },
          { text: 'Architecture', link: '/architecture' },
          { text: 'Testing', link: '/testing' },
          { text: 'Roadmap', link: '/roadmap' },
          { text: 'Browser login contracts', link: '/browser-handoff' },
          { text: 'Desktop acceptance', link: '/desktop-acceptance' },
          { text: 'API call catalog', link: '/api-catalog' },
          { text: 'API survey', link: '/api-survey' },
        ],
      },
    ],
    search: { provider: 'local' },
    outline: { level: [2, 3], label: 'On this page' },
    socialLinks: [{ icon: 'github', link: repository }],
    docFooter: { prev: 'Previous page', next: 'Next page' },
    footer: { message: 'Unofficial. Local-first. Read-only.' },
  },
})
