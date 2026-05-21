# audit-toolkit — reusable bug-hunting workflow ("o ferramentão")

A portable, agent-driven security-audit workflow, distilled from the K2 engagement and built to
run on **any** repo — Code4rena contests **and** ongoing bounties, across **EVM/Solidity and Rust**
(Solana, Soroban, CosmWasm, Substrate, THORChain, …). The idea: you steer scope/severity/dedup,
and parallel agents "pass the rake clean" over the code using the best tool for each job.

## What's here
| File | What it is |
|------|------------|
| `PLAYBOOK.md` | The full methodology — mode switch (contest vs bounty), the 8-phase pipeline, the 20-lens catalog, agent-orchestration patterns, severity calibration. **Read this.** |
| `CLAUDE.template.md` | Per-engagement operating brief. Copy → target repo's `CLAUDE.md`, fill the `{{...}}`. This is what drives a single audit (it's how K2 was run). |
| `skills/bughunt/SKILL.md` | The executable orchestrator skill. Drives the phases, delegates to specialized skills. Invoke with `/bughunt`. |

## Deploy to a new target repo (3 steps)
1. **Brief:** copy `audit-toolkit/CLAUDE.template.md` → `<target-repo>/CLAUDE.md`; fill every `{{...}}`
   (mode, scope, prior audits, the core invariant, target ranking, PoC harness command).
2. **Activate the skill** (once per machine, makes `/bughunt` available everywhere):
   ```
   mkdir -p ~/.claude/skills && cp -r audit-toolkit/skills/bughunt ~/.claude/skills/bughunt
   ```
   (or copy into `<target-repo>/.claude/skills/bughunt` for that repo only — note many setups
   gitignore `.claude/`, so keep this `audit-toolkit/` copy as the source of truth.)
3. **Run:** open the target repo and say `/bughunt` (or "audit this for the bounty"). It detects
   mode + ecosystem and starts Phase 0.

## The two modes — the one thing to get right
- **Contest** (time-boxed): all severities pay, QA pool exists → breadth + a clean QA report are valid.
- **Bounty** (ongoing): **Critical/High ONLY pay**, you pay a deposit per submission and **can't edit**
  → ruthless Crit/High focus, mandatory `fp-check` before every submission, code-only exploits.

A QA report that's worth $5k in a contest is worth **$0** in a bounty. Set the mode first.

## The ferramentão — skill inventory (grouped by phase)
The orchestrator delegates to these. Add more over time (extend the lists in `PLAYBOOK.md` + `SKILL.md`).

**Phase 0 — Recon & context:** `entry-points` / `entry-point-analyzer` · `trailmark` / `trailmark-structural` ·
`audit-context-building` · `code-maturity-assessor` · `guidelines-advisor` · `audit-prep-assistant` ·
`diagramming-code` · `dimensional-analysis` · `supply-chain-risk-auditor`

**Phase 1 — Automated sweep:** `krait` / `krait-quick` (Solidity) · `semgrep` · `codeql` ·
`solana-vulnerability-scanner` · `cosmos-vulnerability-scanner` · `cairo-vulnerability-scanner` ·
`ton-vulnerability-scanner` · `algorand-vulnerability-scanner` · `substrate-vulnerability-scanner` ·
`token-integration-analyzer` · `insecure-defaults` · `sharp-edges` · `c-review` · `constant-time-analysis` ·
`zeroize-audit` · `agentic-actions-auditor` · `sarif-parsing` · `audit-augmentation`

**Phase 2 — Manual hunt:** the Lens Catalog (PLAYBOOK) · `variant-analysis` · `spec-to-code-compliance` / `spec-compliance`

**Phase 3 — PoC & dynamic:** `property-based-testing` · `krait-fuzz` · `mutation-testing` (mewt/muton) ·
`genotoxic` · `cargo-fuzz` · `libfuzzer` · `aflpp` · `atheris` · `ruzzy` · `libafl` · `harness-writing` ·
`coverage-analysis` · `fuzzing-obstacles` · `fuzzing-dictionary` · `ossfuzz` · `wycheproof` · `vector-forge`

**Phase 4 — Verify / kill FPs:** `fp-check` · `second-opinion` · `code-review` · `differential-review` / `diff-review`

**Phase 6 — Continuous / diff:** `graph-evolution` · `diff-review`

**Crypto-protocol (as needed):** `crypto-protocol-diagram` · `mermaid-to-proverif` · `constant-time-testing` · `ct-check`

## Provenance
Methodology distilled from the K2 audit (Soroban/Rust Aave V3 fork): ~18 manual lenses + a
7-cycle continuous loop, ~36 sub-agents, full dedup against a prior AI-auditor's output, two
passing coded PoCs. Outcome documented there: deep invariant work + honest severity calibration
beats breadth-without-PoCs. This toolkit packages that process so it's repeatable on the next repo.
