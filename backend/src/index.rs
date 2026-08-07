use std::sync::Arc;

use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::document::Value;
use tantivy::schema::{Field, NumericOptions, Schema, TEXT};
use tantivy::{doc, Index, IndexReader, ReloadPolicy};

use crate::model::Movie;

/// Full-text index over the movie fields, using Tantivy BM25.
pub struct FullTextIndex {
    reader: IndexReader,
    index: Arc<Index>,
    fields: Vec<Field>,
    movie_id_field: Field,
}

impl FullTextIndex {
    pub fn build(movies: &[Movie]) -> Result<Self, Box<dyn std::error::Error>> {
        let mut schema_builder = Schema::builder();
        let title = schema_builder.add_text_field("movie_title", TEXT);
        let genres = schema_builder.add_text_field("genres", TEXT);
        let director = schema_builder.add_text_field("director_name", TEXT);
        let actors = schema_builder.add_text_field("actors", TEXT);
        let keywords = schema_builder.add_text_field("plot_keywords", TEXT);
        // Hidden field to recover the movie id of a document.
        let movie_id = schema_builder.add_u64_field("movie_id", NumericOptions::default().set_indexed());
        let schema = schema_builder.build();

        let index = Arc::new(Index::create_in_ram(schema));
        let mut writer = index.writer_with_num_threads(4, 64 * 1024 * 1024)?;

        for movie in movies {
            writer.add_document(doc!(
                title => movie.title.clone(),
                genres => movie.genres.join(" "),
                director => movie.director.clone(),
                actors => movie.actors.join(" "),
                keywords => movie.plot_keywords.join(" "),
                movie_id => movie.id as u64,
            ))?;
        }

        writer.commit()?;

        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()?;

        Ok(Self {
            reader,
            index,
            fields: vec![title, genres, director, actors, keywords],
            movie_id_field: movie_id,
        })
    }

    /// Search and return a ranked list of movie ids (BM25 relevance score).
    pub fn search(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<(usize, f32)>, Box<dyn std::error::Error>> {
        let searcher = self.reader.searcher();
        let mut parser = QueryParser::for_index(&self.index, self.fields.clone());
        parser.set_conjunction_by_default();

        // BM25 field boosts: title is the strongest signal, keywords second.
        parser.set_field_boost(self.fields[0], 2.0); // movie_title
        parser.set_field_boost(self.fields[4], 1.5); // plot_keywords

        let q = parser.parse_query(query)?;
        let top_docs = searcher.search(&q, &TopDocs::with_limit(limit))?;

        let mut results = Vec::with_capacity(top_docs.len());
        for (score, doc_address) in top_docs {
            let doc: tantivy::TantivyDocument = searcher.doc(doc_address)?;
            let movie_id: Option<u64> = doc
                .get_first(self.movie_id_field)
                .and_then(|v| v.as_u64());
            if let Some(id) = movie_id {
                results.push((id as usize, score));
            }
        }
        Ok(results)
    }
}