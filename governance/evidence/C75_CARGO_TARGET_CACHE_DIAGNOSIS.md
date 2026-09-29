# C75 Cargo target-cache diagnosis

Recorded 2026-09-29 for independent review. This is an observed environment
correlation, not a source-level causation claim.

## Invocation and results

All six runs used worktree
`C:\Users\aarog\.codex\worktrees\c75-test-isolation\aaroneous-devtools`,
working directory `foundation`, and command `cargo test --all-targets`.

Runs `default-environment-{1,2,3}.log` had no per-command
`CARGO_TARGET_DIR`/`CARGO_INCREMENTAL` override. Before those runs, effective
process, user, and machine values for both variables were empty. The exact
Cargo config path `C:\\Users\\aarog\\.cargo\\config.toml` then contained
`[build] target-dir = "C:\\CargoTargetCache"`; output used that shared cache.
Exit results: run 1: 0 (76 CLI tests); run 2: 101, with the named rollback
test failing because broken source unexpectedly passed Cargo check (75 passed,
1 failed); run 3: 0 (76 CLI tests).

At owner direction that `build.target-dir` entry was removed. After removal,
the same process, user, and machine environment values remained empty and the
same config file was zero bytes. The same command then produced
`post-default-{1,2,3}.log`, used normal worktree `target\` output, and exited
0 each time (76 CLI tests plus 12 maintenance-UI and 4 UI tests).

## SHA-256 retained-log references

Source logs remain at `logs/c75/` in the named worktree.

| Log | SHA-256 |
| --- | --- |
| `default-environment-1.log` | `75E9ECAC64260A6E9593ECB45884A1C52B74474C6478EDE5C21B55BA56153D6D` |
| `default-environment-2.log` | `1FD86112ABF58D0DABC4445745786A7EC1457DD3569A677E70229225C68B3FCF` |
| `default-environment-3.log` | `90023350A0B93B0B4DAF9982193B4692E363688957FEE3F083C43842C27EBE31` |
| `post-default-1.log` | `0DF466014191DD39D8C7F06DF18603BFD3493B6B87381DAF4B42EFBDFED5C9CB` |
| `post-default-2.log` | `DA0144B5460D5593F411D6FA3ABA75D984C17DF1A08F1BE382FCDBF86A9EC4A8` |
| `post-default-3.log` | `3ECFEB056DADF646E97995FF81511EB808E9FA9EBADAC0B34124C84AB82667FC` |

## Proposed disposition

The failure was observed under the shared global target cache and was not
reproduced after its removal. No repository source change is warranted on the
current evidence; future failures under ordinary Cargo defaults reopen C75.
