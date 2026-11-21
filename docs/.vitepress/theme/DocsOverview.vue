<script setup lang="ts">
import { withBase } from 'vitepress'
import gateIcon from '../../../assets/icons/transparent/beyond-gate-icon-1024.png'
import logo from '../../../assets/beyond-gate-logo.png'
</script>

<template>
  <div class="overview">
    <section class="entrance" aria-labelledby="overview-title">
      <div class="entrance-copy">
        <img class="wordmark" :src="logo" alt="Beyond Gate" width="1280" height="546">
        <h2 id="overview-title">Your next lookup.<br><em>Through the gate.</em></h2>
        <p class="entrance-description">
          A native CLI and local MCP server for authorized D&amp;D Beyond reads.
          One shared core. Two ways to look things up.
        </p>
        <div class="entrance-actions">
          <a class="action-primary" :href="withBase('/installation')">Read installation <span aria-hidden="true">&rarr;</span></a>
          <a class="action-secondary" :href="withBase('/authentication')">Understand authentication</a>
        </div>
        <p class="entrance-note">Local-first. Read-only. No Node.js product runtime.</p>
        <p class="entrance-status">No published release yet.
          <a :href="withBase('/cli')">Check current capabilities and limits.</a></p>
      </div>
      <figure class="gate-art" aria-label="Beyond Gate red gate symbol">
        <div class="gate-frame"><img :src="gateIcon" alt="" width="1024" height="1024"></div>
        <figcaption>Keep the source.<br>Choose your interface.</figcaption>
      </figure>
    </section>

    <section class="interface-section" aria-labelledby="interfaces-title">
      <div class="section-intro">
        <h2 id="interfaces-title">Two paths.<br>The same boundaries.</h2>
        <p>Use the terminal directly, or connect a local MCP client.
          Both adapters use the same access checks and bounded reads.</p>
      </div>
      <div class="interface-paths">
        <article class="interface-path">
          <h3><a :href="withBase('/cli')">At the terminal <span aria-hidden="true">&nearr;</span></a></h3>
          <p>Find command syntax, output formats, and errors in the CLI reference.</p>
          <div class="command-example"><span class="command-label">Credential-free help</span><code>beyond-gate --help</code></div>
          <a class="text-link" :href="withBase('/cli')">Open CLI reference <span aria-hidden="true">&rarr;</span></a>
        </article>
        <article class="interface-path">
          <h3><a :href="withBase('/mcp')">With a local client <span aria-hidden="true">&nearr;</span></a></h3>
          <p>Read the stdio setup, tool inputs, result contracts, and lifecycle.</p>
          <div class="command-example"><span class="command-label">Inspect startup options</span><code>beyond-gate-mcp --help</code></div>
          <a class="text-link" :href="withBase('/mcp')">Open MCP reference <span aria-hidden="true">&rarr;</span></a>
        </article>
      </div>
    </section>

    <section class="further-section" aria-labelledby="further-title">
      <h2 id="further-title">Look behind the gate.</h2>
      <p>Building, testing, or reviewing Beyond Gate?</p>
      <div class="further-links">
        <a :href="withBase('/development')">Developer guide <span aria-hidden="true">&rarr;</span></a>
        <a :href="withBase('/architecture')">Architecture <span aria-hidden="true">&rarr;</span></a>
        <a :href="withBase('/testing')">Testing and evidence <span aria-hidden="true">&rarr;</span></a>
        <a :href="withBase('/support')">Support and compatibility <span aria-hidden="true">&rarr;</span></a>
        <a :href="withBase('/privacy')">Privacy <span aria-hidden="true">&rarr;</span></a>
      </div>
    </section>
  </div>
</template>
