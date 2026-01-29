
#[cfg(test)]
mod tests {
    use serde_json;

    /// Helper to parse plan content (copied from agentic_executor.rs for testing)
    fn parse_plan_content(content: &str) -> Vec<String> {
        // Clean markdown ```json ... ``` if present
        let clean_content = content.replace("```json", "").replace("```", "").trim().to_string();
        
        serde_json::from_str(&clean_content)
            .unwrap_or_else(|_| {
                // Fallback: split by newlines if strict JSON parsing fails
                // Also handle numbered lists like "1. Step"
                clean_content.lines()
                    .filter(|l| !l.trim().is_empty())
                    .map(|l| {
                        let trimmed = l.trim();
                        // Remove leading "1. ", "2. ", "- " etc
                        if let Some(idx) = trimmed.find(". ") {
                            if trimmed[..idx].chars().all(char::is_numeric) {
                                return trimmed[idx+2..].to_string();
                            }
                        }
                        if trimmed.starts_with("- ") {
                            return trimmed[2..].to_string();
                        }
                        trimmed.to_string()
                    })
                    .collect()
            })
    }

    #[test]
    fn test_parse_plan_json() {
        let content = r#"["Step 1", "Step 2"]"#;
        let plan = parse_plan_content(content);
        assert_eq!(plan, vec!["Step 1", "Step 2"]);
    }

    #[test]
    fn test_parse_plan_markdown_json() {
        let content = r#"```json
        ["Step 1", "Step 2"]
        ```"#;
        let plan = parse_plan_content(content);
        assert_eq!(plan, vec!["Step 1", "Step 2"]);
    }

    #[test]
    fn test_parse_plan_numbered_list() {
        let content = "1. Step 1\n2. Step 2";
        let plan = parse_plan_content(content);
        assert_eq!(plan, vec!["Step 1", "Step 2"]);
    }
    
    #[test]
    fn test_parse_plan_bullet_list() {
        let content = "- Step 1\n- Step 2";
        let plan = parse_plan_content(content);
        assert_eq!(plan, vec!["Step 1", "Step 2"]);
    }

    #[test]
    fn test_parse_plan_mixed_list() {
        let content = "1. Step 1\n- Step 2\nStep 3";
        let plan = parse_plan_content(content);
        assert_eq!(plan, vec!["Step 1", "Step 2", "Step 3"]);
    }
}
