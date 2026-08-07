<h1 align="center">Search Engine Challenge</h1>

<p align="center">Motor de busca inspirado no Google para o dataset de 5.000 filmes do IMDB, com ranking por relevância.</p>

<p align="center">
  <img src="https://img.shields.io/badge/status-em%20desenvolvimento-orange.svg" alt="Status: em desenvolvimento">
  <img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="Licença MIT">
</p>

## O desafio

Este repositório é a resposta ao desafio lançado no vídeo [Júnior x Pleno x Sênior — Motor de Busca](https://www.youtube.com/watch?v=-igoPz__fng): construir, do zero, um motor de busca estilo Google sobre o dataset de 5.000 filmes do IMDB. A mesma base de dados, três níveis de senioridade, três abordagens diferentes.

## Dataset

O arquivo `db/movie_metadata.csv` contém **5.044 filmes** com 28 colunas. As principais utilizadas pela busca:

| Coluna | Descrição |
| --- | --- |
| `movie_title` | Título do filme |
| `genres` | Gêneros separados por `\|` (ex.: `Action\|Adventure\|Sci-Fi`) |
| `director_name` | Diretor principal |
| `actor_1_name`, `actor_2_name`, `actor_3_name` | Elenco principal |
| `plot_keywords` | Palavras-chave da trama separadas por `\|` |
| `imdb_score` | Nota no IMDB |
| `title_year` | Ano de lançamento |
| `movie_imdb_link` | Link para a página do filme no IMDB |


## Instalação

```bash
git clone https://github.com/VTZanetti/search-engine-challenge.git
cd search-engine-challenge
```

## Estrutura

```
├── backend/        # Rust + Actix-web (BM25 Tantivy + embeddings, RRF fusion)
├── frontend/       # Vue 3 + GlasstoraUI (interface de busca)
└── db/             # movie_metadata.csv + embeddings.bin (cache) 
```

## Backend (Rust + Actix-web)

Motor de busca híbrido: **BM25 (Tantivy)** + **embeddings semânticos**
(`all-MiniLM-L6-v2` via `fastembed`, local/offline), fundidos por **RRF**. Cada
busca reporta `elapsed_ms`.

```bash
cd backend
cargo run              # dev — http://127.0.0.1:8080
cargo build --release  # build otimizado
```

- `GET /api/search?q=<q>&limit=20&offset=0` → resultados + `elapsed_ms` + total.
- `GET /api/suggest?q=<q>&limit=8` → sugestões de autocomplete.
- `GET /api/health` → status e total de filmes.

## Frontend (Vue 3 + GlasstoraUI)

Interface de busca com autocomplete, paginação e exibição do tempo de resposta,
construída com [GlasstoraUI](https://github.com/VTZanetti/GlasstoraUI)
(glassmorphism monochrome components) sobre Vite + Vue 3 + TypeScript.

**Requisitos**: Node.js ≥ 18 e o backend rodando em `http://127.0.0.1:8080`
(`cd backend && cargo run`).

```bash
cd frontend
npm install
npm run dev            # http://localhost:5173
```

O Vite faz proxy de `/api` para `http://127.0.0.1:8080` (config em
`frontend/vite.config.ts`), então não há dependência de CORS em desenvolvimento.

### Recursos

- Busca híbrida (BM25 + embeddings) com campo estilo Google e botão de submit.
- **Autocomplete** com debounce (~300ms) via `GET /api/suggest`, exibido em um
  `GlassPopover` com badges de tipo (`title` / `keyword`).
- Lista de resultados em `GlassCard`: título, ano, diretor, gêneros
  (`GlassBadge`), `imdb_score`, plot keywords, elenco e link do IMDB.
- **Paginação** com `GlassPagination` (20 resultados por página, `limit=20`).
- `GlassBadge` com o total de resultados e `elapsed_ms` de cada consulta.
- `GlassSpinner` durante o carregamento, estado vazio amigável e aviso quando o
  backend está offline.
- Tema monocromático escuro (padrão Glasstora) com fonte monoespaçada.

### Estrutura

```
frontend/
├── index.html
├── vite.config.ts          # proxy /api → 127.0.0.1:8080
├── tsconfig.json
└── src/
    ├── main.ts             # app Vue + GlassProvider (tema escuro)
    ├── style.css           # tokens de tema monoespaçado
    ├── api.ts              # cliente fetch tipado (search, suggest, health)
    ├── App.vue             # busca, autocomplete, resultados, paginação
    ├── env.d.ts
    └── public/favicon.svg
```

### Build de produção

```bash
cd frontend
npm run build      # vue-tsc + vite build → frontend/dist
npm run preview    # serve o build localmente
```
## Licença

[MIT](LICENSE) © Vitor Zanetti
