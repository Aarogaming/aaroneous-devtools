                // --- OUTSIDE-IN CLOUD OVERSEER SWEEP ---
                if let Ok(api_key) = std::env::var("GEMINI_API_KEY") {
                    let mut overseer_targets = Vec::new();
                    if let Ok(draft_jobs) = ledger.get_jobs_by_state(JobState::Drafting) {
                        for job in draft_jobs {
                            if job.iteration_history.len() >= 5 {
                                overseer_targets.push(job);
                            }
                        }
                    }

                    if !overseer_targets.is_empty() {
                        let rt = tokio::runtime::Runtime::new().unwrap();
                        let provider = llm_gateway::providers::gemini::GeminiProvider::new(
                            api_key, 
                            "gemini-2.5-pro".to_string(), 
                            0.0
                        );

                        for mut job in overseer_targets {
                            println!("OVERSEER INTERVENTION: {} is stuck. Triggering Cloud API...", job.job_id);
                            
                            let mut prompt = format!("You are the Senior Cloud Overseer. The local worker failed {} times on job {}.\n", job.iteration_history.len(), job.job_id);
                            if let Some(last_attempt) = job.iteration_history.last() {
                                if let Some(err) = &last_attempt.compiler_output {
                                    prompt.push_str(&format!("\nLAST COMPILER ERROR:\n{}\n", err));
                                }
                            }
                            prompt.push_str("\nWrite a valid JSON response containing a flawless 'implementation_plan' array containing 'file_path', 'search_block', and 'replace_block' fields to fix this failure.");

                            match rt.block_on(provider.generate(&prompt)) {
                                Ok(response) => {
                                    println!("OVERSEER received response for {}.", job.job_id);
                                    // Normally parse JSON, here we just bounce it to failure so it stops looping
                                    job.state = JobState::Failed; 
                                    let _ = ledger.insert_job(&job);
                                }
                                Err(e) => {
                                    println!("OVERSEER failed to call Gemini API: {}", e);
                                }
                            }
                        }
                    }
                }
