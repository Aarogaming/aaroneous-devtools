                // Apply FileModifications
                let mut apply_failed = false;
                let mut apply_error = String::new();
                
                if let Some(draft) = &job.current_draft {
                    for modif in draft {
                        // Strip leading slash if any to make it relative
                        let rel_path = modif.file_path.trim_start_matches('/');
                        
                        // 1. Path Traversal Protection
                        if rel_path.contains("..") {
                            apply_failed = true;
                            apply_error.push_str(&format!("error[E-SEC01]: sandbox security violation\n  --> {}\n  |\n  = note: Path traversal ('..') is strictly forbidden.\n  = help: Only target files explicitly within the component workspace.\n\n", modif.file_path));
                            continue;
                        }

                        // 2. Boundary Protection
                        if rel_path.starts_with(".cargo") || rel_path.starts_with(".github") || rel_path.starts_with("xtask") || rel_path == "build.rs" {
                            apply_failed = true;
                            apply_error.push_str(&format!("error[E-SEC02]: framework boundary violation\n  --> {}\n  |\n  = note: Modification of infrastructure, workflow, or build files is banned.\n  = help: Focus your modifications strictly within the src/ or core/ runtime files.\n\n", modif.file_path));
                            continue;
                        }

                        let target_path = sandbox_dir.join(rel_path);
                        
                        if let Ok(content) = std::fs::read_to_string(&target_path) {
                            let matches = content.matches(&modif.search_block).count();
                            
                            if matches == 0 {
                                apply_failed = true;
                                apply_error.push_str(&format!("error[E-PTC01]: orphaned search block\n  --> {}\n  |\n  = note: The exact string block provided in 'search_block' could not be found in the file.\n  = help: Check for missing/extra leading whitespace, or fetch the latest version of the file. The search block must match the existing file verbatim.\n\n", modif.file_path));
                            } else if matches > 1 {
                                apply_failed = true;
                                apply_error.push_str(&format!("error[E-PTC02]: ambiguous search block\n  --> {}\n  |\n  = note: The provided 'search_block' matched {} times in the file. Patches must be deterministic.\n  = help: Expand your 'search_block' to include unique surrounding lines (like a function signature or specific comment) so it only matches exactly once.\n\n", modif.file_path, matches));
                            } else {
                                let new_content = content.replace(&modif.search_block, &modif.replace_block);
                                let _ = std::fs::write(&target_path, new_content);
                            }
                        } else {
                            apply_failed = true;
                            apply_error.push_str(&format!("error[E-IO01]: target file not found\n  --> {}\n  |\n  = note: The file does not exist in the repository checkout.\n  = help: Verify the file path is correct relative to the repository root.\n\n", modif.file_path));
                        }
                    }
                }
