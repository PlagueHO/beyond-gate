<script setup lang="ts">
import { defineAsyncComponent, onMounted, onUnmounted } from 'vue'
import DefaultTheme from 'vitepress/theme'
import gateIcon from '../../../assets/icons/transparent/beyond-gate-icon-64.png'

const Layout = DefaultTheme.Layout
const ThemeStudy = __DOCS_THEME_STUDIES__
  ? defineAsyncComponent(() => import('./ThemeStudy.vue'))
  : undefined

function restoreSearchFocus(event: KeyboardEvent) {
  if (event.key !== 'Escape' || !document.querySelector('.VPLocalSearchBox')) return
  // The stable theme removes the search input before its focus trap can return focus.
  requestAnimationFrame(() => {
    if (!document.querySelector('.VPLocalSearchBox')) {
      document.querySelector<HTMLButtonElement>('.VPNavBarSearch button')?.focus()
    }
  })
}

onMounted(() => window.addEventListener('keydown', restoreSearchFocus, { capture: true }))
onUnmounted(() => window.removeEventListener('keydown', restoreSearchFocus, { capture: true }))
</script>

<template>
  <Layout>
    <template v-if="ThemeStudy" #layout-top><component :is="ThemeStudy" /></template>
    <template #nav-bar-title-before>
      <img :src="gateIcon" class="logo" alt="" width="34" height="34">
    </template>
  </Layout>
</template>
