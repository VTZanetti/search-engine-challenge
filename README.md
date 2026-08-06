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

Os comandos de instalação de dependências e execução serão detalhados aqui conforme o motor for implementado.
## Licença

[MIT](LICENSE) © Vitor Zanetti
