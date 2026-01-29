use mawi_core::unified::ChatMessage;
use tracing::{info, warn};

/// Trim messages to fit context window (prevents 413 errors)
/// Keeps system prompt + recent history, drops old messages from middle
pub struct ContextManager;

impl ContextManager {
    /// Prune to context limit
    /// Heuristic: 4 chars ≈ 1 token, reserve ~20% for output
    pub fn prune_messages(messages: Vec<ChatMessage>, context_window: usize) -> Vec<ChatMessage> {
        if messages.is_empty() {
            return messages;
        }

        // leave room for model output (min 2048 or 20% of window)
        // TODO: make this configurable - code gen needs more, chat needs less
        let output_reservation = std::cmp::min(2048, context_window / 5);
        let effective_limit = context_window.saturating_sub(output_reservation);

        let total_estimated = Self::estimate_tokens(&messages);
        
        if total_estimated <= effective_limit {
            return messages;
        }

        warn!("✂️ context overflow: {} tokens > limit {} (window {}), pruning...", 
            total_estimated, effective_limit, context_window);

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
            info!("✂️ Pruned {} messages from middle of conversation.", dropped_count);
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
        content_tokens + 4  // role, JSON structure, etc
    }
}
