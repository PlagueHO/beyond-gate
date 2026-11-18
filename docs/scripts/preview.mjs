import { fileURLToPath } from 'node:url'
import { preview } from 'vite'

const studies = process.argv.includes('--themes')
const port = Number(process.env.DOCS_PREVIEW_PORT ?? 4173)
if (!Number.isInteger(port) || port < 1 || port > 65535) {
  throw new Error('DOCS_PREVIEW_PORT must be an integer from 1 to 65535.')
}

const server = await preview({
  configFile: false,
  root: fileURLToPath(new URL('../', import.meta.url)),
  base: '/beyond-gate/',
  build: { outDir: studies ? '.vitepress/studies' : '.vitepress/dist' },
  preview: { host: '127.0.0.1', port, strictPort: true },
})
server.printUrls()
