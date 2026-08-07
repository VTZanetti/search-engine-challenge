/**
 * Typed client for the Rust search backend.
 *
 * The response shapes below mirror the API contract exactly. Do not rename any
 * field — the backend (Session B) owns this contract.
 */

export interface MovieResult {
  id: number
  title: string
  year: number | null
  genres: string[]
  director: string
  actors: string[]
  imdb_score: number | null
  plot_keywords: string[]
  link: string
}

export interface SearchResponse {
  query: string
  total: number
  elapsed_ms: number
  results: MovieResult[]
}

export interface Suggestion {
  type: 'title' | 'keyword'
  text: string
}

export interface SuggestResponse {
  suggestions: Suggestion[]
}

export interface HealthResponse {
  status: string
  movies: number
}

const DEFAULT_LIMIT = 20

/** Base URL. In dev the Vite proxy forwards `/api` to the backend. */
const BASE = '/api'

async function getJson<T>(path: string): Promise<T> {
  const res = await fetch(`${BASE}${path}`)
  if (!res.ok) {
    throw new Error(`Request failed: ${res.status} ${res.statusText}`)
  }
  return (await res.json()) as T
}

function qs(params: Record<string, string | number>): string {
  const search = new URLSearchParams()
  for (const [key, value] of Object.entries(params)) {
    search.set(key, String(value))
  }
  return search.toString()
}

export async function search(
  q: string,
  limit = DEFAULT_LIMIT,
  offset = 0,
): Promise<SearchResponse> {
  return getJson<SearchResponse>(`/search?${qs({ q, limit: String(limit), offset: String(offset) })}`)
}

export async function suggest(q: string, limit = 8): Promise<SuggestResponse> {
  return getJson<SuggestResponse>(`/suggest?${qs({ q, limit: String(limit) })}`)
}

export async function health(): Promise<HealthResponse> {
  return getJson<HealthResponse>('/health')
}