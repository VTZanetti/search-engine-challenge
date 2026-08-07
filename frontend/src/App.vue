<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'

import {
  GlassAlert,
  GlassBadge,
  GlassButton,
  GlassCard,
  GlassField,
  GlassInput,
  GlassPagination,
  GlassPopover,
  GlassSpinner,
} from 'glasstora'
import { health, search, suggest, type MovieResult, type Suggestion } from './api'

const PAGE_SIZE = 20
const DEBOUNCE_MS = 300

/* ---------------------------------- state --------------------------------- */

const inputValue = ref('')

const loading = ref(false)
const error = ref<string | null>(null)

const results = ref<MovieResult[]>([])
const total = ref(0)
const elapsedMs = ref<number | null>(null)

// The query that produced the current results, kept separate from the input so
// pagination always re-runs the same query the user submitted.
const lastQuery = ref('')

const page = ref(1)
const pageCount = computed(() => Math.max(1, Math.ceil(total.value / PAGE_SIZE)))

const suggestions = ref<Suggestion[]>([])
const suggestionsOpen = ref(false)
const highlightIndex = ref(-1)

let suggestTimer: ReturnType<typeof setTimeout> | null = null
let searchSeq = 0
const inputEl = ref<InstanceType<typeof GlassInput> | null>(null)

const hasSearched = ref(false)
const backendOnline = ref<boolean | null>(null)

/* --------------------------------- helpers -------------------------------- */

const pageLabel = (p: number) => `page ${p}`

function sanitize(q: string): string {
  return q.trim()
}

/* -------------------------------- autocomplete ---------------------------- */

async function fetchSuggestions(raw: string) {
  const q = sanitize(raw)
  if (!q) {
    suggestions.value = []
    suggestionsOpen.value = false
    return
  }
  try {
    const res = await suggest(q, 8)
    // Only show the dropdown while the input still matches what we asked for.
    if (q === sanitize(inputValue.value)) {
      suggestions.value = res.suggestions
      suggestionsOpen.value = res.suggestions.length > 0
      highlightIndex.value = -1
    }
  } catch {
    suggestions.value = []
    suggestionsOpen.value = false
  }
}

function onInputFocus() {
  // Reopen the dropdown when the field regains focus and there are suggestions.
  if (suggestions.value.length > 0) suggestionsOpen.value = true
}

// Debounced autocomplete: every keystroke restarts a 300ms timer, then the
// /api/suggest call fires only once the user has stopped typing.
watch(inputValue, (value) => {
  if (suggestTimer) clearTimeout(suggestTimer)
  suggestTimer = setTimeout(() => fetchSuggestions(value), DEBOUNCE_MS)
})

// The popover moves focus to its panel when it opens; hand it straight back to
// the search field so typing never gets interrupted.
watch(suggestionsOpen, async (open) => {
  if (open) {
    await nextTick()
    inputEl.value?.focus()
  }
})

function pickSuggestion(s: Suggestion) {
  inputValue.value = s.text
  suggestions.value = []
  suggestionsOpen.value = false
  runSearch(s.text)
}

function onSuggestionKeydown(ev: KeyboardEvent) {
  const n = suggestions.value.length
  if (!n) return
  if (ev.key === 'ArrowDown') {
    ev.preventDefault()
    highlightIndex.value = (highlightIndex.value + 1) % n
  } else if (ev.key === 'ArrowUp') {
    ev.preventDefault()
    highlightIndex.value = (highlightIndex.value - 1 + n) % n
  } else if (ev.key === 'Enter' && highlightIndex.value >= 0) {
    ev.preventDefault()
    pickSuggestion(suggestions.value[highlightIndex.value])
  } else if (ev.key === 'Escape') {
    suggestionsOpen.value = false
  }
}

/* --------------------------------- search --------------------------------- */

async function runSearch(raw: string, targetPage = 1) {
  const q = sanitize(raw)
  if (!q) return
  if (suggestTimer) clearTimeout(suggestTimer)
  suggestions.value = []
  suggestionsOpen.value = false

  const seq = ++searchSeq
  loading.value = true
  error.value = null
  hasSearched.value = true

  try {
    const offset = (targetPage - 1) * PAGE_SIZE
    const res = await search(q, PAGE_SIZE, offset)
    if (seq !== searchSeq) return // a newer search superseded this one
    results.value = res.results
    total.value = res.total
    elapsedMs.value = res.elapsed_ms
    lastQuery.value = q
    page.value = targetPage
  } catch (e) {
    if (seq !== searchSeq) return
    error.value = e instanceof Error ? e.message : 'Search failed'
    results.value = []
    total.value = 0
  } finally {
    if (seq === searchSeq) loading.value = false
  }
}

function onSubmit() {
  page.value = 1
  runSearch(inputValue.value, 1)
}

function goToPage(p: number) {
  if (p < 1 || p > pageCount.value) return
  // `page` has already been updated by the component's update:modelValue before
  // `change` fires, so there is no equality guard here — the query to re-run
  // comes from lastQuery, not the input.
  runSearch(lastQuery.value || inputValue.value, p)
}

/* ---------------------------------- boot ---------------------------------- */

onMounted(async () => {
  try {
    await health()
    backendOnline.value = true
  } catch {
    backendOnline.value = false
  }
})

onUnmounted(() => {
  if (suggestTimer) clearTimeout(suggestTimer)
})
</script>

<template>
  <div class="app">
    <header class="app__header">
      <p class="app__kicker">hybrid search · bm25 + embeddings · 5043 films</p>
    </header>

    <main class="app__main">
      <div class="search-zone">
        <h1 class="app__title">movie search</h1>

        <GlassPopover
          v-model="suggestionsOpen"
          placement="bottom-start"
          :offset="6"
          close-on-outside
          close-on-esc
        >
          <template #trigger="{ attrs }">
            <form class="search-form" role="search" @submit.prevent="onSubmit">
              <GlassField label="search movies" class="search-field">
                <GlassInput
                  ref="inputEl"
                  v-model="inputValue"
                  size="lg"
                  placeholder="search 5,043 films…"
                  autocomplete="off"
                  block-caret
                  v-bind="attrs"
                  @focus="onInputFocus"
                  @keydown="onSuggestionKeydown"
                />
              </GlassField>
              <GlassButton type="submit" size="lg" :disabled="!inputValue.trim()">
                search
              </GlassButton>
            </form>
          </template>

          <div class="suggest-panel" role="listbox" aria-label="suggestions">
            <p v-if="suggestions.length === 0" class="suggest-panel__empty">
              no suggestions
            </p>
            <button
              v-for="(s, i) in suggestions"
              :key="`${s.type}-${s.text}-${i}`"
              type="button"
              class="suggest-item"
              :class="{ 'suggest-item--active': i === highlightIndex }"
              role="option"
              :aria-selected="i === highlightIndex"
              @click="pickSuggestion(s)"
              @mouseenter="highlightIndex = i"
            >
              <GlassBadge variant="outline" :dot="false" class="suggest-item__type">
                {{ s.type }}
              </GlassBadge>
              <span class="suggest-item__text">{{ s.text }}</span>
            </button>
          </div>
        </GlassPopover>

        <p v-if="backendOnline === false" class="status-line status-line--error">
          ⚠ backend offline — start it with <code>cd backend && cargo run</code>
        </p>
      </div>

      <section class="results" aria-live="polite">
        <div v-if="loading" class="results__loading">
          <GlassSpinner size="lg" label="searching" />
          <span class="results__loading-text">searching…</span>
        </div>

        <GlassAlert v-else-if="error" variant="error" title="search error">
          {{ error }}
        </GlassAlert>

        <div v-else-if="hasSearched && results.length === 0" class="results__empty">
          <p class="results__empty-title">no results</p>
          <p class="results__empty-sub">try a different title, genre or director.</p>
        </div>

        <template v-else-if="results.length > 0">
          <div class="results__meta">
            <GlassBadge variant="neutral" class="results__badge">
              {{ total }} result{{ total === 1 ? '' : 's' }}
            </GlassBadge>
            <GlassBadge
              v-if="elapsedMs !== null"
              variant="solid"
              class="results__badge"
            >
              {{ elapsedMs.toFixed(1) }} ms
            </GlassBadge>
          </div>

          <ul class="results__list">
            <li v-for="(m, i) in results" :key="m.id">
              <GlassCard :elevation="2" radius="md" class="movie-card">
                <div class="movie-card__top">
                  <div class="movie-card__heading">
                    <span class="movie-card__rank">{{ String(i + 1).padStart(2, '0') }}</span>
                    <div>
                      <h3 class="movie-card__title">{{ m.title }}</h3>
                      <p class="movie-card__sub">
                        <span v-if="m.year">{{ m.year }}</span>
                        <span v-if="m.year && m.director"> · </span>
                        <span v-if="m.director">{{ m.director }}</span>
                      </p>
                    </div>
                  </div>
                  <a
                    v-if="m.link"
                    class="movie-card__link"
                    :href="m.link"
                    target="_blank"
                    rel="noopener noreferrer"
                  >
                    imdb ↗
                  </a>
                </div>

                <div v-if="m.genres.length" class="movie-card__genres">
                  <GlassBadge
                    v-for="g in m.genres"
                    :key="g"
                    variant="outline"
                    :dot="false"
                    class="movie-card__genre"
                  >
                    {{ g }}
                  </GlassBadge>
                  <GlassBadge v-if="m.imdb_score !== null" variant="solid" :dot="false">
                    {{ m.imdb_score.toFixed(1) }}
                  </GlassBadge>
                </div>

                <p v-if="m.plot_keywords.length" class="movie-card__keywords">
                  {{ m.plot_keywords.join(' · ') }}
                </p>

                <p v-if="m.actors.length" class="movie-card__actors">
                  {{ m.actors.join(', ') }}
                </p>
              </GlassCard>
            </li>
          </ul>

          <GlassPagination
            v-if="pageCount > 1"
            v-model="page"
            :page-count="pageCount"
            :sibling-count="1"
            :boundary-count="1"
            previous-label="previous page"
            next-label="next page"
            :page-label="pageLabel"
            class="results__pagination"
            @change="goToPage"
          />
        </template>
      </section>
    </main>

    <footer class="app__footer">
      <span>search-engine-challenge</span>
      <span>session a · glasstora ui</span>
    </footer>
  </div>
</template>

<style scoped>
.app {
  display: flex;
  flex-direction: column;
  min-height: 100vh;
  padding: 0 20px;
}

.app__header {
  padding: 18px 0 8px;
}

.app__kicker {
  margin: 0;
  font-size: 12px;
  letter-spacing: 0.12em;
  text-transform: lowercase;
  color: var(--gt-fg-faint);
}

.app__main {
  flex: 1;
  width: 100%;
  max-width: 860px;
  margin: 0 auto;
  padding: 40px 0 60px;
}

.search-zone {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 14px;
  margin-bottom: 36px;
}

.app__title {
  margin: 0 0 10px;
  font-size: clamp(28px, 6vw, 44px);
  font-weight: 600;
  letter-spacing: -0.02em;
  color: var(--gt-fg);
  text-align: center;
}

.search-form {
  display: flex;
  gap: 10px;
  width: 100%;
  max-width: 640px;
  /* Align the button with the input, not with the field label above it. */
  align-items: flex-end;
}

/*
 * GlassPopover wraps its trigger in a `span.gt-popover__anchor` with
 * `display: inline-flex`, which shrink-wraps the form and keeps it from
 * stretching to its intended 640px. Make the anchor a full-width block so the
 * form (and therefore the input + button row) fills the available space.
 */
.search-zone :deep(.gt-popover__anchor) {
  display: block;
  width: min(640px, 100%);
  margin: 0 auto;
}

.search-field {
  flex: 1;
}

.search-form :deep(.gt-field__control) {
  width: 100%;
}

.search-form :deep(.gt-input) {
  width: 100%;
  border-radius: var(--gt-radius-md);
}

.suggest-panel {
  display: flex;
  flex-direction: column;
  gap: 2px;
  width: 100%;
}

.suggest-panel__empty {
  margin: 0;
  color: var(--gt-fg-faint);
}

.suggest-item {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
  padding: 7px 6px;
  background: transparent;
  border: none;
  border-radius: var(--gt-radius-sm);
  font-family: var(--gt-font-mono);
  font-size: var(--gt-text-sm);
  color: var(--gt-fg-muted);
  text-align: left;
  cursor: pointer;
}

.suggest-item--active {
  background: rgb(var(--gt-line-tint) / var(--gt-fill-alpha));
  color: var(--gt-fg);
}

.suggest-item__type {
  flex-shrink: 0;
  min-width: 3.4rem;
  text-align: center;
}

.suggest-item__text {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.status-line {
  margin: 0;
  font-size: 12px;
  color: var(--gt-fg-muted);
}

.status-line--error {
  color: #e8a0a0;
}

.status-line code {
  font-size: 12px;
}

.results__loading {
  display: flex;
  align-items: center;
  gap: 12px;
  justify-content: center;
  padding: 48px 0;
  color: var(--gt-fg-muted);
}

.results__empty {
  padding: 48px 0;
  text-align: center;
}

.results__empty-title {
  margin: 0 0 6px;
  font-size: 18px;
  color: var(--gt-fg);
}

.results__empty-sub {
  margin: 0;
  font-size: 13px;
  color: var(--gt-fg-muted);
}

.results__meta {
  display: flex;
  gap: 8px;
  margin-bottom: 14px;
}

.results__list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.movie-card__top {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 12px;
}

.movie-card__heading {
  display: flex;
  gap: 12px;
  align-items: baseline;
}

.movie-card__rank {
  font-size: 13px;
  color: var(--gt-fg-faint);
}

.movie-card__title {
  margin: 0;
  font-size: 17px;
  font-weight: 600;
  letter-spacing: 0.01em;
  color: var(--gt-fg);
}

.movie-card__sub {
  margin: 3px 0 0;
  font-size: 12px;
  color: var(--gt-fg-muted);
}

.movie-card__link {
  flex-shrink: 0;
  font-size: 12px;
  color: var(--gt-fg-muted);
  text-decoration: none;
}

.movie-card__link:hover {
  color: var(--gt-fg);
  text-decoration: underline;
}

.movie-card__genres {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin-top: 10px;
}

.movie-card__keywords {
  margin: 10px 0 0;
  font-size: 12px;
  line-height: 1.6;
  color: var(--gt-fg-muted);
}

.movie-card__actors {
  margin: 8px 0 0;
  font-size: 12px;
  color: var(--gt-fg-faint);
}

.results__pagination {
  display: flex;
  justify-content: center;
  margin-top: 26px;
}

.app__footer {
  display: flex;
  justify-content: space-between;
  gap: 10px;
  padding: 16px 0 22px;
  font-size: 11px;
  letter-spacing: 0.08em;
  text-transform: lowercase;
  color: var(--gt-fg-faint);
}
</style>
