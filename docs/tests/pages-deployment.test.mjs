import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { test } from 'node:test'
import { fileURLToPath } from 'node:url'
import { checkPagesDeployment } from '../../scripts/check-pages-deployment.mjs'

const sha = 'a'.repeat(40)
const context = {
  repository: 'PlagueHO/beyond-gate',
  eventName: 'push',
  ref: 'refs/heads/main',
  sha,
}

/** Supply synthetic GitHub responses and record every requested endpoint. */
function fixture() {
  const responses = {
    'repos/PlagueHO/beyond-gate': { default_branch: 'main' },
    'repos/PlagueHO/beyond-gate/pages': {
      build_type: 'workflow',
      html_url: 'http://danielscottraynsford.com/beyond-gate/',
    },
    'repos/PlagueHO/beyond-gate/git/ref/heads/main': {
      ref: 'refs/heads/main',
      object: { type: 'commit', sha },
    },
  }
  const calls = []
  return {
    responses,
    calls,
    readJson(endpoint) {
      calls.push(endpoint)
      assert.ok(Object.hasOwn(responses, endpoint), `Unexpected endpoint: ${endpoint}`)
      return responses[endpoint]
    },
  }
}

test('only current default-branch push and manual runs pass', () => {
  for (const eventName of ['push', 'workflow_dispatch']) {
    const { readJson, calls } = fixture()
    assert.deepEqual(checkPagesDeployment({ ...context, eventName }, readJson), {
      configuredUrl: 'http://danielscottraynsford.com/beyond-gate/',
      httpsUrl: 'https://danielscottraynsford.com/beyond-gate/',
    })
    assert.deepEqual(calls, [
      'repos/PlagueHO/beyond-gate',
      'repos/PlagueHO/beyond-gate/pages',
      'repos/PlagueHO/beyond-gate/git/ref/heads/main',
    ])
  }
})

test('pull requests, privileged PR events, forks, and missing SHAs fail before API access', () => {
  for (const change of [
    { eventName: 'pull_request' },
    { eventName: 'pull_request_target' },
    { repository: 'example/beyond-gate' },
    { sha: undefined },
  ]) {
    const { readJson, calls } = fixture()
    assert.throws(() => checkPagesDeployment({ ...context, ...change }, readJson), /requires a push or manual run/)
    assert.deepEqual(calls, [])
  }
})

test('manual feature branches and tags cannot deploy', () => {
  for (const ref of ['refs/heads/feature', 'refs/tags/v1.0.0', 'refs/pull/37/merge']) {
    const { readJson, calls } = fixture()
    assert.throws(() => checkPagesDeployment({ ...context, eventName: 'workflow_dispatch', ref }, readJson), /requires the current default branch/)
    assert.equal(calls.length, 1)
  }
})

test('a default-branch change invalidates an older event context', () => {
  const { responses, readJson } = fixture()
  responses['repos/PlagueHO/beyond-gate'].default_branch = 'trunk'
  assert.throws(() => checkPagesDeployment(context, readJson), /requires the current default branch/)
})

test('an older run cannot deploy after a newer commit, including a manual rerun', () => {
  for (const eventName of ['push', 'workflow_dispatch']) {
    const { responses, readJson } = fixture()
    responses['repos/PlagueHO/beyond-gate/git/ref/heads/main'].object.sha = 'b'.repeat(40)
    assert.throws(() => checkPagesDeployment({ ...context, eventName }, readJson), /Outdated Pages run/)
  }
})

test('missing or non-commit branch responses fail closed', () => {
  for (const latest of [{}, { ref: context.ref }, {
    ref: context.ref,
    object: { type: 'tag', sha },
  }]) {
    const { responses, readJson } = fixture()
    responses['repos/PlagueHO/beyond-gate/git/ref/heads/main'] = latest
    assert.throws(() => checkPagesDeployment(context, readJson), /Outdated Pages run/)
  }
})

test('unexpected hosts, paths, protocols, or publishing types stop deployment', () => {
  for (const html_url of [
    'https://plagueho.github.io/beyond-gate/',
    'https://danielscottraynsford.com/',
    'https://danielscottraynsford.com/beyond-gate',
    'https://danielscottraynsford.com/beyond-gate/?preview=1',
    'https://danielscottraynsford.com/beyond-gate/#fragment',
    'https://user@danielscottraynsford.com/beyond-gate/',
    'ftp://danielscottraynsford.com/beyond-gate/',
  ]) {
    const { responses, readJson, calls } = fixture()
    responses['repos/PlagueHO/beyond-gate/pages'].html_url = html_url
    assert.throws(() => checkPagesDeployment(context, readJson), /Unexpected Pages configuration/)
    assert.equal(calls.length, 2)
  }
  const { responses, readJson } = fixture()
  responses['repos/PlagueHO/beyond-gate/pages'].build_type = 'legacy'
  assert.throws(() => checkPagesDeployment(context, readJson), /Unexpected Pages configuration/)
})

test('the same configured HTTPS URL is accepted without changing settings', () => {
  const { responses, readJson } = fixture()
  responses['repos/PlagueHO/beyond-gate/pages'].html_url = 'https://danielscottraynsford.com/beyond-gate/'
  assert.equal(checkPagesDeployment(context, readJson).configuredUrl, responses['repos/PlagueHO/beyond-gate/pages'].html_url)
})

test('each failed metadata or ref lookup stops deployment', () => {
  for (const failedCall of [1, 2, 3]) {
    const { readJson } = fixture()
    let calls = 0
    assert.throws(() => checkPagesDeployment(context, endpoint => {
      calls += 1
      if (calls === failedCall) throw new Error(`Synthetic HTTP 403: ${endpoint}`)
      return readJson(endpoint)
    }), /Synthetic HTTP 403/)
    assert.equal(calls, failedCall)
  }
})

test('malformed Pages URLs stop deployment before the branch lookup', () => {
  const { responses, readJson, calls } = fixture()
  responses['repos/PlagueHO/beyond-gate/pages'].html_url = 'not a URL'
  assert.throws(() => checkPagesDeployment(context, readJson), /Invalid URL/)
  assert.equal(calls.length, 2)
})

test('the command reports a refusal on stderr with a failing exit code', () => {
  const result = spawnSync(process.execPath, [
    fileURLToPath(new URL('../../scripts/check-pages-deployment.mjs', import.meta.url)),
  ], {
    env: { ...process.env, GITHUB_EVENT_NAME: 'pull_request' },
    encoding: 'utf8',
  })
  assert.equal(result.status, 1)
  assert.equal(result.stdout, '')
  assert.match(result.stderr, /Pages preflight failed:/)
})
