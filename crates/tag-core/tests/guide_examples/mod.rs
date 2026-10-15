//! Extract literal guide text fences without checkout-newline or whitespace changes.
pub fn text_blocks(page: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut lines = page.lines();
    while let Some(line) = lines.next() {
        let Some(indent) = line.strip_suffix("```text") else {
            continue;
        };
        if !indent.chars().all(|ch| ch == ' ') {
            continue;
        }
        let mut body = Vec::new();
        for line in lines.by_ref() {
            if line.trim() == "```" {
                break;
            }
            body.push(line.strip_prefix(indent).unwrap_or(line));
        }
        blocks.push(body.join("\n"));
    }
    blocks
}
