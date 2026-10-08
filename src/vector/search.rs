use crate::vector::embed::embed;
use anyhow::anyhow;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use crate::modals::config::Vector;
use crate::vector::chunk::chunk_handle;

pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let norm_a = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot / (norm_a * norm_b)
    }
}

struct Similarity {
    index: usize,
    similarity: f32,
}

impl PartialEq for Similarity {
    fn eq(&self, other: &Self) -> bool {
        self.similarity == other.similarity
    }
}
impl Eq for Similarity {}

impl PartialOrd for Similarity {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(&other))
    }
}
impl Ord for Similarity {
    fn cmp(&self, other: &Self) -> Ordering {
        other.similarity.total_cmp(&self.similarity)
    }
}

#[derive(Debug)]
pub struct SearchResult {
    pub index: usize,
    pub similarity: f32,
    pub content: String,
}

pub async fn vector_search(
    question: &str,
    answer: &str,
    config: &Vector
) -> anyhow::Result<Vec<SearchResult>> {
    let chunks = chunk_handle(answer, config.chunk_size, config.chunk_overlap);
    if question.is_empty() || chunks.is_empty() || config.top_k == 0 {
        return Ok(vec![]);
    }
    let embed_text = embed(&vec![question.to_string()], &config.model)
        .await?
        .pop()
        .ok_or_else(|| anyhow!("no embed"))?;
    let embed_chunks = embed(&chunks, &config.model).await?;

    let mut heap = BinaryHeap::with_capacity(config.top_k + 1);

    for (index, chunk) in embed_chunks.iter().enumerate() {
        let similarity = cosine_similarity(&chunk, &embed_text);
        heap.push(Similarity { index, similarity });
        if heap.len() > config.top_k {
            heap.pop();
        }
    }

    let mut result = heap.into_vec();
    result.sort();
    let result = result
        .iter()
        .map(|data| {
            return SearchResult {
                index: data.index,
                similarity: data.similarity,
                content: chunks[data.index].clone(),
            };
        })
        .collect::<Vec<SearchResult>>();

    Ok(result)
}