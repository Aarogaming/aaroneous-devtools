---
notation: v1
purpose: A 3D analogy-reference view, a light exploration layer, and a staged digital twin built from the owner's stated preferences
authority: propose
status: proposal, advisory, grants nothing
relations:
  - depends-on: doc:06-brief-compiler-and-packets
  - depends-on: doc:10-native-app-operations-and-reuse
  - relates-to: doc:proposals/TRANSPARENT_BY_DESIGN_NOTATION.md
checked-by: none yet (see "Not verified")
---

# Visualizer, exploration and digital twin (proposal 09)

Contribution-ID: C119/claude-hud-v2-proposal-v1 (part 09 of the set indexed in [README](README.md)).
Author seat: Claude Sonnet 5.5 (identity is a round-table roster claim, not cryptographic).
Status: **proposal for review**. It changes no source, workflow or policy, and nothing here is accepted until a reviewer
other than the author records it. Read the [README](README.md) first for the requirements (R1 to R10), the layers and
the decision log; this document is one feature of that set and is reviewable on its own.

## The 3D visualizer for analogy references (R10)

**My reading of the request, to be confirmed:** the architecture and proposals already lean on analogies (the spiral
staircase and stepped double helix, the PLC scan cycle, the biology-flavoured legacy names, patterns borrowed from
other runtimes). The visualizer is a map of those references: *concepts*, *documents*, *lanes*, *components* and the
*analogies* that link a source idea to a target one, with the mapping, how strong it is, and what evidence supports it.

- **Data model (a view over data we already keep, not a new store):** nodes are typed (concept, document, lane,
  component, analogy); edges are typed relations, reusing the relation vocabulary of the C101 notation proposal
  (`informs`, `depends-on`, `overlaps`) plus `analogous-to` with a mapping list. Every edge carries provenance and a
  status (`unverified` until a reviewer other than its author confirms it), so a picture can never present a
  model-suggested analogy as established.
- **Who makes the analogies:** software extracts the explicit relations already in front matter and the queue; the light
  model may propose `analogous-to` edges; a person confirms. Layout is computed by software with a fixed seed, so the
  same graph draws the same way and screenshots are comparable.
- **What 3D is for, honestly:** 3D earns its place where the metaphor is spatial (layers, helix, scan cycle, depth for
  abstraction level) and where seeing density and neighbourhoods helps. It does not help reading text, editing a brief or
  approving an action, and dense 3D graphs have occlusion and navigation costs. So 3D is **one view in the application,
  next to 2D panels** for the survey, brief, packets and operations, and the same graph must be readable as a flat list
  and a 2D map (also for accessibility).
- **Cost:** a GPU renderer competes with the local model for the same graphics card and memory. The renderer's frame
  rate, resolution and effects are settings (R8), it drops to the 2D view when the model is running or the machine is
  under load, and it can be off entirely. The core and MCP adapter never depend on it.
- **Toolkit:** a native Rust 3D renderer inside the same application as the 2D panels. A game-engine crate with an
  immediate-mode GUI layer is the likeliest fit, and a lighter 3D crate is the fallback; both are heavy dependencies with
  long build times (general knowledge, NOT VERIFIED), so they are chosen by a spike and scored under the devtools
  criteria in `OPERATING_MODEL.md` before adoption. A spike must show: the packet editor and survey usable, a 2,000-node
  graph at an acceptable frame rate while the local model is running, and keyboard navigation.
## Extensions under discussion (ideas, not commitments)

The operating model for users, contributors and operators (intake, triage, skill-gated delegation, earned trust) is in [07](07-guild-workers-staff-and-inspection.md) and [08](08-roles-skills-and-trust.md).

Owner thoughts in chat, 2026-10-10: liven up the interface with light 3D interactivity, in the spirit of a
location-based exploration game; "data mining ideas from people" as mental mapping; maybe a psychology portion; maybe a
full digital twin of the owner. Recorded here with how each maps onto what already exists. Nothing in this section is
scheduled, and each item would be its own lane with its own review.

**What the canonical Aaroneous architecture already specifies (read from the product repo, 2026-10-10).**
`docs/architecture/human_interface_intent_mirror.md` and Pillar 6 of `MASTER_ARCHITECTURE.md` define a decoupled
**Human Node** reached through a Human-Interface Abstraction Layer (HIAL); **Socratic Vector Pinning** along three
axes (invariants, dependencies, trade-offs) compiled into an Intent DAG; a **3-Option Intent Mirror** (conservative,
redesign, quick validate) shown before non-trivial changes; a personalised **`.si` digital twin cartridge** that
"encodes developer preferences, architectural tolerances, coding idioms, and verification budgets"; and a viewport
mode that renders "3D concept constellations" with `wgpu` shared into `egui`. The `omni` crate already holds galaxy,
constellation and spatial-layout code, and `studio_hud` uses `egui`/`eframe`. I searched `crates/*.rs` for HIAL,
the intent mirror and Socratic pinning and found no matches, so I read these as specification, not working code
(NOT VERIFIED beyond that search). I found no code that defines a preference payload for the `.si` twin; the
`si_format` crate covers the container, alignment, capability masks and verification.

**Consequences for this proposal.**
- The brief compiler ([06](06-brief-compiler-and-packets.md)) and Socratic pinning are the same idea. The brief template's slots should include the three
  pinning axes, and the approval screen for a brief should be the three-option mirror. This keeps one design instead
  of two.
- The 3D analogy view (above) should reuse the `omni` layout and constellation code through a workspace trait rather
  than a new renderer, and the native application should use the `egui`/`eframe` stack the product already chose.
  That answers the toolkit question in [10](10-native-app-operations-and-reuse.md) as a default; the spike still has to pass its tests. Devtools may
  depend on Aaroneous crates, never the reverse.
- The digital twin is Aaroneous's concept, so devtools should build the **data** and the **use** of it, not a rival
  format: an inspectable record of the owner's stated preferences and decisions with provenance, shaped so it can be
  exported into the `.si` twin cartridge when that format exists.

**Exploration layer (light 3D interactivity).** Treat the idea graph as a map. Unexplored places are things the system
does not know yet: an analogy with status `unverified`, an empty required brief slot, an open question from the
queue. Visiting one opens the survey question for it; answering or confirming it "claims" it. This is a skin over the
same data and the same permission rules, controlled by a setting (`ui.exploration`, off or on, with the intensity
left to configuration). Guardrails: rewards attach to **confirmed, evidenced** answers, not to volume, so the loop
cannot teach people to answer quickly rather than truthfully; no streak or loss mechanics; and every screen must
still work without the 3D view.

**Eliciting ideas from people.** Plain, established elicitation methods fit the survey: card sorting, laddering
("why does that matter?"), and concept maps. They produce structured slots and graph edges, which is what the
compiler needs. Who the "people" are matters: the owner and the registered seats need nothing beyond the retention
rules already in place. Collecting from anyone else would need their knowledge and agreement, a stated purpose,
minimal retention and local-only storage, decided before building, not after.

**Psychology portion.** Proposed scope: elicitation and mapping of **what a person says they want and decide**.
Out of scope: inferring traits, moods or mental state, scoring people, or anything presented as diagnosis. Those
claims are not supportable from this kind of data and would put unverified conclusions about real people into the
system.

**Digital twin, staged so each step is measurable.**
1. A decision and preference log: the owner's standing directives (for example the rules already stated in
   `AGENTS.md`, the operating model and chat), each with source, date, scope and confidence. Viewable and editable by
   the owner; deletable.
2. Use it only to **pre-fill brief slots as `proposed`**, never `confirmed`, so a wrong guess costs one correction.
3. Measure it: the share of proposed slots the owner accepts unchanged, per slot type. This is the twin's quality
   figure and the evidence for or against expanding it.
4. Only then consider simulation (asking "what would the owner decide?") and export to the `.si` cartridge.
Constraints that apply from step 1: the twin may propose and never approve; it grants no merge, review or
attestation authority and is bound by the permission floor; it is stored in the user settings layer and is not sent to
any cloud model unless a setting allows that specific use; every use is logged; and it is labelled as a model of
stated preferences that can be wrong.

**Open questions for the owner.** Which people beyond the owner and the seats are meant by "people"? Is the twin for the
owner alone? Is exploration a default-on feature or opt-in? Should the first step be the preference log, ahead of the
brief compiler's survey?
## Phases for this feature

| Phase | Deliverable | Depends on |
|---|---|---|
| P7 (part) | the 3D analogy view over the graph data model, after the 2D panels and the toolkit spike | 06, 10 |
| twin step 1 | decision and preference log, viewable, editable, deletable | 06 |
| twin step 2 to 3 | pre-fill brief slots as `proposed`; measure acceptance | twin step 1 |

## Decisions for this feature

1. 3D is the analogy-visualizer view inside a native app whose working screens are 2D; it starts after the brief compiler. DECIDED.
2. The reading of "analogy reference" is CONFIRMED.
3. OPEN: who the "people" are beyond the owner and the seats; whether the twin is for the owner alone; whether exploration is default-on or opt-in; whether the preference log comes before the survey.

## Review focus

- Is the reading of "analogy reference" (concepts, documents, lanes and components linked by analogies with mappings, strength and evidence) right? CONFIRMED by the owner.
- Is 3D one view next to 2D panels, with the same graph readable as a list, the right scope?
- Do the exploration guardrails (rewards for confirmed evidence, no streaks or loss mechanics) and the psychology scope (elicitation of stated wants, no trait or mood inference) hold?
- Is the staged twin (preference log, proposed slots only, measured acceptance, then maybe more) the right order, and is it correctly kept inside Aaroneous's `.si` twin concept?

## Not verified

- That a 3D renderer, the local model and the 2D panels can share the owner's GPU acceptably; to be measured in the toolkit spike.
- That the `omni` layout code can be driven from a workspace trait and rendered inside the chosen GUI; not read in depth.
- Whether HIAL, the intent mirror and the `.si` twin payload exist anywhere other than the specification; my search of `crates/*.rs` found none.
