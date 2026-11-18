import type { Theme } from 'vitepress'
import DefaultTheme from 'vitepress/theme'
import DocsOverview from './DocsOverview.vue'
import SiteLayout from './SiteLayout.vue'
import './theme.css'

if (__DOCS_THEME_STUDIES__) import('./studies.css')

export default {
  extends: DefaultTheme,
  Layout: SiteLayout,
  enhanceApp({ app }) {
    app.component('DocsOverview', DocsOverview)
  },
} satisfies Theme
