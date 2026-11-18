import { execFileSync } from 'node:child_process'
import { resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

/**
 * Read GitHub metadata without changing repository or Pages settings.
 * API failures and invalid JSON stop deployment.
 */
function githubJson(endpoint) {
  try {
    return JSON.parse(execFileSync('gh', ['api', endpoint], {
      encoding: 'utf8',
      timeout: 30_000,
      stdio: ['ignore', 'pipe', 'pipe'],
    }))
  } catch (error) {
    throw new Error(`Cannot read ${endpoint}: ${error.message}`, { cause: error })
  }
}

/**
 * Require a trusted event, the configured site, and the current default-branch SHA.
 * The injected reader lets offline tests exercise failures without GitHub access.
 */
export function checkPagesDeployment(context, readJson = githubJson) {
  const { eventName, ref, sha, repository } = context
  if (repository !== 'PlagueHO/beyond-gate' ||
      !['push', 'workflow_dispatch'].includes(eventName) ||
      !/^[0-9a-f]{40}$/.test(sha ?? '')) {
    throw new Error('Pages deployment requires a push or manual run in PlagueHO/beyond-gate with a valid commit SHA.')
  }

  const { default_branch: branch } = readJson(`repos/${repository}`)
  if (typeof branch !== 'string' || !branch || ref !== `refs/heads/${branch}`) {
    throw new Error('Pages deployment requires the current default branch. Pull requests, tags, and feature branches cannot deploy.')
  }

  const pages = readJson(`repos/${repository}/pages`)
  const configured = new URL(pages.html_url)
  if (pages.build_type !== 'workflow' ||
      !['http:', 'https:'].includes(configured.protocol) ||
      configured.host !== 'danielscottraynsford.com' ||
      configured.pathname !== '/beyond-gate/' ||
      configured.username || configured.password || configured.search || configured.hash) {
    throw new Error('Unexpected Pages configuration. Expected workflow publishing at danielscottraynsford.com/beyond-gate/. No settings were changed.')
  }

  // Read the branch last, after environment approval and deployment concurrency waiting.
  const latest = readJson(`repos/${repository}/git/ref/heads/${encodeURIComponent(branch)}`)
  if (latest.ref !== ref || latest.object?.type !== 'commit' || latest.object.sha !== sha) {
    throw new Error('Outdated Pages run. Build the current default-branch commit; use a revert commit for recovery.')
  }

  return {
    configuredUrl: configured.href,
    httpsUrl: 'https://danielscottraynsford.com/beyond-gate/',
  }
}

if (process.argv[1] && fileURLToPath(import.meta.url) === resolve(process.argv[1])) {
  try {
    const result = checkPagesDeployment({
      eventName: process.env.GITHUB_EVENT_NAME,
      ref: process.env.GITHUB_REF,
      sha: process.env.GITHUB_SHA,
      repository: process.env.GITHUB_REPOSITORY,
    })
    console.log(`Configured Pages URL: ${result.configuredUrl}`)
    console.log(`HTTPS acceptance URL (verify after deployment): ${result.httpsUrl}`)
  } catch (error) {
    console.error(`Pages preflight failed: ${error.message}`)
    process.exitCode = 1
  }
}
