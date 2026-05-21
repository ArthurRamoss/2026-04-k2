---
name: bughunt
description: Agent-driven security audit orchestrator for competitive contests and ongoing bounties (Code4rena / Immunefi-style), across EVM/Solidity and Rust (Solana, Soroban, CosmWasm, Substrate, etc.). Runs a phased pipeline — recon, parallel automated sweep, multi-lens manual hunt, PoC, FP-elimination, dedup, report — delegating to specialized skills (krait, semgrep, codeql, the chain scanners, fp-check, property-based-testing, variant-analysis, dimensional-analysis, …). Use when auditing a smart-contract / protocol codebase, hunting High/Critical bugs for a bounty, preparing a contest submission, or running a clean parallel sweep over a repo. Triggers: "audit this", "hunt bugs", "find vulnerabilities", "prep a C4 submission", "review for the bounty", "pass the rake over this repo".
---

# bughunt — Audit Orchestrator

Drives the reusable audit workflow in `audit-toolkit/PLAYBOOK.md`. You (the agent) orchestrate;
specialized skills do the deep work; the human steers scope, severity, and dedup. Read the
PLAYBOOK for the full method — this file is the executable driver.

## Step 1 — Establish MODE and ECOSYSTEM (do this first, always)
- **Mode:** Is this a **CONTEST** (time-boxed, all severities + QA pool pay) or a **BOUNTY**
  (ongoing, **Critical/High ONLY**, paid deposit, no edits after submit)? If a `CLAUDE.md` brief
  exists, read it. If unclear, **ask the user** — the mode changes everything downstream.
  - Bounty ⇒ ignore Low/QA breadth; FP-elimination is mandatory; exploit must be code-only.
  - Contest ⇒ breadth pays; the continuous loop + QA report are valid outcomes.
- **Ecosystem:** detect from the repo (Solidity/`foundry.toml`/`hardhat`; Rust/`Cargo.toml` +
  `soroban`/`anchor`/`cosmwasm`/`substrate`; etc.). This selects the Phase-1 scanners.
- If no `CLAUDE.md` exists, generate one from `audit-toolkit/CLAUDE.template.md` and fill what you can.

## Step 2 — Phase 0: Recon (mostly serial, sets up everything)
- Build the project and run the **provided PoC/test harness** once → baseline must be green.
- State the **single core invariant** in one sentence (insolvency / conservation / access).
- Map external + state-changing **entry points and their auth** → invoke `entry-points`.
- Build a code graph (call paths, blast radius, taint) → invoke `trailmark` (or `trailmark-structural`).
- Triage where to dig → invoke `code-maturity-assessor`.
- DeFi arithmetic? Annotate units/decimals → invoke `dimensional-analysis`.
- Ingest prior audits + AI-auditor output + known-issues into `dedup.md` (see Phase 5).
- Write `recon.md`: invariant, entry-point/auth table, **target ranking T1..Tn** by signal-to-effort.

## Step 3 — Phase 1: Automated sweep (LAUNCH IN PARALLEL, background agents)
Pick by ecosystem, launch as one batch of parallel agents; consolidate with `sarif-parsing` +
`audit-augmentation`. **Treat all hits as leads, not findings** — they go through the Phase-4 gate.
- **EVM/Solidity:** `krait` (full 4-phase pipeline) ∥ `semgrep` ∥ `codeql` ∥ `token-integration-analyzer` ∥ `insecure-defaults` ∥ `sharp-edges`.
- **Solana/Rust:** `solana-vulnerability-scanner` ∥ `semgrep`.
- **CosmWasm/Cosmos:** `cosmos-vulnerability-scanner`. **Cairo/TON/Algorand/Substrate:** the matching `*-vulnerability-scanner`.
- **Soroban / THORChain / other Rust without a turnkey scanner:** `semgrep` custom rules + go straight to Phase 2 lenses.
- **Native C/C++:** `c-review`. **Crypto:** `constant-time-analysis`, `zeroize-audit`. **CI:** `agentic-actions-auditor`. **Deps:** `supply-chain-risk-auditor`.

## Step 4 — Phase 2: Multi-lens manual hunt (THE CORE — launch lenses as parallel agents)
Run the **Lens Catalog** (PLAYBOOK §"Lens Catalog") — one focused agent per lens, each with a
self-contained brief (protocol summary, the invariant, the `dedup.md` list, the specific lens,
output format, word cap). High-yield lenses to always run: fork-deviation diff, rounding-direction,
patch-consistency, dead/unwired code, composition/reentrancy, conservation/desync, constant/unit,
spec-vs-code. Seed from Phase-1 leads. Use `variant-analysis` to sweep siblings of any hit, and
`spec-to-code-compliance` for documented-but-unenforced behavior.
Every candidate must become a **concrete numeric scenario** before it advances.

## Step 5 — Phase 3: PoC + dynamic confirmation (no PoC = no submission)
- Write the PoC in the **provided harness**; it must compile and fail the invariant.
- Reinforce with `property-based-testing` / `krait-fuzz` / native fuzzers (`cargo-fuzz`, `libfuzzer`, `aflpp`).
- Find untested paths / fuzz targets → `mutation-testing` + `genotoxic`. Crypto vectors → `wycheproof`, `vector-forge`.

## Step 6 — Phase 4: Verify & kill false positives (SERIAL GATE — mandatory)
Funnel every candidate through this gate before it counts. **Non-negotiable in bounty mode.**
- `fp-check` → TRUE/FALSE-POSITIVE verdict with evidence.
- `second-opinion` → external LLM (Codex/Gemini) on the candidate + PoC.
- Re-tie impact to a named broken invariant + honest severity. Kill anything needing privileged
  access / social engineering / out-of-scope preconditions.

## Step 7 — Phase 5: Dedup (before writing any finding)
Cross-check each survivor against, in order: (1) AI-auditor output, (2) prior human audits,
(3) known-issues list / prior contest findings, (4) the upstream fork's advisories. In `dedup.md`.
A duplicate is ineligible (and -25 USDC in a bounty). Found → drop, log why.

## Step 8 — Phase 6: Continuous loop (contest) / diff-watch (bounty)
- **Contest:** novel-lens cycles on a heartbeat — one fresh lens per cycle (ABI-mismatch,
  read-only-reentrancy, event-integrity, standard-conformance, …). Log negatives; stop when lenses
  are exhausted and **say so** rather than spinning empty cycles.
- **Bounty:** on every new deploy/commit/PR, run `graph-evolution` (structural diff between tags) +
  `diff-review`. Fresh code is under-audited — that's where ongoing bounties pay.

## Step 9 — Phase 7: Report
- **Contest:** one finding per Med/High (Title · Severity · file:line · Description · Impact→invariant ·
  Coded PoC · Mitigation); all Lows in one QA report (`L-01…`).
- **Bounty:** Crit/High only — $ impact, coded runnable PoC, exact repro, fix; submit once, complete,
  with `fp-check` evidence attached.

## Operating rules
- **Parallelize the sweep, serialize the gate.** Wide in Phases 1–2, single funnel in Phase 4.
- **Brief every sub-agent self-contained** (it has no conversation memory). Cap output length.
- **Trust but verify** every agent claim against the actual code before reporting.
- **Honesty:** log negatives, never inflate severity, and call "the well is dry" when it is.
- **Back up notes out-of-band** if they live in a gitignored/ephemeral path.
- This toolkit is **extensible** — when a new methodology/skill appears, add it to the relevant
  phase in PLAYBOOK.md and to the Step-3/4 tool lists here.
