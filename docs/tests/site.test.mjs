import assert from 'node:assert/strict'
import { access, readFile, readdir } from 'node:fs/promises'
import path from 'node:path'
import { after, before, test } from 'node:test'
import { fileURLToPath } from 'node:url'
import puppeteer from 'puppeteer-core'
import { preview } from 'vite'

const root = fileURLToPath(new URL('../', import.meta.url))
const studies = process.env.DOCS_TEST_THEMES === '1'
const outDir = path.join(root, '.vitepress', studies ? 'studies' : 'dist')
let server
let browser
let page
let base
const browserErrors = []

async function findBrowser() {
  const candidates = [
    process.env.CHROME_PATH,
    process.env.CHROME_BIN,
    process.env.ProgramFiles && path.join(process.env.ProgramFiles, 'Google', 'Chrome', 'Application', 'chrome.exe'),
    process.env['ProgramFiles(x86)'] && path.join(process.env['ProgramFiles(x86)'], 'Microsoft', 'Edge', 'Application', 'msedge.exe'),
    '/usr/bin/google-chrome',
    '/usr/bin/chromium',
    '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',
  ].filter(Boolean)
  for (const candidate of candidates) {
    try {
      await access(candidate)
      return candidate
    } catch (error) {
      if (error.code !== 'ENOENT') throw error
    }
  }
  throw new Error('Install Chrome or Edge, or set CHROME_PATH to its executable. Site checks cannot run without a browser.')
}

before(async () => {
  const executablePath = await findBrowser()
  await access(path.join(outDir, 'index.html'))
  server = await preview({
    configFile: false,
    root,
    base: '/beyond-gate/',
    build: { outDir },
    preview: { host: '127.0.0.1', port: 0, strictPort: true },
  })
  base = `http://127.0.0.1:${server.httpServer.address().port}/beyond-gate/`
  browser = await puppeteer.launch({ executablePath, headless: true })
  page = await browser.newPage()
  await page.setCacheEnabled(false)
  page.on('pageerror', error => browserErrors.push(error.message))
  page.on('response', response => {
    if (response.status() >= 400 && response.url().startsWith(base)) {
      browserErrors.push(`${response.status()} ${response.url()}`)
    }
  })
})

after(async () => {
  await browser?.close()
  if (server) {
    await new Promise((resolve, reject) => server.httpServer.close(error => error ? reject(error) : resolve()))
  }
})

test('overview and long references render below the deployment subpath', async () => {
  for (const width of [1440, 390]) {
    await page.setViewport({ width, height: 1000 })
    for (const route of ['', 'installation.html', 'authentication.html', 'cli.html', 'mcp.html', 'support.html', 'privacy.html']) {
      await page.goto(`${base}${route}`, { waitUntil: 'networkidle0' })
      assert.equal(await page.$eval('h1', heading => Boolean(heading.textContent.trim())), true)
      assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true, `${route} overflows at ${width}px`)
      assert.deepEqual(await page.$$eval('img', images => images.filter(image => !image.complete || !image.naturalWidth).map(image => image.src)), [])
      assert.equal(Boolean(await page.$('.theme-study')), studies, 'Comparison controls must not ship in production')
      if (route === '') {
        assert.doesNotMatch(await page.$eval('.overview', overview => overview.textContent), /Know the limits|Read current command status/)
      }
      if (width === 390 && route === '') {
        const layout = await page.evaluate(() => ({
          copyBottom: document.querySelector('.entrance-copy').getBoundingClientRect().bottom,
          gateTop: document.querySelector('.gate-art').getBoundingClientRect().top,
          gateWidth: document.querySelector('.gate-frame').getBoundingClientRect().width,
        }))
        assert.ok(layout.gateTop >= layout.copyBottom, 'The mobile gate must follow the copy, not squeeze it into a column')
        assert.ok(layout.gateWidth >= 96, 'The mobile gate mark must remain legible')
      }
    }
  }
})

test('policy pages are reachable from the overview and user sidebar', async () => {
  await page.setViewport({ width: 1440, height: 1000 })
  for (const route of ['support', 'privacy']) {
    await page.goto(base, { waitUntil: 'networkidle0' })
    await page.click(`.overview a[href$="/${route}"]`)
    await page.waitForFunction(target => location.pathname.replace(/\.html$/, '') === `/beyond-gate/${target}`, {}, route)
    const group = await page.$eval(`.VPSidebar a[href$="/${route}.html"]`, link => link.closest('.group').textContent)
    assert.match(group, /Use Beyond Gate/)
    assert.doesNotMatch(group, /Development/)
  }
})

test('authentication setup stays separate from development design', async () => {
  await page.setViewport({ width: 1440, height: 1000 })
  await page.goto(`${base}authentication.html`, { waitUntil: 'networkidle0' })
  const guideGroup = await page.$eval('.VPSidebar a[href$="/authentication.html"]', link => link.closest('.group').textContent)
  assert.match(guideGroup, /Start here/)
  assert.doesNotMatch(guideGroup, /Authentication design/)
  const designGroup = await page.$eval('.VPSidebar a[href$="/authentication-design.html"]', link => link.closest('.group').textContent)
  assert.match(designGroup, /Development/)
  await page.click('.VPSidebar a[href$="/authentication-design.html"]')
  await page.waitForFunction(() => document.querySelector('.vp-doc h1')?.textContent.includes('Authentication design'))
  assert.equal(new URL(page.url()).pathname, '/beyond-gate/authentication-design.html')
})

test('all generated internal page links and fragments resolve', async () => {
  const filenames = (await readdir(outDir)).filter(name => name.endsWith('.html') && name !== '404.html')
  const documents = new Map(await Promise.all(filenames.map(async name => [name, await readFile(path.join(outDir, name), 'utf8')])))
  for (const filename of filenames) {
    const response = await page.goto(`${base}${filename}`, { waitUntil: 'networkidle0' })
    assert.equal(response.status(), 200, filename)
    const links = await page.$$eval('a[href]', links => links.map(link => link.href))
    for (const href of new Set(links)) {
      if (!href.startsWith(base)) continue
      const url = new URL(href)
      let target = url.pathname.slice('/beyond-gate/'.length) || 'index.html'
      if (!target.endsWith('.html')) target += '.html'
      const html = documents.get(target)
      assert.ok(html, `${filename}: missing page ${target}`)
      if (url.hash) assert.ok(html.includes(`id="${decodeURIComponent(url.hash.slice(1))}"`), `${filename}: missing fragment ${href}`)
    }
  }
})

test('local search supports keyboard opening, results, no results, and focus return', async () => {
  await page.setViewport({ width: 1440, height: 1000 })
  await page.goto(`${base}cli.html`, { waitUntil: 'networkidle0' })
  await page.focus('.VPNavBarSearch button')
  await page.keyboard.press('Enter')
  await page.waitForSelector('.VPLocalSearchBox input')
  await page.type('.VPLocalSearchBox input', 'auth validate')
  await page.waitForFunction(() => [...document.querySelectorAll('.VPLocalSearchBox a')].some(link => link.textContent.includes('auth validate')))
  await page.keyboard.press('Escape')
  await page.waitForFunction(() => Boolean(document.activeElement?.closest('.VPNavBarSearch')))
  await page.keyboard.press('Enter')
  await page.waitForSelector('.VPLocalSearchBox input')
  await page.keyboard.down('Control')
  await page.keyboard.press('a')
  await page.keyboard.up('Control')
  await page.type('.VPLocalSearchBox input', 'zzzxxyynotaword')
  await page.waitForFunction(() => document.querySelector('.VPLocalSearchBox')?.textContent.includes('No results'))
  await page.keyboard.press('Escape')
})

test('mobile navigation and sidebar remain keyboard accessible', async () => {
  await page.setViewport({ width: 390, height: 844 })
  await page.goto(`${base}cli.html`, { waitUntil: 'networkidle0' })
  await page.focus('.VPNavBarHamburger')
  await page.keyboard.press('Enter')
  await page.waitForSelector('.VPNavScreen', { visible: true })
  await page.click('.VPNavBarHamburger')
  await page.waitForFunction(() => !document.querySelector('.VPNavScreen'))
  await page.focus('.VPLocalNav .menu')
  await page.keyboard.press('Enter')
  await page.waitForSelector('.VPSidebar.open', { visible: true })
  await page.keyboard.press('Escape')
  await page.waitForFunction(() => !document.querySelector('.VPSidebar.open'))
})

test('production includes only the selected world; comparison retains all five studies', async () => {
  await page.setViewport({ width: 1440, height: 1000 })
  await page.goto(base, { waitUntil: 'networkidle0' })
  if (studies) {
    for (const id of ['gatehouse', 'folio', 'atlas', 'ember', 'sigil']) {
      await page.click(`.study-options button:has(.${id})`)
      await page.waitForFunction(theme => document.documentElement.dataset.study === theme, {}, id)
      await page.click('.study-heading a')
      await page.waitForFunction(theme => location.search === `?theme=${theme}`, {}, id)
      assert.equal(await page.evaluate(() => document.documentElement.dataset.study), id)
      await page.goto(`${base}?theme=${id}`, { waitUntil: 'networkidle0' })
    }
  } else {
    const files = await readdir(path.join(outDir, 'assets', 'chunks'))
    assert.equal(files.some(name => /ThemeStudy|studies/i.test(name)), false)
    const styles = (await readdir(path.join(outDir, 'assets'))).filter(name => name.endsWith('.css'))
    for (const name of styles) {
      assert.equal((await readFile(path.join(outDir, 'assets', name), 'utf8')).includes('.theme-study'), false)
    }
    await page.click('.VPSwitchAppearance')
    await page.waitForFunction(() => !document.documentElement.classList.contains('dark'))
    await page.reload({ waitUntil: 'networkidle0' })
    assert.equal(await page.evaluate(() => document.documentElement.classList.contains('dark')), false)
  }
  assert.deepEqual(browserErrors, [])
})
