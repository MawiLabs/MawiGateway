use serde::{Serialize, Deserialize};
use std::collections::VecDeque;
use chrono::{DateTime, Local};

/// Represents a single memory entry (Task + Result)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub timestamp: DateTime<Local>,
    pub action: String,
    pub result: String,
    pub metadata: Option<String>,
}

/// Short-Term Memory (STM) system
/// Lightweight, in-memory, bounded capacity.
#[derive(Debug, Clone)]
pub struct ShortTermMemory {
    pub capacity: usize,
    pub entries: VecDeque<MemoryEntry>,
    pub enable_ltm: bool,
}

impl ShortTermMemory {
    /// Create a new STM with a fixed capacity
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            entries: VecDeque::with_capacity(capacity),
            enable_ltm: false, // Default to off
        }
    }

    /// Toggle Long-Term Memory (LTM) integration
    pub fn set_ltm(&mut self, enabled: bool) {
        self.enable_ltm = enabled;
    }

    /// Add a new entry to memory, evicting oldest if full
    pub fn add(&mut self, action: String, result: String) {
        if self.entries.len() >= self.capacity {
            self.entries.pop_front();
        }

        let entry = MemoryEntry {
            timestamp: Local::now(),
            action,
            result,
            metadata: None,
        };
        self.entries.push_back(entry);
    }

    /// Retrieve the current memory context formatted for the Planner LLM
    pub fn get_context(&self) -> String {
        let mut context = String::new();
        
        context.push_str("### SHORT-TERM MEMORY (Recent Actions):\n");
        if self.entries.is_empty() {
            context.push_str("(No recent actions recorded)\n");
        } else {
            for (i, entry) in self.entries.iter().enumerate() {
                context.push_str(&format!("{}. [{}] ACTION: {}\n   RESULT: {}\n", 
                    i + 1, 
                    entry.timestamp.format("%H:%M:%S"), 
                    entry.action, 
                    entry.result
                ));
            }
        }

        // LTM Hook (Stubbed)
        if self.enable_ltm {
            let ltm_context = self.retrieve_ltm_stub();
            if !ltm_context.is_empty() {
                context.push_str("\n### LONG-TERM MEMORY (Relevant Past):\n");
                context.push_str(&ltm_context);
                context.push_str("\n");
            }
        }

        context
    }

    /// Placeholder for LTM retrieval
    fn retrieve_ltm_stub(&self) -> String {
        // In a real implementation, this would query a vector DB
        // For now, we return a static stub or nothing
        String::new() 
    }
}

// Example usage tailored for the Planner LLM
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_lifecycle() {
        let mut stm = ShortTermMemory::new(3);
        
        stm.add("List files".to_string(), "file1.txt, file2.txt".to_string());
        stm.add("Read file1.txt".to_string(), "Hello World".to_string());
        stm.add("Count lines".to_string(), "1 line".to_string());
        
        // This should evict "List files"
        stm.add("Delete file2.txt".to_string(), "Deleted".to_string());

        let context = stm.get_context();
        println!("PLANNER CONTEXT:\n{}", context);
        
        assert!(!context.contains("List files"));
        assert!(context.contains("Delete file2.txt"));
    }
}
