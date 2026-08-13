pub fn chunk_handle(text: &str, chunk_size: usize, chunk_overlap: usize) -> Vec<String> {
    if chunk_size == 0 {
        return vec![];
    }
    let chars = text.chars().collect::<Vec<char>>();
    
    if chars.is_empty() {
        return vec![];
    }
    
    let mut result = vec![];
    let mut start_index = 0;
    
    while start_index < chars.len() {
        let end = (start_index + chunk_size).min(chars.len());
        let chunk = chars[start_index..end].iter().collect::<String>();
        if !chunk.trim().is_empty() {
            result.push(chunk);
        }
        if end == chars.len() {
            break;
        }
        start_index = end - chunk_overlap;
    }

    result
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_chunking() {
        let text = "abcdefghijklmnopqrstuvwxyz";

        let chunks = chunk_handle(text, 10, 2);

        assert_eq!(chunks, vec!["abcdefghij", "ijklmnopqr", "qrstuvwxyz",]);
    }

    #[test]
    fn test_short_text() {
        let text = "hello";

        let chunks = chunk_handle(text, 100, 10);

        assert_eq!(chunks, vec!["hello"]);
    }

    #[test]
    fn test_exact_size_text() {
        let text = "1234567890";

        let chunks = chunk_handle(text, 10, 2);

        assert_eq!(chunks, vec!["1234567890"]);
    }

    #[test]
    fn test_overlap() {
        let text = "abcdefghijklmnop";

        let chunks = chunk_handle(text, 6, 2);

        assert_eq!(chunks, vec!["abcdef", "efghij", "ijklmn", "mnop",]);
    }

    #[test]
    fn test_empty_text() {
        let chunks = chunk_handle("", 10, 2);

        assert!(chunks.is_empty());
    }

    #[test]
    fn test_zero_chunk_size() {
        let chunks = chunk_handle("hello", 0, 0);

        assert!(chunks.is_empty());
    }

    #[test]
    fn test_chinese_utf8_boundary() {
        let text = "你好世界你好世界";

        let chunks = chunk_handle(text, 4, 1);

        for chunk in &chunks {
            // Important:
            // every chunk must be valid UTF-8
            assert!(std::str::from_utf8(chunk.as_bytes()).is_ok());
        }
    }

    #[test]
    fn test_no_replacement_character() {
        let text = "你好世界";

        let chunks = chunk_handle(text, 4, 1);

        for chunk in chunks {
            assert!(
                !chunk.contains('�'),
                "chunk contains invalid UTF-8 replacement character"
            );
        }
    }
}