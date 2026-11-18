import { fileURLToPath } from 'node:url'

process.env.BEYOND_GATE_THEME_STUDIES = '1'
const { build } = await import('vitepress')
await build(fileURLToPath(new URL('../', import.meta.url)))
