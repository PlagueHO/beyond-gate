<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { useRouter, withBase } from 'vitepress'

const themes = [
  { id: 'gatehouse', name: 'Gatehouse', description: 'Black stone, white lettering, and a red threshold. A direct, modern fantasy guide.' },
  { id: 'folio', name: 'Crimson folio', description: 'A pale reading page with red rules and book-like headings. A reference volume for long sessions.' },
  { id: 'atlas', name: 'Iron atlas', description: 'Cool grey fields, angular details, and a red compass line. An ordered field guide.' },
  { id: 'ember', name: 'Ember archive', description: 'Deep wine, soft white text, and generous book typography. A quiet archive after dark.' },
  { id: 'sigil', name: 'Ivory sigil', description: 'Bright white pages and a bold crimson masthead. A clean handbook with a strong gate identity.' },
]
const selected = ref('gatehouse')
const router = useRouter()
const activeTheme = computed(() => themes.find(theme => theme.id === selected.value)!)

function chooseTheme(id: string) {
  if (!themes.some(theme => theme.id === id)) return
  selected.value = id
  document.documentElement.dataset.study = id
  document.documentElement.classList.toggle('dark', id === 'gatehouse' || id === 'ember')
  const url = new URL(window.location.href)
  url.searchParams.set('theme', id)
  window.history.replaceState(null, '', url)
}

onMounted(() => {
  const requested = new URL(window.location.href).searchParams.get('theme')
  chooseTheme(requested && themes.some(theme => theme.id === requested) ? requested : selected.value)
  // VitePress normalizes link URLs, so preserve the study after route changes.
  router.onAfterRouteChange = () => chooseTheme(selected.value)
})

onUnmounted(() => {
  router.onAfterRouteChange = undefined
})
</script>

<template>
  <aside class="theme-study" aria-label="Theme preview controls">
    <div class="study-heading">
      <strong>Five theme studies</strong>
      <span>Comparison build. Gatehouse is the site default.</span>
      <a :href="withBase('/cli')">Compare the CLI reference <span aria-hidden="true">&rarr;</span></a>
    </div>
    <div class="study-options" role="group" aria-label="Choose a theme">
      <button
        v-for="theme in themes"
        :key="theme.id"
        type="button"
        :aria-pressed="selected === theme.id"
        @click="chooseTheme(theme.id)"
      >
        <span class="study-swatch" :class="theme.id" aria-hidden="true"></span>
        {{ theme.name }}
      </button>
    </div>
    <label class="study-mobile">
      Theme
      <select :value="selected" @change="chooseTheme(($event.target as HTMLSelectElement).value)">
        <option v-for="theme in themes" :key="theme.id" :value="theme.id">{{ theme.name }}</option>
      </select>
    </label>
    <p class="study-description" aria-live="polite">{{ activeTheme.description }}</p>
  </aside>
</template>
