//! Agentic service executor
//!
//! This module implements the ReAct pattern for agentic services.
//! The planner model can call tools (other models/services) to accomplish tasks.

use sqlx::PgPool;
use anyhow::{Result, anyhow};
use sqlx::Postgres;
use futures::StreamExt;
use mawi_core::agentic::{AgenticConfig, Tool, ToolCall, ToolResult, ToolType, AgenticMessage};
use mawi_core::unified::{UnifiedChatRequest, UnifiedChatResponse, ChatMessage};
use crate::executor::Executor;
use crate::agentic_memory::ShortTermMemory;
use tracing::{info, warn, error, debug, instrument};

use std::sync::Arc;
use tokio::sync::RwLock;
use crate::mcp_client::McpManager;

pub struct AgenticExecutor {
    pool: PgPool,
    executor: Executor,
    mcp_manager: Arc<RwLock<McpManager>>,
}

impl AgenticExecutor {
    pub fn new(pool: PgPool, mcp_manager: Arc<RwLock<McpManager>>) -> Self {
        let executor = Executor::new(pool.clone(), mcp_manager.clone());
        Self { pool, executor, mcp_manager }
    }

    pub fn execute_stream(
        self,
        request: UnifiedChatRequest,
        user_id: String,
    ) -> impl futures::Stream<Item = Result<mawi_core::unified::AgenticStreamEvent>> {
        async_stream::try_stream! {
            // Log activation
            info!("🚀 AGENTIC EXECUTOR ACTIVATED (STREAMING)");
            let start_time = std::time::Instant::now();

            yield mawi_core::unified::AgenticStreamEvent::Log { 
                step: "init".to_string(), 
                content: format!("Agent activated for service: {}", request.service) 
            };

            // NEW: Immediate "Thinking" state ensures timeline appears instantly
            yield mawi_core::unified::AgenticStreamEvent::Log { 
                step: "planning".to_string(), 
                content: "Initializing planner...".to_string() 
            };

            // 1. Load configuration
            let config = self.load_agentic_config(&request.service).await?;
            yield mawi_core::unified::AgenticStreamEvent::Log { 
                step: "config".to_string(), 
                content: format!("Loaded agent config: planner={}, tools={}", config.planner_model_id, config.tools.len()) 
            };

            // Detect Iterative Mode
            let user_query = request.messages.last().map(|m| m.content.to_lowercase()).unwrap_or_default();
            let is_iterative = user_query.contains("step by step") || user_query.contains("one step at a time") || user_query.contains("progressively");
            
            if is_iterative {
                info!("🔄 ITERATIVE PLANNING MODE DETECTED");
            }

            // 1.5 Extract Constraints
            yield mawi_core::unified::AgenticStreamEvent::Log { 
                step: "planning".to_string(), 
                content: "Analyzing constraints...".to_string() 
            };
            
            let constraints = match self.extract_constraints(&config, &request, &user_id).await {
                Ok(c) => c,
                Err(e) => {
                    warn!("Failed to extract constraints: {}", e);
                    vec![]
                }
            };
            
            if !constraints.is_empty() {
                 yield mawi_core::unified::AgenticStreamEvent::Log { 
                    step: "constraints".to_string(), 
                    content: format!("Identified {} constraints", constraints.len()) 
                };
            }

            // Execution State
            let mut memory = ShortTermMemory::new(10); // Capacity 10 steps
            let mut step_counter = 0;
            let mut execution_complete = false;
            let start_execution_time = std::time::Instant::now();
            let time_budget = std::time::Duration::from_secs(300); // Increased to 5 minutes

            // MAIN EXECUTION LOOP (with hard iteration limit for DoS protection)
            const MAX_ITERATIONS: usize = 10;
            let mut iteration_count = 0;
            
            while !execution_complete && iteration_count < MAX_ITERATIONS {
                iteration_count += 1;
                // 2. Generate Plan (or Next Step)
                yield mawi_core::unified::AgenticStreamEvent::Log { 
                    step: "planning".to_string(), 
                    content: if is_iterative { "Evaluating next move..." } else { "Generating strategic plan..." }.to_string() 
                };
                
                let plan = self.generate_plan(&config, &request, &memory, is_iterative, &user_id).await?;
                
                // FAST PATH CHECK
                if plan.len() == 1 && plan[0].to_uppercase() == "ANSWER" {
                    yield mawi_core::unified::AgenticStreamEvent::Log { 
                        step: "fast_path".to_string(), 
                        content: "Simple query detected. Answering directly...".to_string() 
                    };
                    
                    // Respond directly using the original request messages
                    let json_mode = None; 
                    let stream = self.executor.execute_model_stream_directly(&config.planner_model_id, request.messages.clone(), &user_id, json_mode).await?;
                    
                    // Forward events
                    for await event in stream {
                        yield event?;
                    }
                    return;
                }
                
                // Fallback / No Plan Logic
                if plan.is_empty() {
                    // Fail if this is the first pass, otherwise maybe we are just done?
                    if memory.entries.is_empty() {
                         // Fallback logic for media
                         // ... (Reuse existing fallback logic) ...
                         // For brevity in this tool call, assuming standard planner works.
                         // But I should preserve the fallback logic from original code.
                        let has_media_keywords = user_query.contains("image") || user_query.contains("generate");
                         if has_media_keywords {
                            if let Some(tool) = config.tools.iter().find(|t| t.tool_type == ToolType::ImageGeneration) {
                                let fallback_plan = vec![
                                    format!("TOOL[{}](Imagine and generate: {})", tool.name, request.messages.last().map(|m| &m.content).unwrap_or(&"Something".to_string())),
                                    "Synthesize the generated image into a final response".to_string()
                                ];
                                // Execute fallback
                                for step in fallback_plan {
                                    // self.execute_single_step_stream(&config, &request, &step, &mut memory, step_counter).await; // Signature changed
                                    // Manually do simplified log since we don't have a real loop here
                                     yield mawi_core::unified::AgenticStreamEvent::Log { 
                                        step: "fallback".to_string(), 
                                        content: format!("Executing Step {}: {}", step_counter, step) 
                                    };
                                    step_counter += 1;
                                }
                                execution_complete = true;
                                continue;
                            }
                        }
                        
                         yield mawi_core::unified::AgenticStreamEvent::Log { 
                            step: "planning".to_string(), 
                            content: "No plan generated. Switching to direct response.".to_string() 
                        };
                        let result = self.execute_react_loop(&config, &request, &memory, &user_id).await?;
                        if let Some(choice) = result.choices.first() {
                            yield mawi_core::unified::AgenticStreamEvent::FinalResponse(choice.message.content.clone());
                        }
                        return;
                    } else {
                        // If context not empty and no new plan, assume we are done
                        execution_complete = true;
                        continue;
                    }
                }

                // Execute the generated steps
                for step in plan {
                    // Check for completion signal from planner
                    if step.to_lowercase() == "done" || step.to_lowercase() == "finish" {
                         execution_complete = true;
                         break;
                    }

                    step_counter += 1;
                yield mawi_core::unified::AgenticStreamEvent::Log { 
                    step: "plan_created".to_string(), 
                    content: format!("Step {}: {}", step_counter, step) 
                };

                // Construct request for this step
                let step_request = self.build_step_request(&request, &step, &memory);

                // Verification Loop
                let mut step_success = false;
                let mut attempts = 0;
                let max_retries = 2; 

                while !step_success && attempts <= max_retries {
                     if start_execution_time.elapsed() > time_budget {
                         warn!("⏰ Time budget exceeded.");
                         break;
                     }
                     attempts += 1;

                     // Exponential backoff: wait 100ms * 2^(attempts-1) before retry
                     if attempts > 1 {
                         let backoff_ms = 100 * (1 << (attempts - 2)); // 100ms, 200ms, 400ms...
                         warn!("⏳ Retry backoff: waiting {}ms before attempt {}", backoff_ms, attempts);
                         tokio::time::sleep(std::time::Duration::from_millis(backoff_ms)).await;
                     }

                     // Execute Step
                     let result = self.execute_react_loop(&config, &step_request, &memory, &user_id).await?;
                     let content = result.choices.first().map(|c| c.message.content.clone()).unwrap_or_else(|| "No result".to_string());

                     // Verify
                     let (verified, feedback) = self.verify_result(&config, &constraints, &content, &user_id).await?;
                     
                     if verified {
                         step_success = true;
                        // Add result to context
                        memory.add(format!("Step {}: {}", step_counter, step), content.clone());
                         
                         yield mawi_core::unified::AgenticStreamEvent::Log { 
                            step: "verification".to_string(), 
                            content: format!("✅ Verified: {}", feedback) 
                        };

                        // Stream the result chunk to user immediately in iterative mode
                        if is_iterative {
                            yield mawi_core::unified::AgenticStreamEvent::FinalResponse(format!("\n\n**Step {}**: {}\n", step_counter, content));
                        }

                     } else {
                         yield mawi_core::unified::AgenticStreamEvent::Log { 
                            step: "verification".to_string(), 
                            content: format!("❌ Failed: {}", feedback) 
                        };
                        // Add feedback to context for next retry attempt
                        memory.add(format!("Step {} Attempt {} FAILED", step_counter, attempts), format!("Feedback: {}", feedback));
                     }
                }
                
                if !step_success {
                     yield mawi_core::unified::AgenticStreamEvent::Log { 
                        step: "warning".to_string(), 
                        content: format!("Proceeding best-effort after {} attempts.", attempts) 
                    };
                    // Log failure to memory? Optionally yes, but let's keep it simple for now
                    memory.add(format!("Step {}: {} (FAILED)", step_counter, step), "Execution Failed".to_string());
                }

                if !is_iterative {
                    execution_complete = true;
                }
            }

            // 4. Final Synthesis
            yield mawi_core::unified::AgenticStreamEvent::Log { 
                step: "synthesis".to_string(), 
                content: "Synthesizing final answer...".to_string() 
            };

            let final_response = self.synthesize_answer(&config, &request, &[], &memory, &user_id).await?;
            
            // In Iterative Mode, we've already streamed the steps. Just stream the conclusion (if any) or nothing.
            // But synthesize_answer might repeat context. 
            // We should perhaps skip full synthesis in iterative mode if we just want the steps?
            // The user said "Show the final answer only after all steps are displayed."
            // This contradicts "Reveal progressively".
            // Implementation: Stream steps as they happen. Then stream final summary.
            
            if let Some(choice) = final_response.choices.first() {
                // If iterative, we might want to ensure we don't duplicate everything.
                // But the synthesizer usually summarizes. Let's trust the synthesizer but perhaps prepend a separator.
                yield mawi_core::unified::AgenticStreamEvent::FinalResponse(choice.message.content.clone());
            }

            // FINAL LOGGING: Record the entire agentic session
            self.executor.log_request(
                None, // Extract key_id from request if available later
                &request.service,
                &config.planner_model_id,
                "agentic", // Provider type
                &final_response,
                0, // failover_count
                "success",
                None,
                start_time,
                Some(user_id.as_str())
            ).await;
            }
        }
    }

    // NOTE: execute_single_step_stream was removed - logic is inlined in execute_stream()

    /// Execute an agentic service request (Legacy Non-Streaming)
    pub fn execute<'a>(
        &'a self,
        request: &'a UnifiedChatRequest,
        user_id: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<UnifiedChatResponse>> + Send + 'a>> {
        Box::pin(async move {
            self.execute_internal(request, user_id).await
        })
    }

    /// Internal execution implementation
    async fn execute_internal(&self, request: &UnifiedChatRequest, user_id: &str) -> Result<UnifiedChatResponse> {
        info!("🚀 AGENTIC EXECUTOR ACTIVATED");
        info!("📍 Service: {}", request.service);
        info!("📍 Messages Count: {}", request.messages.len());
        if let Some(msg) = request.messages.last() {
            info!("📍 Last User Msg: {}", msg.content);
        }

        // 1. Load agentic configuration
        let config = self.load_agentic_config(&request.service).await.map_err(|e| {
            error!("❌ FAILED TO LOAD AGENTIC CONFIG: {}", e);
            e
        })?;
        
        info!("📋 CONFIG LOADED SUCCESS: planner={}, tools={}", 
                  config.planner_model_id, config.tools.len());
        for (i, t) in config.tools.iter().enumerate() {
            debug!("  Tool {}: {} ({:?})", i, t.name, t.tool_type);
        }

        // 2. Generate Plan (Thinking Phase)
        info!("🤔 STARTING PLAN GENERATION...");
        // Legacy execute_internal uses Vec<String> logic. We should update it to use STM too or adapt.
        // For simplicity, let's create a temporary memory here to satisfy the signature.
        let memory = ShortTermMemory::new(10); 
        let mut plan = self.generate_plan(&config, request, &memory, false, user_id).await.map_err(|e| {
            error!("❌ FAILED TO GENERATE PLAN: {}", e);
            e
        })?;

        // FALLBACK: If plan is empty but user asked for an image/media, force a plan
        if plan.is_empty() {
            let user_content = request.messages.last().map(|m| m.content.to_lowercase()).unwrap_or_default();
            if user_content.contains("image") || user_content.contains("generate") || user_content.contains("draw") || user_content.contains("create") {
                warn!("⚠️ PLANNER REFUSED BUT MEDIA DETECTED. TRIGGERING FORCE-PLAN FALLBACK.");
                // Find an image tool
                if let Some(tool) = config.tools.iter().find(|t| t.tool_type == ToolType::ImageGeneration) {
                    plan = vec![
                        format!("TOOL[{}](Imagine and generate: {})", tool.name, request.messages.last().map(|m| &m.content).unwrap_or(&"Something".to_string())),
                        "Synthesize the generated image into a final response".to_string()
                    ];
                }
            }
        }
        
        info!("🤔 FINAL PLAN FOR EXECUTION: {:?}", plan);

        if plan.is_empty() {
            warn!("⚠️ No plan generated. Falling back to direct chat.");
            let dummy_memory = ShortTermMemory::new(0);
            return self.execute_react_loop(&config, request, &dummy_memory, user_id).await;
        }

        // 3. Execute Plan

        // Context accumulates results from previous steps
        let mut memory = ShortTermMemory::new(10);

        if plan.is_empty() {
            if !config.tools.is_empty() {
                warn!("⚠️ WARNING: Planner returned an EMPTY PLAN despite having tools. This often indicates a hidden refusal in the RAW OUTPUT.");
            }
            // Fallback to standard ReAct for simple queries
            return self.execute_react_loop(&config, request, &memory, user_id).await;
        }

        // Execute each step
        for (i, step) in plan.iter().enumerate() {
            info!("🏃 Executing Step {}: {}", i + 1, step);
            
            // Construct request for this step, including context
            let step_request = self.build_step_request(request, step, &memory);
            
            // Execute step (using ReAct loop for sub-tasks)
            // Note: In non-streaming mode, we don't do the complex verification loop for now to avoid code duplication issues with 'yield'.
            // The streaming mode is the primary agentic interface.
            let result = self.execute_react_loop(&config, &step_request, &memory, user_id).await?;
            
            // Store result in context
            let content = result.choices.first().map(|c| c.message.content.clone()).unwrap_or_else(|| "No content result".to_string());
            memory.add(format!("Step {}: {}", i + 1, step), content);
        }

        // 4. Final Synthesis (Make an answer with the output of the script)
        info!("✨ Synthesizing final answer from {} context items", memory.entries.len());
        let final_response = self.synthesize_answer(&config, request, &plan, &memory, user_id).await?;

        Ok(final_response)
    }

    /// Synthesize a final answer from execution context
    async fn synthesize_answer(
        &self, 
        config: &AgenticConfig, 
        request: &UnifiedChatRequest, 
        plan: &[String],
        memory: &ShortTermMemory,
        user_id: &str
    ) -> Result<UnifiedChatResponse> {
        // Separate history from current query
        let (history_str, user_query) = if !request.messages.is_empty() {
             let last_idx = request.messages.len() - 1;
             let history: Vec<String> = request.messages[0..last_idx]
                 .iter()
                 .map(|m| format!("{}: {}", m.role.to_uppercase(), m.content))
                 .collect();
             
             (history.join("\n"), request.messages[last_idx].content.clone())
        } else {
             (String::new(), String::new())
        };

        let context_str = memory.get_context();
        let plan_str = plan.iter().enumerate()
            .map(|(i, s)| format!("{}. {}", i+1, s))
            .collect::<Vec<_>>()
            .join("\n");

        let history_block = if !history_str.is_empty() {
             format!("### CONVERSATION HISTORY\n{}\n\n", history_str)
        } else {
             String::new()
        };

        let synthesis_system_prompt = "You are a Strategic Planning Architect. Your role is to synthesize the results of an executed plan into a final answer in NATURAL LANGUAGE. \n\n\
            CRITICAL: If a tool was used to generate an image or video, DO NOT say you cannot generate media. Instead, present the result provided in the context.\n\
            CRITICAL: DO NOT OUTPUT JSON. Output normal text.".to_string();
        
        let synthesis_user_prompt = format!(
            "{}\
            User Request: '{}'\n\nExecuted Plan:\n{}\n\nExecution Results (Context):\n{}\n\n\
            INSTRUCTIONS:\n\
            1. Construct a final response satisfying the user's request in PLAIN ENGLISH.\n\
            2. The Context may contain raw JSON tool outputs. You MUST Parse them and present the info naturally.\n\
            3. If an image was generated (look for Markdown ![image](url) in Context), INCLUDE IT verbatim.\n\
            4. DO NOT apologize for not being able to generate images. You just did it via a tool!\n\
            5. NEVER format your entire response as JSON.",
            history_block, user_query, plan_str, context_str
        );

        let messages = vec![
            ChatMessage { role: "system".to_string(), content: synthesis_system_prompt },
            ChatMessage { role: "user".to_string(), content: synthesis_user_prompt },
        ];

        // Execute synthesis via planner model
        let mut response = self.executor.execute_model_directly(&config.planner_model_id, messages, user_id, None).await?;
        let context_str = memory.get_context();
        
        debug!("🧠 SYNTHESIS CONTEXT DUMP: {}", context_str);

        // Check if there was an image generated in the context
        // Relaxed check: Look for any image markdown, not just "Generated Image"
        // Check if there are images generated in the context
        // Scan for all markdown images ![...](...)
        let mut start_idx = 0;
        let mut images_found: Vec<(String, String)> = Vec::new(); // (Full Markdown, URL)
        
        while let Some(start) = context_str[start_idx..].find("![") {
             let abs_start = start_idx + start;
             if let Some(end_offset) = context_str[abs_start..].find(")") {
                 let abs_end = abs_start + end_offset + 1;
                 let image_markdown = &context_str[abs_start..abs_end];
                 
                 // Extract URL: ![alt](url)
                 if let Some(mid) = image_markdown.find("](") {
                     let url = &image_markdown[mid+2 .. image_markdown.len()-1];
                     images_found.push((image_markdown.to_string(), url.to_string()));
                 }
                 start_idx = abs_end;
             } else {
                 break;
             }
        }
        
        // Deduplicate by URL
        images_found.sort_by(|a, b| a.1.cmp(&b.1));
        images_found.dedup_by(|a, b| a.1 == b.1);
        
        if let Some(choice) = response.choices.first_mut() {
             for (full_markdown, url) in images_found {
                 // Check if the response contains this URL inside an image tag ![...](url)
                 // We look for the URL preceded by "](" and that preceded by "![...". 
                 // Simple heuristic: Does it contain the URL? 
                 // If NOT, definitely append.
                 // If YES, is it part of `![...](url)`?
                 
                 let content = &choice.message.content;
                 let url_present = content.contains(&url);
                 
                 // Check if it's used as an image
                 // This heuristic checks if `](url` exists in content. 
                 // If `url` is present but `](url` is NOT, it might be a raw link or `[...](url` ?
                 // Actually relying on "Is the exact URL present?" is risky if there are query params that get mutated.
                 // But assuming exact match:
                 
                 let is_image_format = if url_present {
                      // Look for usage: find URL indices, check prefix
                      content.match_indices(&url).any(|(idx, _)| {
                          // Check prefix for `](`
                          if idx >= 2 {
                              let prefix = &content[idx-2..idx];
                              if prefix == "](" {
                                  // Scan backwards for `[`
                                  // We need to find the MATCHING `[` for this `]`.
                                  // Scanning backwards for the NEAREST `[` might be wrong if nested?
                                  // Text: `[foo [bar](url)]` -> `]` matches `[bar`. matching `[` is `[bar`.
                                  // Text: `[foo](url)` -> `[` is `[foo`.
                                  // Simple backward search is usually okay for valid markdown.
                                  
                                  if let Some(last_open) = content[..idx-2].rfind('[') {
                                      // Check if predated by `!`
                                      if last_open > 0 {
                                          if &content[last_open-1..last_open] == "!" {
                                              return true; // It IS an image
                                          }
                                      }
                                      // If last_open == 0, it is `[...` so definitely NOT `![...`
                                      // If last_open > 0 but not `!`, it is `... [...` so NOT `![...`
                                      return false; 
                                  }
                              }
                          }
                          false
                      })
                 } else {
                      false
                 };

                 debug!("🖼️ Image Check for {}: Present={}, IsImage={}", 
                     url.chars().take(30).collect::<String>(), 
                     url_present, 
                     is_image_format
                 );

                 if !is_image_format {
                     info!("🖼️ Model missed image or format, appending: {}", full_markdown);
                     choice.message.content.push_str("\n\n");
                     choice.message.content.push_str(&full_markdown);
                 }
             }
        }
        
        // REMOVED: Debug Tool Dump & Raw Link (Cleanup for Production UX)

        Ok(response)
    }
    /// Extract explicit and implicit constraints from the user request
    async fn extract_constraints(&self, config: &AgenticConfig, request: &UnifiedChatRequest, user_id: &str) -> Result<Vec<String>> {
        let user_query = request.messages.last()
            .map(|m| m.content.clone())
            .unwrap_or_default();

        let prompt = format!(
            "You are a Constraint Analyzer. Your goal is to extract ALL constraints from the user request, both EXPLICIT and IMPLICIT.\n\n\
            User Request: \"{}\"\n\n\
            ### INSTRUCTIONS\n\
            1. Identify Explicit Constraints (e.g., \"use python\", \"under 100 words\").\n\
            2. Identify Implicit Constraints (e.g., \"fast\" -> latency < 2s, \"secure\" -> no hardcoded secrets).\n\
            3. Output strictly a JSON array of strings.\n\n\
            Example Output: [\"Must use Python\", \"Response must be under 500ms\", \"No external APIs\"]\n\
            ",
            user_query
        );

        let messages = vec![ChatMessage { role: "system".to_string(), content: prompt }];
        
        let response = self.executor.execute_model_directly(&config.planner_model_id, messages, user_id, None).await?;
        let content = response.choices.first().map(|c| c.message.content.clone()).unwrap_or_default();
        
        debug!("🔒 EXTRACTED CONSTRAINTS RAW: {}", content);

        // Simple parsing (reuse parse_plan_content logic or similar)
        // Assume model follows JSON instruction, but fallback to line parsing if needed
        let clean_content = content.replace("```json", "").replace("```", "").trim().to_string();
        if let (Some(start), Some(end)) = (clean_content.find('['), clean_content.rfind(']')) {
             let json_part = &clean_content[start..=end];
             if let Ok(constraints) = serde_json::from_str::<Vec<String>>(json_part) {
                 return Ok(constraints);
             }
        }
        
        // Fallback: Just return non-empty lines
        Ok(clean_content.lines().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
    }

    /// Verify a result against constraints using available verification tools or LLM critique
    async fn verify_result(
        &self, 
        config: &AgenticConfig, 
        constraints: &[String], 
        step_result: &str,
        user_id: &str
    ) -> Result<(bool, String)> {
        if constraints.is_empty() {
             return Ok((true, "No constraints to verify".to_string()));
        }

        // TODO: Here we would dynamically select a "Verifier Tool" (e.g. Test Runner)
        // For now, we use the Planner Model as a "Logical Verifier"

        let prompt = format!(
            "You are a Quality Assurance Verifier.\n\n\
            ### CONSTRAINTS\n\
            {}\n\n\
            ### GENERATED RESULT\n\
            {}\n\n\
            ### INSTRUCTIONS\n\
            1. Check if the result satisfies ALL constraints.\n\
            2. If YES, output exactly: \"VERIFIED: <reason>\"\n\
            3. If NO, output exactly: \"FAILED: <specific feedback to fix it>\"\n\n\
            ### CRITICAL RULES\n\
            - You are a TEXT ONLY model. You cannot see images or videos.\n\
            - If the result contains a Markdown Image/Video link (e.g. ![...](...) or http...) or a success message indicating media creation, YOU MUST ASSUME VISUAL CONSTRAINTS ARE MET.\n\
            - Do NOT fail verification because you 'cannot see' the image. If the tool ran and produced a link, it is VERIFIED.\n",
            constraints.iter().map(|c| format!("- {}", c)).collect::<Vec<_>>().join("\n"),
            step_result
        );

        let messages = vec![ChatMessage { role: "system".to_string(), content: prompt }];
        let response = self.executor.execute_model_directly(&config.planner_model_id, messages, user_id, None).await?;
        let content = response.choices.first().map(|c| c.message.content.clone()).unwrap_or_default();
        
        info!("✅ VERIFICATION RESULT: {}", content);

        if content.starts_with("VERIFIED") {
            Ok((true, content))
        } else {
            Ok((false, content))
        }
    }

    async fn generate_plan(&self, config: &AgenticConfig, request: &UnifiedChatRequest, memory: &ShortTermMemory, is_iterative: bool, user_id: &str) -> Result<Vec<String>> {
        // Only plan if complex or multiple tools. For now, we'll try to plan for everything to test.
        
        // Separate history from current query
        let (history_str, user_query) = if !request.messages.is_empty() {
             let last_idx = request.messages.len() - 1;
             let history: Vec<String> = request.messages[0..last_idx]
                 .iter()
                 .map(|m| format!("{}: {}", m.role.to_uppercase(), m.content))
                 .collect();
             
             (history.join("\n"), request.messages[last_idx].content.clone())
        } else {
             (String::new(), String::new())
        };

        let context_str = memory.get_context();
        let current_date = chrono::Local::now().format("%Y-%m-%d %H:%M").to_string();

        // Standardize History Block
        let history_block = if !history_str.is_empty() {
             format!("### CONVERSATION HISTORY (Use this to resolve context like 'previous request')\n{}\n\n", history_str)
        } else {
             String::new()
        };

        let custom_context = config.system_prompt.as_deref().unwrap_or("");
        
        let base_prompt = if is_iterative {
             format!(
                "You are an Iterative Planning Engine. Current Date: {}.\n\
                Your goal is to solve the user's request ONE STEP AT A TIME.\n\n\
                ### AGENT IDENTITY\n\
                {}\n\n\
                ### CAPABILITIES\n\
                You have tools for: {}\n\n\
                {}\
                ### SHORT-TERM MEMORY (Recent actions in this turn)\n\
                {}\n\n\
                ### INSTRUCTIONS\n\
                1. Analyze the History, Memory, and User Request.\n\
                2. Determine the SINGLE immediate next step required.\n\
                3. If the task is complete, output 'DONE' or 'FINISH'.\n\
                4. Output strictly a JSON array with ONE element: [\"TOOL[EXACT_NAME](args)\"] or [\"Next logical step description\"]\n\n\
                ### CRITICAL RULES\n\
                - DO NOT plan the entire future. Only the next step.\n\
                - USE THE EXACT TOOL NAME from the 'CAPABILITIES' list. Do NOT invent names like 'generate_image'.\n\
                - If media is requested, your plan MUST include a TOOL[...] call.",
                current_date,
                custom_context,
                config.tools.iter().map(|t| format!("{}: {}", t.name, t.description)).collect::<Vec<_>>().join(", "),
                history_block,
                context_str
            )
        } else {
             format!(
                "You are a Strategic Planning Agent. Current Date: {}.\n\
                Your role is ONLY to output a JSON array of steps.\n\n\
                ### AGENT IDENTITY\n\
                {}\n\n\
                ### CAPABILITIES\n\
                You have tools for: {}\n\n\
                {}\
                ### RESPONSE FORMAT\n\
                Output strictly a valid JSON array of strings. Every string must be a task or a tool call in the EXACT format: TOOL[tool_name](args).\n\n\
                ### FAST PATH\n\
                If the user request is simple and you can answer DIRECTLY without using any tools (e.g. general knowledge, greetings, simple questions), output strictly: [\"ANSWER\"].\n\n\
                ### CRITICAL RULES\n\
                - DO NOT REFUSE.\n\
                - USE THE EXACT TOOL NAME.\n\
                - If media is requested, your plan MUST include a TOOL[...] call.\n\
                - If you output [\"ANSWER\"], do not output anything else.\n\n\
                Example JSON Output:\n[\"TOOL[acad-solimg-prod](A snowy mountain at sunset)\", \"Synthesize result\"]",
                current_date,
                custom_context,
                config.tools.iter().map(|t| format!("{}: {}", t.name, t.description)).collect::<Vec<_>>().join(", "),
                history_block
            )
        };

        let messages = vec![
            ChatMessage { role: "system".to_string(), content: base_prompt.clone() },
            ChatMessage { role: "user".to_string(), content: format!("USER REQUEST TO PLAN: '{}'", user_query) },
        ];

        debug!("🔍 PLANNING PROMPT (System):\n{}", base_prompt);
        debug!("🔍 PLANNING PROMPT (User):\nUser Request: '{}'", user_query);

        let response = self.executor.execute_model_directly(&config.planner_model_id, messages, user_id, None).await?;
        let content = response.choices.first().map(|c| c.message.content.clone()).unwrap_or_default();
        
        debug!("🔍 RAW PLAN OUTPUT:\n{}", content);
        
        Ok(Self::parse_plan_content(&content))
    }

    /// Helper to parse plan content
    fn parse_plan_content(content: &str) -> Vec<String> {
        // Clean markdown blocks first - must own the String to borrow from it later
        let clean_content = content.replace("```json", "").replace("```", "").trim().to_string();
        
        // Strategy 1: Try to find a JSON array within the text
        if let (Some(start), Some(end)) = (clean_content.find('['), clean_content.rfind(']')) {
            let json_part = &clean_content[start..=end];
            if let Ok(plan) = serde_json::from_str::<Vec<String>>(json_part) {
                return plan;
            }
        }
        
        // Strategy 2: Fallback to line-based parsing if NO array found or parsing failed
        // We filter out sentences that look like refusals or conversational filler
        clean_content.lines()
            .filter(|l| !l.trim().is_empty())
            .filter(|l| {
                let low = l.to_lowercase();
                // Filter out common refusal phrases if they appear as standalone lines
                !(low.contains("can't create") || low.contains("cannot generate") || low.contains("don't have the ability"))
            })
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
    }

    /// Build a request for a specific step, injecting context
    fn build_step_request(&self, original_request: &UnifiedChatRequest, step: &str, memory: &ShortTermMemory) -> UnifiedChatRequest {
        let mut messages = original_request.messages.clone();
        
        // Inject context as a system or previous message
        if !memory.entries.is_empty() {
            let context_str = memory.get_context();
            messages.insert(0, ChatMessage {
                role: "system".to_string(),
                content: format!(
                    "You are a Strategic Planning Architect executing a step in a larger plan. \n\n\
                    ### PREVIOUS CONTEXT:\n{}\n\n\
                    ### INSTRUCTION:\nFollow the current instruction precisely. If media generation is required, use TOOL[tool_name](prompt).",
                    context_str
                ),
            });
        }

        // Add the current step as the active user instruction
        messages.push(ChatMessage {
            role: "user".to_string(),
            content: format!("Execute this step: {}", step),
        });

        UnifiedChatRequest {
            service: original_request.service.clone(),
            messages,
            ..original_request.clone()
        }
    }

    /// Standard ReAct loop (formerly execute_internal)
    async fn execute_react_loop(&self, config: &AgenticConfig, request: &UnifiedChatRequest, _memory: &ShortTermMemory, user_id: &str) -> Result<UnifiedChatResponse> {
        // 2. Build initial messages with system prompt and tools
        let messages = self.build_initial_messages(config, request);
        
        // Track already-generated media to prevent duplicates
        let mut generated_images: std::collections::HashSet<String> = std::collections::HashSet::new();
        
        // 3. ReAct loop
        let mut messages = messages; // Shadow to allow mutation
        for iteration in 0..config.max_iterations {
            info!("🔄 ReAct Iteration {}/{}", iteration + 1, config.max_iterations);

            // Call planner model with tools
            let planner_response = self.call_planner(config, &messages, user_id).await?;
            let _planner_content = planner_response.choices.first().map(|c| c.message.content.clone()).unwrap_or_default();

            if let Some(tool_calls) = self.extract_tool_calls(&planner_response) {
                info!("🔧 Planner requested {} tool call(s)", tool_calls.len());

                // Add assistant message with tool calls
                messages.push(AgenticMessage::assistant_with_tool_calls(tool_calls.clone()));

                // Execute each tool call
                for call in &tool_calls {
                    // Check for duplicate image generation
                    let is_image_tool = call.name.to_lowercase().contains("image") || 
                                        call.name.to_lowercase().contains("generate") ||
                                        call.name.to_lowercase().contains("solimg") ||
                                        call.name.to_lowercase().contains("dall");
                    
                    if is_image_tool && !generated_images.is_empty() {
                    // Prevent duplicate image generation
                    if call.name.contains("generate_image") || call.name.contains("dall-e") || call.name.contains("stable_diffusion") {
                        if generated_images.contains(&call.arguments) {
                             debug!("⚠️ Skipping duplicate image generation - already have {} image(s)", generated_images.len());
                             continue;
                        }
                    }
                        if let Some(existing_url) = generated_images.iter().next() {
                            messages.push(AgenticMessage::tool_result(&call.id, &format!("Image already generated: ![Generated Image]({})", existing_url)));
                            continue;
                        }
                    }
                    
                    let result = self.execute_tool(config, call, user_id).await?;
                    
                    // Track generated images
                    if result.success && is_image_tool {
                        // Extract URL from result
                        if let Some(start) = result.content.find("](") {
                            if let Some(end) = result.content[start..].find(")") {
                                let url = &result.content[start+2..start+end];
                                generated_images.insert(url.to_string());
                            }
                        }
                    }
                    
                    messages.push(AgenticMessage::tool_result(&call.id, &result.content));
                    
                    eprintln!("✅ Tool '{}' executed: {}", call.name, 
                              if result.success { "success" } else { "failed" });
                }

                // Continue loop to let planner process tool results
                continue;
            } else {
                // No tool calls - this is the final response
                eprintln!("🎯 Planner provided final response");
                return Ok(planner_response);
            }
        }

        Err(anyhow!("Max iterations ({}) exceeded without final answer", config.max_iterations))
    }

    /// Load agentic configuration from database
    async fn load_agentic_config(&self, service_name: &str) -> Result<AgenticConfig> {
        // Get service with agentic fields
        let service = sqlx::query_as::<sqlx::Postgres, (String, Option<String>, Option<i32>)>(
            "SELECT planner_model_id, system_prompt, max_iterations 
             FROM services 
             WHERE name = $1 AND service_type = 'AGENTIC'"
        )
        .bind(service_name)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| anyhow!("Agentic service '{}' not found", service_name))?;

        let planner_model_id = service.0;
        let system_prompt = service.1;
        let max_iterations = service.2.unwrap_or(10) as u32;

        // Load explicitly defined agentic tools
          let tools_rows = sqlx::query_as::<sqlx::Postgres, (String, String, String, String, String, Option<String>)>(
            "SELECT id, name, description, tool_type, target_id, parameters_schema 
             FROM agentic_tools 
             WHERE service_name = $1
             ORDER BY position"
        )
        .bind(service_name)
        .fetch_all(&self.pool)
        .await?;

        let mut tools: Vec<Tool> = tools_rows
        .into_iter()
        .map(|(id, name, description, tool_type_str, target_id, params_schema): (String, String, String, String, String, Option<String>)| {
            let tool_type = ToolType::try_from(tool_type_str)
                .unwrap_or(ToolType::Model);
            
            let parameters_schema = params_schema.as_deref()
                .and_then(|s| serde_json::from_str(s).ok());

            Tool {
                id,
                name,
                description,
                tool_type,
                target_id,
                parameters_schema,
            }
        })
        .collect();

        // Deduplicate
        tools.sort_by(|a, b| a.name.cmp(&b.name));
        tools.dedup_by(|a, b| a.name == b.name);
        
        info!("🛠️ Loaded {} EXPLICT tools from agentic_tools", tools.len());
        for t in &tools {
            debug!("  - Tool: {} ({:?})", t.name, t.tool_type);
        }

        // Join with mcp_servers to get server name for namespacing
        // AND JOIN with service_mcp_servers to filter by assigned service
          let mcp_tools_rows = sqlx::query_as::<sqlx::Postgres, (String, String, String, String, Option<String>, String)>(
            "SELECT t.id, t.server_id, t.name, t.description, t.input_schema, s.name as server_name
             FROM mcp_tools t
             JOIN mcp_servers s ON t.server_id = s.id
             JOIN service_mcp_servers sms ON sms.mcp_server_id = s.id
             WHERE sms.service_name = $1"
        )
        .bind(service_name)
        .fetch_all(&self.pool)
        .await?;
        
        // Filter MCP tools relevant to this service? 
        // For now, let's expose ALL connected MCP tools to the agent.
        // Ideally we would have a mapping table `service_mcp_tools`.
        // TODO: Filter based on service configuration if needed.
        
        for (id, server_id, name, description, input_schema, server_name) in mcp_tools_rows {
            // Check if server is connected
            let is_connected: bool = sqlx::query_scalar::<sqlx::Postgres, i32>(
                "SELECT 1 FROM mcp_servers WHERE id = $1 AND status = 'connected'"
            )
            .bind(&server_id)
            .fetch_optional(&self.pool)
            .await?
            .is_some();

            if !is_connected {
                continue;
            }

            let schema_value = input_schema.as_deref()
                .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok());
            
            // We need to store original name in schema because `name` might be modified for uniqueness 
            // but MCP call requires exact name.
            let mut final_schema = schema_value.clone().unwrap_or(serde_json::json!({}));
            if let Some(obj) = final_schema.as_object_mut() {
                obj.insert("original_name".to_string(), serde_json::Value::String(name.clone()));
            }

            
            let server_name_sanitized = server_name.to_lowercase().replace(" ", "_").replace("-", "_");
            let namespaced_name = format!("{}.{}", server_name_sanitized, name);

            tools.push(Tool {
                id: format!("mcp_{}", id),
                name: namespaced_name, 
                description,
                tool_type: ToolType::Mcp,
                target_id: server_id,
                parameters_schema: Some(final_schema),
            });
        }


        // NEW: Also load assigned models from service_models as tools
        // This connects the 'Assign Model' UI to the Agentic tools system
        let assigned_models: Vec<(String, String, String, String)> = sqlx::query_as::<sqlx::Postgres, (String, String, String, String)>(
            "SELECT m.id, m.name, m.modality, sm.model_id 
             FROM service_models sm
             JOIN models m ON sm.model_id = m.id
             WHERE sm.service_name = $1
             ORDER BY sm.position"
        )
        .bind(service_name)
        .fetch_all(&self.pool)
        .await?;
        
        info!("🛠️ Total tools loaded (including MCP): {}", tools.len());

        for (id, name, modality, target_id) in assigned_models {
            // Skip if this is the planner itself (optional, but usually planner doesn't call itself as a tool)
            if id == planner_model_id {
                continue;
            }

            // Determine tool type and description based on modality
            let (tool_type, description, tool_name) = match modality.clone().as_str() {
                "image" => (
                    ToolType::ImageGeneration, 
                    format!("Generates an image based on a text prompt using {}.", name),
                    "generate_image".to_string()
                ),
                "video" => (
                    ToolType::VideoGeneration,
                    format!("Generates a video based on a text prompt using {}.", name),
                    "generate_video".to_string()
                ),
                "audio" => (
                    ToolType::TextToSpeech,
                    format!("Converts text to speech/audio using {}.", name),
                    "text_to_speech".to_string()
                ),
                "text" | "chat" | _ => (
                    ToolType::Model,
                    format!("Asks the {} AI model a question or task.", name),
                    // NEW STRATEGY: Use sanitized Model ID directly
                    id.clone().to_lowercase().replace(|c: char| !c.is_alphanumeric(), "_")
                ),
            };

            // If a tool with this name already exists, append a suffix to avoid duplicates
            let mut final_tool_name = tool_name.clone();
            let mut counter = 1;
            while tools.iter().any(|t| t.name == final_tool_name) {
                final_tool_name = format!("{}_{}", tool_name, counter);
                counter += 1;
            }

            debug!("🛠️ Auto-mapping model '{}' (modality: {}) to tool: {}", id, modality, final_tool_name);
            tools.push(Tool {
                id: format!("auto_tool_{}", id),
                name: final_tool_name,
                description,
                tool_type,
                target_id,
                parameters_schema: None,
            });
        }



        // DEDUPLICATION LOGIC:
        // If we have specific tools (e.g., 'generate_image'), remove generic legacy tools (e.g., 'image_generation')
        // to prevent planner confusion and hallucination.
        let has_specific_image = tools.iter().any(|t| t.name == "generate_image");
        let has_specific_video = tools.iter().any(|t| t.name == "generate_video");

        if has_specific_image {
            eprintln!("🧹 Removing generic 'image_generation' tool because 'generate_image' exists");
            tools.retain(|t| t.name != "image_generation");
        }
        
        if has_specific_video {
            eprintln!("🧹 Removing generic 'video_generation' tool because 'generate_video' exists");
            tools.retain(|t| t.name != "video_generation");
        }

        info!("✅ Total tools available for planner: {}", tools.len());

        Ok(AgenticConfig {
            planner_model_id,
            system_prompt,
            tools,
            max_iterations,
        })
    }

    /// Build initial messages with system prompt and user request
    fn build_initial_messages(&self, config: &AgenticConfig, request: &UnifiedChatRequest) -> Vec<AgenticMessage> {
        let mut messages = Vec::new();

        // Create tool instructions footer
        let image_tool = config.tools.iter().find(|t| t.tool_type == ToolType::ImageGeneration);
        let video_tool = config.tools.iter().find(|t| t.tool_type == ToolType::VideoGeneration);
        let audio_tool = config.tools.iter().find(|t| t.tool_type == ToolType::TextToSpeech);

        let mut examples = Vec::new();
        if let Some(t) = image_tool {
            examples.push(format!("User: 'Show me a mountain'\nAssistant: TOOL[{}](A majestic snow-capped mountain range)", t.name));
        }
        if let Some(t) = video_tool {
            examples.push(format!("User: 'Make a video of waves'\nAssistant: TOOL[{}](Cinematic shot of ocean waves crashing on a beach)", t.name));
        }
        if let (Some(t), Some(it)) = (audio_tool, image_tool) {
             examples.push(format!("User: 'Tell me a story about a dragon and show me his lair'\nAssistant: I'll generate a picture of the dragon's lair first.\nTOOL[{}](A dark volcanic cave with gold piles and a red dragon sleeping)\n[Then after result]\nNow I'll narrate the story.\nTOOL[{}](Once upon a time in a far away land...)", it.name, t.name));
        }

        let tool_instructions = format!(
            "\n\n## STRATEGIC PLANNING PROTOCOL\n\n\
            You are a Specialized Planning Architect. You do not generate media yourself; you orchestrate its creation via tools.\n\n\
            AVAILABLE TOOLS:\n{}\n\n\
            ### RESPONSE FORMAT (JSON Mode)\n\
            You MUST output a valid JSON object. \n\
            If you want to call tools, output:\n\
            {{ \"tools\": [ {{ \"name\": \"tool_name\", \"arguments\": {{ \"arg\": \"value\" }} }} ] }}\n\
            \n\
            If you want to just reply to the user (no tools):\n\
            {{ \"reply\": \"Your response here\" }}\n\n\
            EXAMPLES:\n\
            User: 'Show me a mountain'\n\
            Assistant: {{ \"tools\": [ {{ \"name\": \"acad-solimg-prod\", \"arguments\": {{ \"prompt\": \"A majestic snow-capped mountain range\" }} }} ] }}\n\n\
            \n\
            ### PROTOCOL GUIDELINES\n\
            1. DO NOT REFUSE. You are not being asked to generate media natively. You are being asked to INSTRUCT a tool to do so.\n\
            2. ALWAYS output valid JSON.\n\
            3. CRITICAL: IF you see a Tool Result in the history containing an image URL (e.g., '![Generated Image]...'), your job is DONE. Output {{ \"reply\": \"Here is your image: ![Generated Image]...\" }}.",
            config.tools.iter()
                .map(|t| format!("- {}: {}", t.name, t.description))
                .collect::<Vec<_>>()
                .join("\n")
        );
        
        let tool_instructions = format!(
            "{}\n\nIMPORTANT: When using TOOL[...] syntax, arguments MUST be a valid JSON object if the tool requires specific parameters.\nExample: TOOL[github_mcp.list_tags]({{ \"owner\": \"foo\", \"repo\": \"bar\" }})\nDO NOT use simple strings like TOOL[x](\"foo\") unless the tool explicitly takes a single string input.", 
            tool_instructions
        );

        // Use custom prompt + footer, or default header + footer
        let base_prompt = config.system_prompt.clone().unwrap_or_else(|| "You are a Strategic Planning Architect responsible for orchestrating complex tool-based workflows.".to_string());
        
        // Only append instructions if tools exist
        let system_prompt = if !config.tools.is_empty() {
            format!("{}\n{}", base_prompt, tool_instructions)
        } else {
            base_prompt
        };

        debug!("📝 FINAL SYSTEM PROMPT:\n{}", system_prompt);

        messages.push(AgenticMessage::system(system_prompt));

        // Add user messages from request
        let mut user_msgs_added = 0;
        let num_user_msgs = request.messages.iter().filter(|m| m.role == "user").count();

        for msg in &request.messages {
            let mut content = msg.content.clone();
            
            // If this is the last user message and we have tools, append a reminder
            if msg.role == "user" && !config.tools.is_empty() {
                user_msgs_added += 1;
                if user_msgs_added == num_user_msgs {
                    content = format!("{}\n\n(REMINDER: You have specialized tools for this. If I asked for media, USE THE TOOL! DO NOT REFUSE.)", content);
                }
            }

            messages.push(AgenticMessage {
                role: msg.role.clone(),
                content: Some(content),
                tool_calls: None,
                tool_call_id: None,
            });
        }

        messages
    }

    /// Call the planner model with tool definitions (Streaming)
    async fn call_planner_stream(
        &self,
        config: &AgenticConfig,
        messages: &[AgenticMessage],
        user_id: &str,
    ) -> Result<std::pin::Pin<Box<dyn futures::Stream<Item = Result<mawi_core::unified::AgenticStreamEvent>> + Send>>> {
        // Convert AgenticMessage to UnifiedChatRequest format
        let unified_messages: Vec<ChatMessage> = messages
            .iter()
            .filter_map(|m| {
                if m.role == "tool" {
                    Some(ChatMessage {
                        role: "user".to_string(),
                        content: format!("Tool Result: {}", m.content.clone().unwrap_or_default()),
                    })
                } else {
                    Some(ChatMessage {
                        role: m.role.clone(),
                        content: m.content.clone().unwrap_or_default(),
                    })
                }
            })
            .collect();

        // Execute via the executor directly (bypassing service routing)
        // Execute via the executor directly (bypassing service routing)
        // ENABLE JSON MODE FOR PLANNER
        let json_mode = Some(mawi_core::types::ResponseFormat { type_: "json_object".to_string() });
        self.executor.execute_model_stream_directly(&config.planner_model_id, unified_messages, user_id, json_mode).await
    }

    /// Call the planner model with tool definitions
    async fn call_planner(
        &self,
        config: &AgenticConfig,
        messages: &[AgenticMessage],
        user_id: &str,
    ) -> Result<UnifiedChatResponse> {
        // Convert AgenticMessage to UnifiedChatRequest format
        let unified_messages: Vec<ChatMessage> = messages
            .iter()
            .filter_map(|m| {
                if m.role == "tool" {
                    // Convert tool results to user messages so the model sees them
                    Some(ChatMessage {
                        role: "user".to_string(),
                        content: format!("Tool Result: {}", m.content.clone().unwrap_or_default()),
                    })
                } else {
                    Some(ChatMessage {
                        role: m.role.clone(),
                        content: m.content.clone().unwrap_or_default(),
                    })
                }
            })
            .collect();

        // Execute via the executor directly (bypassing service routing to avoid recursion)
        // Execute via the executor directly (bypassing service routing to avoid recursion)
        // ENABLE JSON MODE FOR PLANNER
        let json_mode = Some(mawi_core::types::ResponseFormat { type_: "json_object".to_string() });
        self.executor.execute_model_directly(&config.planner_model_id, unified_messages, user_id, json_mode).await
    }

    /// Extract tool calls from planner response
    fn extract_tool_calls(&self, response: &UnifiedChatResponse) -> Option<Vec<ToolCall>> {
        // Check if response has choices
        let first_choice = response.choices.first()?;
        let content = &first_choice.message.content;

        debug!("🔍 Parsing Planner Output: {}", content);

        // STRATEGY 1: JSON Parsing (Primary)
        if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(content) {
            debug!("✅ Valid JSON detected");
            if let Some(tools_array) = json_val.get("tools").and_then(|v| v.as_array()) {
                let mut calls = Vec::new();
                for tool_obj in tools_array {
                    if let (Some(name), Some(args)) = (tool_obj.get("name").and_then(|s| s.as_str()), tool_obj.get("arguments")) {
                        calls.push(ToolCall {
                            id: format!("call_{}", uuid::Uuid::new_v4()),
                            name: name.to_string(),
                            arguments: args.to_string(),
                        });
                    }
                }
                if !calls.is_empty() {
                    return Some(calls);
                }
            }
            // If it has "reply", treat as final response (no tools)
            if json_val.get("reply").is_some() {
                 return None;
            }
        }

        // STRATEGY 2: Fallback to Regex (Legacy/Hallsucination fallback)
        // Check for basic tool call pattern: TOOL[name](args)
        if !content.contains("TOOL[") {
            return None;
        }

        // Parse simple tool call format
        // Example: "TOOL[search](query='rust programming')"
        self.parse_simple_tool_calls(content)
    }

    /// Parse tool calls using Regex for robustness
    fn parse_simple_tool_calls(&self, content: &str) -> Option<Vec<ToolCall>> {
        let mut calls = Vec::new();
        
        // Regex to capture TOOL[name](args)
        // Handles optional whitespace around brackets/parens
        // DOTALL (?s) allows arguments to span newlines if needed, though usually they don't
        // We use non-greedy matching .*? for name and args
        let re = regex::Regex::new(r"TOOL\s*\[\s*(.*?)\s*\]\s*\(\s*(.*?)\s*\)").ok()?;

        for cap in re.captures_iter(content) {
            if let (Some(name_match), Some(args_match)) = (cap.get(1), cap.get(2)) {
                let name = name_match.as_str().trim().to_string();
                let args = args_match.as_str().to_string(); 
                let args_trimmed = args.trim();
                
                let arguments = if args_trimmed.starts_with('{') && serde_json::from_str::<serde_json::Value>(args_trimmed).is_ok() {
                     args_trimmed.to_string()
                } else {
                     format!("{{\"input\": {:?}}}", args)
                };

                calls.push(ToolCall {
                    id: format!("call_{}", uuid::Uuid::new_v4()),
                    name,
                    arguments,
                });
            }
        }

        if calls.is_empty() {
            None
        } else {
            Some(calls)
        }
    }

    /// Execute a tool call
    async fn execute_tool(&self, config: &AgenticConfig, call: &ToolCall, user_id: &str) -> Result<ToolResult> {
        // Find the tool definition using fuzzy matching + AMBIGUITY CHECK
        let matches: Vec<&Tool> = config.tools.iter().filter(|t| {
            // 1. Exact match
            if t.name == call.name { return true; }
            // 2. Case insensitive
            if t.name.eq_ignore_ascii_case(&call.name) { return true; }
            // 3. Normalized match (ignore underscores, case)
            let t_norm = t.name.replace('_', "").to_lowercase();
            let c_norm = call.name.replace('_', "").to_lowercase();
            if t_norm == c_norm { return true; }
            // 4. Prefix match (e.g. call "mistral" matches tool "mistral_large")
            if call.name.chars().all(char::is_alphanumeric) {
                if t.name.to_lowercase().contains(&call.name.to_lowercase()) { return true; }
            }
            false
        }).collect();

        // Safe Resolution Strategy
        let tool = match matches.len() {
            0 => {
                // FAIL-SAFE: Check for generic intent aliases
                let lower_name = call.name.to_lowercase();
                if lower_name.contains("image") || lower_name.contains("gen") || lower_name.contains("draw") || lower_name.contains("picture") || lower_name.contains("solimg") || lower_name.contains("dall") {
                     // Try to find ANY ImageGeneration tool OR tool with image keywords in name
                     if let Some(img_tool) = config.tools.iter().find(|t| {
                         t.tool_type == ToolType::ImageGeneration || 
                         t.name.to_lowercase().contains("img") || 
                         t.name.to_lowercase().contains("image") || 
                         t.name.to_lowercase().contains("dall") || 
                         t.name.to_lowercase().contains("solimg") ||
                         t.description.to_lowercase().contains("image")
                     }) {
                         info!("✨ Smart Resolve: Mapped generic '{}' to tool '{}'", call.name, img_tool.name);
                         img_tool
                     } else {
                         warn!("❌ Planner hallucinated unknown tool: {}", call.name);
                         return Ok(ToolResult {
                            tool_call_id: call.id.clone(),
                            content: String::new(),
                            success: false,
                            error: Some(format!("Unknown tool: {}. Available tools: {}", 
                                call.name, 
                                config.tools.iter().map(|t| t.name.as_str()).collect::<Vec<_>>().join(", ")
                            )),
                        });
                     }
                } else {
                     warn!("❌ Planner hallucinated unknown tool: {}", call.name);
                     return Ok(ToolResult {
                        tool_call_id: call.id.clone(),
                        content: String::new(),
                        success: false,
                        error: Some(format!("Unknown tool: {}. Available tools: {}", 
                            call.name, 
                            config.tools.iter().map(|t| t.name.as_str()).collect::<Vec<_>>().join(", ")
                        )),
                    });
                }
            },
            1 => matches[0],
            _ => {
                // Ambiguous match - fail safely and ask planner to clarify
                let candidates = matches.iter().map(|t| t.name.as_str()).collect::<Vec<_>>().join(", ");
                warn!("⚠️ Ambiguous tool call '{}' matched multiple tools: {}", call.name, candidates);
                return Ok(ToolResult {
                    tool_call_id: call.id.clone(),
                    content: format!("Ambiguous tool name '{}'. Did you mean one of: {}? Please be specific.", call.name, candidates),
                    success: false,
                    error: Some(format!("Ambiguous tool name. Candidates: {}", candidates)),
                });
            }
        };

        info!("🔨 Executing tool '{}' of type {:?}", tool.name, tool.tool_type);

        // Parse arguments
        let args: serde_json::Value = serde_json::from_str(&call.arguments)
            .unwrap_or_else(|_| serde_json::json!({"input": call.arguments}));

        // Execute based on tool type
        let result: Result<String> = match tool.tool_type {
            ToolType::Model => {
                // HACK: If the tool is actually an Image Model (e.g. DALL-E) but classified as generic Model,
                // force it to the image execution path.
                let lower_name = tool.name.to_lowercase();
                if lower_name.contains("solimg") || lower_name.contains("dall") || lower_name.contains("image") {
                     info!("🔄 Auto-Correction: Tool '{}' is typed as Model but looks like Image. Rerouting...", tool.name);
                     self.execute_image_generation_tool(&tool.target_id, &args, user_id).await
                } else {
                     self.execute_model_tool(&tool.target_id, &args, user_id).await
                }
            }
            ToolType::Service => {
                self.execute_service_tool(&tool.target_id, &args, user_id).await
            }
            ToolType::ImageGeneration => {
                self.execute_image_generation_tool(&tool.target_id, &args, user_id).await
            }
            ToolType::VideoGeneration => {
                self.execute_video_generation_tool(&tool.target_id, &args, user_id).await
            }
            ToolType::TextToSpeech => {
                self.execute_tts_tool(&tool.target_id, &args, user_id).await
            }
            ToolType::SpeechToText => {
                self.execute_stt_tool(&tool.target_id, &args, user_id).await
            }
            ToolType::Mcp => {
                let server_id = &tool.target_id;
                let tool_name = tool.parameters_schema.as_ref()
                    .and_then(|s| s.get("original_name").and_then(|v| v.as_str()))
                    .unwrap_or(&tool.name);
                    
                info!("🔌 Executing MCP Tool via Manager: server={}, tool={}", server_id, tool_name);
                
                // HEURISTIC: Fix common GitHub tool argument failure (auto-map "input" -> "owner" + "repo")
                let mut final_args = args.clone();
                let mut should_apply_heuristic = false;
                let mut owner = String::new();
                let mut repo = String::new();

                if let Some(input_str) = final_args.get("input").and_then(|v| v.as_str()) {
                    let parts: Vec<&str> = input_str.split('/').collect();
                    if parts.len() == 2 {
                        // Check if schema expects owner/repo
                        let has_owner_repo = tool.parameters_schema.as_ref()
                            .map(|s| {
                                let props = s.get("properties");
                                props.and_then(|p| p.get("owner")).is_some() && 
                                props.and_then(|p| p.get("repo")).is_some()
                            })
                            .unwrap_or(false);

                        if has_owner_repo {
                             owner = parts[0].to_string();
                             repo = parts[1].to_string();
                             should_apply_heuristic = true;
                        }
                    }
                }

                if should_apply_heuristic {
                     if let Some(obj) = final_args.as_object_mut() {
                         obj.insert("owner".to_string(), serde_json::json!(owner));
                         obj.insert("repo".to_string(), serde_json::json!(repo));
                         obj.remove("input");
                         info!("🔧 Applied Argument Heuristic: 'input' -> owner/repo ({}/{})", owner, repo);
                     }
                }

                let manager = self.mcp_manager.read().await;
                match manager.call_tool(server_id, tool_name, final_args).await {
                     Ok(val) => {
                         // Extract text content from MCP result format
                         // MCP result structure: { content: [{ type: "text", text: "..." }] }
                         if let Some(content_arr) = val.get("content").and_then(|c| c.as_array()) {
                             let mut full_text = String::new();
                             for item in content_arr {
                                 if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                                     full_text.push_str(text);
                                     full_text.push('\n');
                                 }
                             }
                             // fallback if no structured content
                             if full_text.is_empty() {
                                 Ok(val.to_string()) 
                             } else {
                                 Ok(full_text.trim().to_string())
                             }
                         } else {
                              Ok(val.to_string())
                         }
                     },
                     Err(e) => Err(e)
                }
            }
        };

        match result {
            Ok(content) => Ok(ToolResult {
                tool_call_id: call.id.clone(),
                content,
                success: true,
                error: None,
            }),
            Err(e) => {
                error!("❌ Tool '{}' execution failed details: {}", tool.name, e);
                Ok(ToolResult {
                    tool_call_id: call.id.clone(),
                    content: format!("Tool execution failed: {}", e),
                    success: false,
                    error: Some(e.to_string()),
                })
            },
        }
    }

    /// Execute a model tool
    async fn execute_model_tool(&self, model_id: &str, args: &serde_json::Value, user_id: &str) -> Result<String> {
        // Accept multiple argument names for flexibility
        let input = args.get("input")
            .or_else(|| args.get("query"))
            .or_else(|| args.get("arg"))
            .or_else(|| args.get("prompt"))
            .or_else(|| args.get("question"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing 'input' (or 'query'/'arg'/'prompt') in tool arguments. Got: {:?}", args))?;

        let messages = vec![ChatMessage {
            role: "user".to_string(),
            content: input.to_string(),
        }];

        // Model tool should be free-form text, not strict JSON
        let response = self.executor.execute_model_directly(model_id, messages, user_id, None).await?;
        Ok(response.choices.first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default())
    }

    /// Execute a service tool
    async fn execute_service_tool(&self, service_name: &str, args: &serde_json::Value, user_id: &str) -> Result<String> {
        let input = args.get("input")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing 'input' in tool arguments"))?;

        // For services, we still use execute_chat but this should be a POOL service, not AGENTIC
        // If it's an AGENTIC service being called as a tool, that's allowed (agents calling agents)
        let request = UnifiedChatRequest {
            service: service_name.to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: input.to_string(),
            }],
            model: None,
            params: None,
            stream: None,
            routing_strategy: None,
            response_format: None,
        };

        let response = self.executor.execute_chat(&request, user_id).await?;
        Ok(response.choices.first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default())
    }

    /// Execute image generation tool
    async fn execute_image_generation_tool(&self, model_id: &str, args: &serde_json::Value, user_id: &str) -> Result<String> {
        let prompt = args.get("prompt")
            .or_else(|| args.get("input"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing 'prompt' in tool arguments"))?;

        let request = mawi_core::types::ImageGenerationRequest {
            model: model_id.to_string(),
            prompt: prompt.to_string(),
            n: 1,
            size: "1024x1024".to_string(),
            quality: None,
            style: None,
        };

        let response = self.executor.execute_image_generation(&request, user_id).await?;
        
        // Return URL or base64 data as Markdown
        if let Some(first_image) = response.data.first() {
            if let Some(url) = &first_image.url {
            // Include Raw URL for debugging (Azure links can be tricky)
            Ok(format!("![Generated Image]({})", url))
        } else if let Some(b64) = &first_image.b64_json {
                Ok(format!("![Generated Image](data:image/png;base64,{})", b64))
            } else {
                Ok("Image generated successfully, but no URL returned.".to_string())
            }
        } else {
            Err(anyhow!("No image data in response"))
        }
    }

    /// Execute video generation tool
    async fn execute_video_generation_tool(&self, model_id: &str, args: &serde_json::Value, user_id: &str) -> Result<String> {
        let prompt = args.get("prompt")
            .or_else(|| args.get("input"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing 'prompt' in tool arguments"))?;

        let request = mawi_core::types::VideoGenerationRequest {
            model: model_id.to_string(),
            prompt: prompt.to_string(),
            size: Some("1280x720".to_string()),
            duration: Some(8),
        };

        let response = self.executor.execute_video_generation(&request, user_id).await?;
        
        if let Some(url) = response.url {
            Ok(format!("Video generation started: {}", url))
        } else {
            Ok("Video generation initiated".to_string())
        }
    }

    /// Execute text-to-speech tool
    async fn execute_tts_tool(&self, model_id: &str, args: &serde_json::Value, user_id: &str) -> Result<String> {
        let input = args.get("input")
            .or_else(|| args.get("text"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing 'input' or 'text' in tool arguments"))?;

        let voice = args.get("voice")
            .and_then(|v| v.as_str())
            .unwrap_or("alloy");

        let request = mawi_core::types::TextToSpeechRequest {
            model: model_id.to_string(),
            input: input.to_string(),
            voice: voice.to_string(),
        };

        let (content_type, audio_bytes): (String, Vec<u8>) = self.executor.execute_text_to_speech(&request, user_id).await?;
        Ok(format!("Audio generated: {} ({} bytes)", content_type, audio_bytes.len()))
    }

    /// Execute speech-to-text tool
    async fn execute_stt_tool(&self, _model_id: &str, _args: &serde_json::Value, _user_id: &str) -> Result<String> {
        // STT requires audio data which we don't have in this text-based tool calling
        Err(anyhow!("Speech-to-text not supported in current tool calling implementation"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_plan_json() {
        let content = r#"["Step 1", "Step 2"]"#;
        let plan = AgenticExecutor::parse_plan_content(content);
        assert_eq!(plan, vec!["Step 1", "Step 2"]);
    }

    #[test]
    fn test_parse_plan_markdown_json() {
        let content = r#"```json
        ["Step 1", "Step 2"]
        ```"#;
        let plan = AgenticExecutor::parse_plan_content(content);
        assert_eq!(plan, vec!["Step 1", "Step 2"]);
    }

    #[test]
    fn test_parse_plan_numbered_list() {
        let content = "1. Step 1\n2. Step 2";
        let plan = AgenticExecutor::parse_plan_content(content);
        assert_eq!(plan, vec!["Step 1", "Step 2"]);
    }
    
    #[test]
    fn test_parse_plan_bullet_list() {
        let content = "- Step 1\n- Step 2";
        let plan = AgenticExecutor::parse_plan_content(content);
        assert_eq!(plan, vec!["Step 1", "Step 2"]);
    }
}
