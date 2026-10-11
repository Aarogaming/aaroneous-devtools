---
notation: v1
purpose: Cut token cost with tiered routing, content-addressed packs, conditional responses, caches and measured dense handles
authority: propose
status: proposal, advisory, grants nothing
relations:
  - depends-on: doc:02-agent-door-and-mcp
  - informs: doc:05-model-profiles-and-prompt-adaptation
checked-by: none yet (see "Not verified")
---

# Token optimization (proposal 04)

Contribution-ID: C119/claude-hud-v2-proposal-v1 (part 04 of the set indexed in [README](README.md)).
Author seat: Claude Sonnet 5.5 (identity is a round-table roster claim, not cryptographic).
Status: **proposal for review**. It changes no source, workflow or policy, and nothing here is accepted until a reviewer
other than the author records it. Read the [README](README.md) first for the requirements (R1 to R10), the layers and
the decision log; this document is one feature of that set and is reviewable on its own.

## Token optimization (ranked by expected saving, all measurable)

1. **Tiered routing (biggest).** Software, then local model, then cloud, as in 4.3. Expected effect: most routine
   status/queue/"what's next" questions never reach a cloud model.
2. **Context packs, content-addressed.** The collector builds a small, structured digest of the relevant state
   (open lanes, the seat's card, last N events) and names it by hash. A model is given the pack plus the question,
   not the files. Identical pack + identical template = identical prompt prefix, which also helps provider-side
   prompt caching.
3. **Conditional responses.** `ETag`/`304` on cards and work lists: an unchanged card costs a few bytes and zero model
   tokens. This also gives "stay quiet on unchanged healthy state" a mechanical form.
4. **Result cache** keyed by (template version, input digests). Re-asking the same question over unchanged evidence
   returns the stored answer.
5. **Delta cards.** After the first full card, a seat receives only what changed since its last `ETag`.
6. **Event stream with sequence numbers** instead of repeated polling; a seat sleeps until something relevant changes.
7. **Structured outputs and short templates** for the local model; stable prefixes; cap output length per class.
8. **Accounting.** Every model call records input/output token counts, the tier that answered, and cache hit/miss in the
   retained log. The HUD shows tokens saved by tier and cache hits. Without this, none of the above is verifiable.

9. **Dense handles: choose the encoding by measurement.** The owner's idea, 2026-10-10: send tokens cheaper by
   re-encoding numbers into a higher base (decimal to base 36 or 64), done by software. Measured result in Appendix A:
   on the local model's tokenizer it is **true for dense, incompressible data** (digests, large numbers, packed
   telemetry) and **false for ordinary text**. Policy: identifiers, digests and sequence numbers that go into a prompt
   use a dense alphabet chosen per backend from a calibration run; content the model must read or reason about
   stays plain; the software decodes. Caveats: the best alphabet depends on each tokenizer (the digit-splitting seen
   here is not universal), a model decodes dense strings unreliably so they must be opaque handles software resolves,
   and the larger saving is not sending the payload at all (handles to stored packs, items 2 to 5).

10. **A ratchet on model reliance.** Every model-driven step is recorded with its programmatic predecessor (the rule, parser, lookup, type or audit that is
    tried first) and with two measured numbers per task type: how often the predecessor settled it, and how often the model was reached. The model-reached share
    may fall but not rise without a recorded reason, the same one-way rule the Aaroneous ratchet applies to compliance. Raised by the Gemini seat's entries 74 and
    78 and adopted: if a task can be proven by a compiler, an auditor rule or a parser, it does not go to a model, and each tool built should make the next
    agent cycle faster, quieter and less model-dependent. The figures that make this checkable (call counts by type, tokens, latency) come from item 8.

Savings figures are not claimed in this document except the measurement in Appendix A; the rest are to be measured by item 8.
## Appendix A. Token cost of encodings, measured 2026-10-10

Method: the script below sends each encoding of the same 512 random bytes (4096 bits of information, fixed seed)
to the local Ollama model `qwen3.5:9b-q6` with `raw` prompting and `num_predict = 1`, and reads `prompt_eval_count`,
the model's own token count. The wrapper overhead was 1 token and is subtracted. Run on the owner's Windows machine; one
tokenizer only.

| Encoding | Characters | Tokens | Bits per token |
|---|---:|---:|---:|
| decimal (base10) | 1233 | 1232 | 3.32 |
| hex | 1024 | 906 | 4.52 |
| base36 | 793 | 561 | 7.30 |
| base32 (lower case) | 820 | 527 | 7.77 |
| base62 | 688 | 521 | 7.86 |
| base85 | 640 | 500 | 8.19 |
| base64 | 684 | 499 | 8.21 |

Also measured: a 64-bit identifier written as 13 base-36 characters cost 9 tokens. A prose sample with identifiers and
timestamps came to 2.46 characters per token.

Reading: on this tokenizer every decimal digit is its own token, so decimal carries 3.32 bits per token and base 64
carries about 8.2, roughly 2.5 times more. A secondary source on other tokenizers reports that base64 still beats hex
but costs about 2 to 2.5 characters per token on pseudo-random data, and that other tokenizers split digits
differently (so the decimal baseline, and the gain, will differ). **Not measured:** any cloud model's tokenizer,
decode accuracy when a model must read the string, and the end-to-end saving on real prompts. Those need the
calibration run in item 9, using each backend's reported usage.

```python
import base64, json, random, string, urllib.request
MODEL, URL = "qwen3.5:9b-q6", "http://127.0.0.1:11434/api/generate"

def tokens(prompt):
    body = json.dumps({"model": MODEL, "prompt": prompt, "stream": False, "raw": True,
                       "options": {"num_predict": 1, "temperature": 0}}).encode()
    req = urllib.request.Request(URL, body, {"Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=300) as r:
        return json.load(r)["prompt_eval_count"]

def to_base(n, alphabet):
    out = []
    while n:
        n, rem = divmod(n, len(alphabet)); out.append(alphabet[rem])
    return "".join(reversed(out)) or alphabet[0]

payload = bytes(random.Random(7).randrange(256) for _ in range(512))
n = int.from_bytes(payload, "big")
b36, b62 = string.digits + string.ascii_lowercase, string.digits + string.ascii_uppercase + string.ascii_lowercase
encodings = {"hex": payload.hex(), "decimal": str(n),
             "base32": base64.b32encode(payload).decode().lower().rstrip("="),
             "base36": to_base(n, b36), "base62": to_base(n, b62),
             "base64": base64.b64encode(payload).decode(), "base85": base64.b85encode(payload).decode()}
base = tokens("x")
for name, text in encodings.items():
    cost = tokens(text) - base
    print(name, len(text), cost, round(4096 / cost, 2))
```
## Amendments from review (MiMo seat, board entry 121)

- **One change-propagation channel.** Delta cards (item 5) and the event stream with sequence numbers (item 6) are the same mechanism. Build the stream first and **derive**
  the deltas from it, or two caches will drift.
- **Name each cache and give each one key.** The *result cache* (answers to a step, key: template version, adapter version, model identity as defined in
  [05](05-model-profiles-and-prompt-adaptation.md), and the input digests) is ours. The *provider prompt cache* is the provider's and is keyed by a stable prompt prefix, which we only
  arrange. Item 4 and the profile key in 05 must use these same definitions before both are built.
- **The ratchet binds two numbers, not one.** The model-reached share can be lowered by auto-settling trivial predecessor cases while the absolute number of model calls rises,
  so item 10 binds both the share and the absolute calls per task type.
- **The ranking is a hypothesis.** Item 1 is called the biggest saving but is unmeasured; in the two-host operations the observed waste was repeated unchanged context, which items 2 to 5
  target. Accounting (item 8) may reorder the ranking entirely.
- **Falsifying experiment (credit: MiMo seat, NOT RUN, needs the Ubuntu host):** run the script in Appendix A unmodified against the Ubuntu machine's `qwen3.5:9b` Q4_K_M build
  (digest 56671c2ab938) and compare the ordering of the seven encodings with the Windows q6 result. If the ordering differs materially between two quantisations of one family, the policy
  defaults to per-deployment calibration and stops inheriting this table.

## Phases for this feature

| Phase | Deliverable | Depends on |
|---|---|---|
| P4 (part) | token accounting per call and tier; result cache; context packs; handle encoding chosen per backend | 02, 05 |

## Decisions for this feature

1. Adopt the dense-handle policy only after a calibration run on each backend (proposed).

## Review focus

- Is the ranking of the techniques sensible, and is accounting (item 8) the precondition for claiming any saving?
- Is the dense-handle policy (measured per backend, opaque handles only, plain text where the model must read) the right use of the base-encoding idea?
- Re-run Appendix A on another tokenizer if you have one; the finding depends on how a tokenizer splits digits.

## Not verified

- All savings other than Appendix A are targets, not measurements.
- Any cloud model's tokenizer, decode accuracy when a model must read a dense string, and the end-to-end saving on real prompts.
