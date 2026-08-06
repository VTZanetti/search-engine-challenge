# Planejamento — Search Engine Challenge

## Visão geral

Motor de busca estilo Google para o dataset de **5.044 filmes do IMDB** (`db/movie_metadata.csv`), inspirado no desafio Júnior x Pleno x Sênior do vídeo [Motor de Busca](https://www.youtube.com/watch?v=-igoPz__fng).

O motor usa **ranking BM25** sobre um índice invertido em memória, com **expansão de consulta** para buscar por conceitos ("filmes tristes"), além de autocomplete, paginação, filtros e ordenação.

## Stack

| Camada | Tecnologia | Local |
| --- | --- | --- |
| Backend | Rust (axum, tokio, csv, serde, deunicode) | `backend/` |
| Frontend | Vue 3 + Vite + TypeScript + GlasstoraUI | `frontend/` |
| Dados | `db/movie_metadata.csv` (5.044 filmes, 28 colunas) | `db/` |

## Decisões

| Decisão | Escolha |
| --- | --- |
| Ranking | BM25/TF-IDF |
| Extras | Autocomplete, paginação, filtros, ordenação, busca por conceito |
| Testes | Mínimo (build + validação manual com curl/navegador) |
| Idioma da interface | PT-BR |
| Execução | Local — backend na porta `3000`, frontend na `5173` com proxy `/api` |

## Fases

| Fase | Nome | Resultado |
| --- | --- | --- |
| [0](fases/fase-0-fundacao.md) | Fundação | Repositório estruturado, CSV carregado e normalizado, servidor no ar |
| [1](fases/fase-1-indice-bm25.md) | Índice e BM25 | `GET /api/search` com ranking BM25 por campo |
| [2](fases/fase-2-busca-conceito.md) | Busca por conceito | "filmes tristes" devolve dramas e tragédias relevantes |
| [3](fases/fase-3-api-busca.md) | API de busca | Paginação, filtros, ordenação e autocomplete |
| [4](fases/fase-4-frontend.md) | Frontend | SPA minimalista estilo Google com GlasstoraUI |
| [5](fases/fase-5-polimento.md) | Polimento | README final, `.gitignore`, validação manual completa |

Cada fase possui objetivo, tarefas, critérios de conclusão e escopo próprio, detalhados nos arquivos de `docs/fases/`.
