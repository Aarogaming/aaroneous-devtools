# C93 claim-check hand-grading (retained evidence)

Retained so the grading survives the sessions that did it (Mimo's note on #26: live-run evidence sat only in comments). Author: Claude Sonnet 5.5. Reviewer should re-grade, not trust.

**Setup.** Local worker, model `qwen3.5:9b-q6` via Ollama, temperature 0. Feeder-generated fixture jobs against the product checkout `D:\Aaroneous` (read-only). Flags are shadow-mode `conformance_flags` from the C93 checks (#26) as extended in #35. Grading method: read each observation, then `grep -n` the cited source. Sample sizes are tiny; nothing here is a measured rate.

## Flagged observations (all graded false positives)

| job | source file | flag | what the source shows | cause |
|---|---|---|---|---|
| FEED-6693CACA2022DC54 obs 1 | crates/capabilities/src/traits.rs | line range does not contain quoted text; first found at line 2 | `pub trait Specialist: Send + Sync {` is at line 43; model cited 42 | first quote was bare `Specialist`, found in a doc comment at line 2 |
| FEED-79F28968AAC33ADF obs 1 | crates/capabilities/src/universal_tool.rs | not anchored | `grep -c` for `cfg(test)` and `#[test]` returns 0, so "no test module" is true | negative claim quoting text absent by definition |
| FEED-BC531D6C234A149A obs 1 | crates/compute/src/token_consumer.rs | not anchored | `pub fn new` in `impl StateAdaptor` at line 49; model said 47 | quoted Rust path `StateAdaptor::new`, not literal in source |
| FEED-BC531D6C234A149A obs 2 | crates/compute/src/token_consumer.rs | not anchored | `fn process_token` at line 62; model said 54 | quoted Rust path `StateAdaptor::process_token` |
| FEED-CFC7D55104E450C7 obs 0 | crates/compute/src/tensor_kernel.rs | not anchored | the file defines `MaelstromTensorEngine` | quote was the file's own path, never in file contents |

Fixes: #35 (negation, any-quote-in-range, `::` path segments, declared evidence paths).

## Unflagged observations spot-checked (all true)

Job FEED-F279B98E0D46616D, crates/llm_gateway/src/gateway.rs: (0) public methods `chat`, `embed`, `chat_with_ast_prefix` exist (lines 56, 66, 76); (1) no `cfg(test)` or `tokio::test` in the file; (2) the `LLMProvider` and `AstPrefixCacheManager` references exist (lines 11/25 and 78).

## Not verified
- No flagged observation has been a truly wrong claim yet, so the checks' true-positive behaviour is only covered by a constructed test (`real_observations_pass_and_mutated_ones_are_caught`).
- Unflagged observations of the other jobs were not graded; recall is unknown.
- The worker result JSON files are in a scratch directory and are not retained here.
