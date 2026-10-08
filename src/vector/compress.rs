use tracing::{debug, error};
use crate::modals::config::Vector;
use crate::vector::search::vector_search;

pub async fn compress(question: &str, answer: &str, config: &Vector) -> String {
    debug!("正在进行向量压缩： 问题：{}， 答案： {}", question, answer);
    let search_result = vector_search(question, answer, &config).await;
    match search_result {
        Ok(result) => {
            if result.is_empty() {
                return answer.to_string();
            }
            result.iter()
                .map(|data| data.content.clone())
                .collect::<Vec<String>>()
                .join("\n\n")
        },
        Err(e) => {
            error!("{}", e);
            answer.to_string()
        }
    }
}