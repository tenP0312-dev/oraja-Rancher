use std::collections::{BTreeMap, BTreeSet};

fn line_key(line: &str, section: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with(';') || trimmed.starts_with('#') {
        return None;
    }
    let (key, _) = trimmed.split_once('=')?;
    let key = key.trim();
    if key.is_empty() {
        return None;
    }
    Some(if section.is_empty() {
        key.to_string()
    } else {
        format!("{section}.{key}")
    })
}

pub fn update_preserving_layout(input: &str, updates: &BTreeMap<String, String>) -> String {
    let newline = if input.contains("\r\n") { "\r\n" } else { "\n" };
    let mut section = String::new();
    let mut seen = BTreeSet::new();
    let mut output = Vec::new();
    for line in input.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            section = trimmed[1..trimmed.len() - 1].trim().to_string();
            output.push(line.to_string());
            continue;
        }
        if let Some(full_key) = line_key(line, &section) {
            if let Some(value) = updates.get(&full_key) {
                let indent = &line[..line.len() - line.trim_start().len()];
                let key = line
                    .split_once('=')
                    .map(|(key, _)| key.trim())
                    .unwrap_or("");
                output.push(format!("{indent}{key}={value}"));
                seen.insert(full_key);
                continue;
            }
        }
        output.push(line.to_string());
    }
    for (key, value) in updates {
        if !seen.contains(key) {
            output.push(format!("{key}={value}"));
        }
    }
    let mut result = output.join(newline);
    if input.ends_with('\n') || !result.is_empty() {
        result.push_str(newline);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_comments_unknown_keys_order_and_newline_style() {
        let input = "; keep\r\nunknown = yes\r\n[PLAYER]\r\nname=old\r\n# tail\r\n";
        let updates = BTreeMap::from([
            ("PLAYER.name".into(), "new".into()),
            ("language".into(), "ja".into()),
        ]);
        let result = update_preserving_layout(input, &updates);
        assert!(result.contains("; keep\r\nunknown = yes\r\n[PLAYER]\r\nname=new\r\n# tail"));
        assert!(result.ends_with("language=ja\r\n"));
    }
}
