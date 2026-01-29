use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RtcrosConfig {
    pub role: Option<String>,
    pub task: Option<String>,
    pub context: Option<String>,
    pub reasoning: Option<String>,
    pub output: Option<String>,
    pub stop: Option<String>,
}

impl RtcrosConfig {
    pub fn build_system_prompt(&self) -> Option<String> {
        let parts: Vec<String> = vec![
            self.role.as_ref().map(|s| format!("## Role\n{}", s)),
            self.task.as_ref().map(|s| format!("## Task\n{}", s)),
            self.context.as_ref().map(|s| format!("## Context\n{}", s)),
            self.reasoning.as_ref().map(|s| format!("## Reasoning\n{}", s)),
            self.output.as_ref().map(|s| format!("## Output format\n{}", s)),
            self.stop.as_ref().map(|s| format!("## Stop Criteria\n{}", s)),
        ]
        .into_iter()
        .flatten()
        .collect();

        if parts.is_empty() {
            None
        } else {
            Some(parts.join("\n\n"))
        }
    }
}
