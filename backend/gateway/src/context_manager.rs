use mawi_core::unified::ChatMessage;
use tracing::{info, warn};

/// Trim messages to fit context window (prevents 413 errors)
/// Keeps system prompt + recent history, drops old messages from middle
pub struct ContextManager;

/// How many tokens of the context window to keep free for the model's
/// own response. Reads `MG_CONTEXT_OUTPUT_RESERVATION_TOKENS` (absolute
/// cap, default 2048) and `MG_CONTEXT_OUTPUT_RESERVATION_RATIO` (fraction
/// of the window, default 0.2). Returns the smaller of the two so a
/// large window doesn't accidentally reserve absurdly many output
/// tokens, while a small window doesn't leave the model with too few.
/// Closes #56.
fn output_reservation(context_window: usize) -> usize {
    let max_tokens: usize = std::env::var("MG_CONTEXT_OUTPUT_RESERVATION_TOKENS")
        .ok()
        .and_then(|s| s.parse().ok())
        .filter(|n| *n > 0)
        .unwrap_or(2048);
    let ratio: f64 = std::env::var("MG_CONTEXT_OUTPUT_RESERVATION_RATIO")
        .ok()
        .and_then(|s| s.parse().ok())
        .filter(|r: &f64| *r > 0.0 && *r < 1.0)
        .unwrap_or(0.2);
    std::cmp::min(max_tokens, (context_window as f64 * ratio) as usize)
}

impl ContextManager {
    /// Prune to context limit
    /// Heuristic: 4 chars ≈ 1 token, reserve a configurable slice for output.
    pub fn prune_messages(messages: Vec<ChatMessage>, context_window: usize) -> Vec<ChatMessage> {
        if messages.is_empty() {
            return messages;
        }

        let reservation = output_reservation(context_window);
        let effective_limit = context_window.saturating_sub(reservation);

        let total_estimated = Self::estimate_tokens(&messages);

        if total_estimated <= effective_limit {
            return messages;
        }

        warn!(
            "✂️ context overflow: {} tokens > limit {} (window {}), pruning...",
            total_estimated, effective_limit, context_window
        );

        // separate system prompt if present
        let mut pruned = Vec::new();
        let mut current_tokens = 0;
        let mut messages_pool = messages.clone();

        // keep system prompt if present
        if let Some(first) = messages_pool.first() {
            if first.role == "system" {
                let tokens = Self::estimate_single_token(first);
                current_tokens += tokens;
                pruned.push(messages_pool.remove(0));
            }
        }

        // accumulate recent msgs from end until we hit limit
        let mut recent_history = Vec::new();

        for msg in messages_pool.iter().rev() {
            let tokens = Self::estimate_single_token(msg);
            if current_tokens + tokens > effective_limit {
                break;
            }
            current_tokens += tokens;
            recent_history.push(msg.clone());
        }

        recent_history.reverse();

        // skip marker insertion to save tokens

        let dropped_count = messages.len() - (pruned.len() + recent_history.len());
        if dropped_count > 0 {
            info!(
                "✂️ Pruned {} messages from middle of conversation.",
                dropped_count
            );
        }

        pruned.extend(recent_history);

        pruned
    }

    fn estimate_tokens(messages: &[ChatMessage]) -> usize {
        messages.iter().map(Self::estimate_single_token).sum()
    }

    /// rough token estimate: 4 chars ≈ 1 token + ~4 for msg overhead
    fn estimate_single_token(msg: &ChatMessage) -> usize {
        let content_tokens = msg.content.len() / 4;
        content_tokens + 4 // role, JSON structure, etc
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Env vars are process-wide; serialise tests that mutate them.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn reservation_defaults_to_min_of_2048_and_20_percent() {
        let _g = ENV_LOCK.lock().unwrap();
        std::env::remove_var("MG_CONTEXT_OUTPUT_RESERVATION_TOKENS");
        std::env::remove_var("MG_CONTEXT_OUTPUT_RESERVATION_RATIO");
        // 8000 * 0.2 = 1600 → min(2048, 1600) = 1600
        assert_eq!(output_reservation(8000), 1600);
        // 100_000 * 0.2 = 20_000 → min(2048, 20_000) = 2048
        assert_eq!(output_reservation(100_000), 2048);
    }

    #[test]
    fn reservation_ratio_override_takes_effect() {
        let _g = ENV_LOCK.lock().unwrap();
        std::env::remove_var("MG_CONTEXT_OUTPUT_RESERVATION_TOKENS");
        std::env::set_var("MG_CONTEXT_OUTPUT_RESERVATION_RATIO", "0.5");
        // 4000 * 0.5 = 2000 → min(2048, 2000) = 2000
        assert_eq!(output_reservation(4000), 2000);
        std::env::remove_var("MG_CONTEXT_OUTPUT_RESERVATION_RATIO");
    }

    #[test]
    fn reservation_tokens_override_caps_value() {
        let _g = ENV_LOCK.lock().unwrap();
        std::env::set_var("MG_CONTEXT_OUTPUT_RESERVATION_TOKENS", "8192");
        std::env::remove_var("MG_CONTEXT_OUTPUT_RESERVATION_RATIO");
        // 100_000 * 0.2 = 20_000 → min(8192, 20_000) = 8192
        assert_eq!(output_reservation(100_000), 8192);
        std::env::remove_var("MG_CONTEXT_OUTPUT_RESERVATION_TOKENS");
    }

    #[test]
    fn invalid_ratio_falls_back_to_default() {
        let _g = ENV_LOCK.lock().unwrap();
        std::env::remove_var("MG_CONTEXT_OUTPUT_RESERVATION_TOKENS");
        // Out of range → ignored, fall back to 0.2.
        std::env::set_var("MG_CONTEXT_OUTPUT_RESERVATION_RATIO", "1.5");
        assert_eq!(output_reservation(10_000), 2000); // 10_000 * 0.2
        std::env::remove_var("MG_CONTEXT_OUTPUT_RESERVATION_RATIO");
    }
}
