# Foundation Boundary

`aaroneous-devtools` is the private foundation and operations workspace for Aaroneous-adjacent
work. It contains reusable developer setup, operational automation, evaluation harnesses, and
private experiments that support Aaroneous without becoming part of its product contract.

It is not a generic archive for retired framework source. Aaroneous Git history is authoritative
for removed product artifacts. A transferred item must have a maintained foundation or operations
use case, a named owner, documented inputs and outputs, and no dependency path back into the
Aaroneous workspace, CI, release, or runtime.

## Companion flexibility

Devtools follows the [companion operating model](governance/OPERATING_MODEL.md), not Aaroneous's zero-allocation execution-plane constraints. Ordinary Rust services, heap allocation, async I/O and a TypeScript presentation client are acceptable here. Maintained policy remains Rust-first; quick research and thin platform adapters stay isolated.

User-approved patches may flow from devtools into an isolated Aaroneous branch and pass Aaroneous's native gates. This is a development action, not a dependency from the framework back into devtools. No autonomous writer may overwrite shared product work or treat generated output as verified.
