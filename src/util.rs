pub fn estimate_tokens_for_text(text: &str) -> usize {
    // Simple heuristic: 1 token ~ 4 characters
    let chars = text.chars().count();
    (chars + 3) / 4
}

// Keep util minimal for now; chunking moved to io::chunk_text_for_token_limit
