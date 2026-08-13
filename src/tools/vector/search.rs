use std::cmp::Ordering;
use std::collections::BinaryHeap;
use anyhow::anyhow;
use tiktoken_rs::cl100k_base;
use tracing::{error, info};
use crate::state::TEXT_EMBEDDING_3_SMALL_MODEL;
use crate::tools::vector::embed::embed;

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
    pub content: String
}

pub async fn vector_search(text: &str, chunks: &[String], top: usize) -> anyhow::Result<Vec<SearchResult>> {
    if text.is_empty() || chunks.is_empty() || top == 0 {
        return Ok(chunks.into_iter().enumerate().map(|(index, data)| {
            return SearchResult {
                index,
                similarity: 1.0,
                content: data.clone()
            }
        }).collect());
    }
    let embed_text = embed(&vec![text.to_string()], TEXT_EMBEDDING_3_SMALL_MODEL).await?.pop().ok_or_else(|| anyhow!("no embed"))?;
    let embed_chunks = embed(chunks, TEXT_EMBEDDING_3_SMALL_MODEL).await?;

    let mut heap = BinaryHeap::with_capacity(top + 1);

    for (index, chunk) in embed_chunks.iter().enumerate() {
        let similarity = cosine_similarity(&chunk, &embed_text);
        heap.push(
            Similarity {
                index,
                similarity,
            }
        );
        if heap.len() > top {
            heap.pop();
        }
    }

    let mut result = heap.into_vec();
    result.sort();
    let result = result.iter().map(|data| {
        return SearchResult {
            index: data.index,
            similarity: data.similarity,
            content: chunks[data.index].clone()
        }
    }).collect::<Vec<SearchResult>>();

    let before_text = chunks.iter().map(String::as_str).collect::<Vec<&str>>().join("");
    let after_text = result.iter().map(|data| data.content.as_str()).collect::<Vec<&str>>().join("");
    let core =  cl100k_base();
    match core {
        Ok(core) => {
            let before_token = core.encode_with_special_tokens(&before_text).len();
            let after_token = core.encode_with_special_tokens(&after_text).len();
            info!("向量搜索执行成功，向量搜索前后token比： {} / {}", after_token, before_token)
        },
        Err(_) => {
            error!("向量搜索执行成功，但节省token初始化失败，向量搜索前后字符串长度比： {} / {}", after_text.len(), before_text.len());
        }
    }
    
    Ok(result)
}