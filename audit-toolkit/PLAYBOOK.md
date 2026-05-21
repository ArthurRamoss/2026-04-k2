# Audit Playbook — Reusable Bug-Hunting Workflow

Distilled from the K2 engagement (Soroban/Rust lending fork of Aave V3). Ecosystem-agnostic
core, with EVM + Rust tool maps. Built for **agent-driven parallel sweeps** — you steer scope,
severity, and dedup; agents "pass the rake" across the code in parallel and report back.

This is the reference manual. The executable driver is the `bughunt` skill
(`audit-toolkit/skills/bughunt/SKILL.md`). The per-engagement config is `CLAUDE.template.md`.

---

## 0. Mode switch — READ FIRST. Contest vs Bounty economics are different.

| | **Contest** (time-boxed, e.g. a C4 audit) | **Bounty** (ongoing, c4 / Immunefi-style) |
|---|---|---|
| What pays | All severities — High, Medium, **and** Low/QA/Info pools | **Critical / High ONLY.** Med/Low usually pay **$0** |
| Submission cost | Free | **25 USDC deposit per submission** (C4 bounties) |
| Editable after submit | Usually yes (until close) | **No** — submission is frozen |
| Judge | C4 judge + sponsor | **Sponsor team** directly (14-day response window) |
| Duplicates | Split the pot | Often **first-valid-wins** / deprioritized |
| Time pressure | Hard deadline → triage hard | Continuous → re-audit on every new deploy/diff |
| Out-of-scope (typical) | Per-contest README | Centralization, governance/economic (51%), Sybil, depeg, best-practices, test/config, oracle-data-errors, already-exploited, privileged-access |

**Consequences for the workflow:**
- **Bounty → ruthless Crit/High focus.** A clean QA report is worth ~$0. Do **not** spend cycles
  on Low/Info breadth. Only chase: theft of funds, permanent/temporary freezing, protocol
  insolvency, unauthorized mint, manipulated RNG.
- **Bounty → FP-elimination is mandatory.** You pay to submit and can't edit. Run `fp-check`
  (and a `second-opinion`) on every candidate **before** submitting. A wrong submission = -25 USDC.
- **Bounty → exploit must be code-only.** No social engineering, no privileged access (unless the
  scope explicitly allows it). "High likelihood" = attacker controls the circumstances with public info.
- **Contest → breadth pays.** The continuous-loop breadth sweep (Phase 6) and a strong QA report
  are legitimate, achievable outcomes. Don't force a Medium that gets downgraded to Low (= ineligible).

Set the mode in the engagement's `CLAUDE.md` before starting. Everything below branches on it.

---

## The pipeline (Phases 0–7)

Each phase lists the goal, the agent pattern, and the skills/tools to invoke. Phases 0–1 are
mostly parallel automated sweeps; 2–4 are the real bug-finding; 5–7 are dedup, loop, report.

### Phase 0 — Setup & recon
**Goal:** build, get the PoC harness running, map the attack surface, ingest prior audits.
- Clone, install toolchain, run the build and the **provided test/PoC harness** once (green = baseline).
- Identify the **single core invariant** (e.g. lending: `aToken_supply × index ≤ underlying + debt × borrow_index`).
- Map every **external / state-changing entry point** and its **auth coverage** → `entry-points` skill.
- Build a **code graph** (call paths, blast radius, taint, entry points) → `trailmark` / `trailmark-structural`.
- Triage *where* to dig with a maturity scorecard → `code-maturity-assessor`.
- For DeFi arithmetic, **annotate units/decimals** → `dimensional-analysis` (catches WAD/RAY/bps & decimal-scale bugs).
- Ingest prior-audit + known-issue + AI-auditor outputs into a **dedup list** (see Phase 5).
- Output: `recon.md` (invariant, entry-point + auth table, target ranking T1..Tn by signal-to-effort).

### Phase 1 — Automated sweep (parallel, "pass the rake")
**Goal:** cheap wide coverage; generate leads, not final findings.
Launch these as **parallel background agents**, then consolidate SARIF:
- **EVM/Solidity:** `krait` (4-phase audit pipeline) + `semgrep` + `codeql` + `token-integration-analyzer` + `insecure-defaults` + `sharp-edges`.
- **Solana/Rust:** `solana-vulnerability-scanner` + `semgrep` + `cargo`-clippy.
- **Cosmos/CosmWasm:** `cosmos-vulnerability-scanner`.
- **Cairo / TON / Algorand / Substrate:** the matching `*-vulnerability-scanner`.
- **THORChain/Rust, Soroban/Rust:** no turnkey scanner → lean on `semgrep` custom rules + manual lenses (Phase 2).
- **Native C/C++:** `c-review` + `address-sanitizer`. **Crypto:** `constant-time-analysis`, `zeroize-audit`.
- **CI/agentic:** `agentic-actions-auditor`. **Deps:** `supply-chain-risk-auditor`.
- Consolidate: `sarif-parsing` → project findings onto the graph with `audit-augmentation`.
- **Triage every hit through Phase 4 before trusting it.** Scanners are lead generators; most hits are noise.

### Phase 2 — Manual multi-lens hunt (the core, where real findings come from)
**Goal:** break the invariant. Run the **Lens Catalog** (below) as focused parallel agents, one lens
each, each briefed self-contained. For every candidate produce a **concrete numeric scenario**
(specific amounts, decimals, ordering) — not a vague "could be wrong."
- Seed from Phase-1 leads + the fork-deviation diff (compare against the audited upstream: Aave/Compound/Uniswap/etc.).
- `variant-analysis` — given one bug, sweep the codebase for siblings.
- `spec-to-code-compliance` — code vs whitepaper/docs (finds unenforced documented behavior — this was K2's only Medium-maybe).

### Phase 3 — PoC + dynamic confirmation
**Goal:** a runnable PoC that demonstrates the broken invariant. **No PoC = no submission** (both modes).
- Write the PoC in the **provided harness** (the contest/bounty's own test suite). Run it; it must compile + fail the invariant.
- Invariant/property fuzzing → `property-based-testing`, `krait-fuzz`, or native fuzzers (`cargo-fuzz`, `libfuzzer`, `aflpp`, `atheris`, `ruzzy`).
- Find untested paths / fuzz targets → `mutation-testing` (mewt/muton) + `genotoxic`.
- Harness help → `harness-writing`, `coverage-analysis`, `fuzzing-obstacles`, `fuzzing-dictionary`.
- Crypto: `wycheproof`, `vector-forge` for test vectors; `crypto-protocol-diagram` → `mermaid-to-proverif` for protocol-level proofs.

### Phase 4 — Verify & kill false positives (GATE)
**Goal:** only airtight findings pass. Mandatory before any submission, **non-negotiable in bounty mode**.
- `fp-check` — forces a TRUE-POSITIVE / FALSE-POSITIVE verdict with documented evidence.
- `second-opinion` — external LLM (Codex/Gemini) review of the candidate + PoC.
- Re-derive the impact and tie it to a **named broken invariant** and a **severity** (see Calibration).
- Kill anything that needs privileged access, social engineering, or an out-of-scope precondition (bounty).

### Phase 5 — Dedup discipline (do this BEFORE writing the finding)
A finding that duplicates a known/prior issue is **ineligible** — wasted effort (and -25 USDC in a bounty).
Cross-check every candidate against, in order:
1. **AI-auditor outputs** (e.g. Zellic V12 in K2) — often linked in the README and **explicitly ineligible**.
2. **Prior human audits** (Halborn, WatchPug, Trail of Bits, etc.) — assume the obvious is gone.
3. **Known-issues list** + previous contest findings for the same/forked protocol.
4. **The upstream fork's** known issues (Aave/Compound advisories).
Maintain a single `dedup.md`. If it's there → drop it, log why.

### Phase 6 — Continuous loop (contest breadth) / diff-watch (bounty)
- **Contest:** run **novel-lens cycles** on a heartbeat (one fresh lens per cycle: ABI-mismatch,
  read-only-reentrancy, event-integrity, constant/unit audit, standard-conformance, …). Each negative
  result is a hardening signal; each positive grows the QA report. Stop when distinct lenses are exhausted.
- **Bounty:** subscribe to the repo. On every **new deploy / commit / PR**, run `graph-evolution`
  (structural diff between tags) + `diff-review` to catch regressions and newly-introduced Crit/High.
  This is where ongoing bounties actually pay — fresh code is under-audited.

### Phase 7 — Report
- **Contest:** one finding per Med/High (Title · Severity · file:line · Description · Impact→invariant ·
  Coded PoC · Mitigation). All Lows in **one** QA report, labeled `L-01, L-02…`.
- **Bounty:** Crit/High only. Each: clear impact (theft/freeze/insolvency $ amount), the **coded
  runnable PoC**, exact reproduction, suggested fix. Submit **once, complete** (no edits). Attach `fp-check` evidence.

---

## The Lens Catalog (Phase 2 — the bug-finding lenses)

Each is one parallel agent. Brief it self-contained: protocol, the invariant, the dedup list, the lens.

| # | Lens | Bug shape it hunts | Skill / aid |
|---|------|--------------------|-------------|
| 1 | **Core-invariant break** | Sequence that makes value created > value backed (insolvency) | `property-based-testing` |
| 2 | **Fork-deviation diff** | Char-level diffs vs the audited upstream (Aave/Compound/Uni); a "harmless" tweak that isn't | `git diff` vs upstream, `variant-analysis` |
| 3 | **Rounding direction** | Mint vs burn / deposit vs withdraw rounding that favors the user not the protocol; `mul` vs `mul_up` misuse | `dimensional-analysis` |
| 4 | **Liquidation math** | Post-liq HF not improving but still succeeds; bonus over-seize; dust-debt below min; bad-debt socialization rounding | manual + PoC |
| 5 | **Interest/index accounting** | Index update ordering vs balance change; first-deposit/empty-reserve donation-inflation; scaled-balance drift | `property-based-testing` |
| 6 | **Patch-consistency** | A fix applied in one path but not its siblings (K2 L-45 was this) | `variant-analysis` |
| 7 | **Dead / unwired code** | Registered-but-never-called contracts; missing `require_auth`/pause guards; empty `validate_*` no-ops | `entry-points` |
| 8 | **Composition / reentrancy** | Cross-contract callbacks; read-only reentrancy (stale view mid-callback); CEI violations | `trailmark` taint |
| 9 | **Oracle / price** | Stale price via wrong-timestamp TTL; circuit-breaker griefing; cascade source override/removal | manual + PoC |
| 10 | **Economic / MEV** | Sandwich, donation/inflation, oracle manipulation, fee-skim, first-depositor | manual |
| 11 | **Protection-assumption map** | Enumerate what each check *assumes*; find the input that violates the assumption | manual |
| 12 | **Griefing / DoS** | Permanent state lock (e.g. dust blocking reserve drop); unbounded loop; gas/VM-budget bricking | manual |
| 13 | **Conservation / desync** | `sum(user_balances) == total_supply` violated at some mutation; ghost value | `property-based-testing` |
| 14 | **Native-platform mechanics** | Soroban storage TTL/auth; Solana PDA/CPI/signer/owner; EVM delegatecall/proxy/storage-collision | chain `*-vulnerability-scanner` |
| 15 | **Boundary / extreme values** | `u128::MAX`/`type(uint).max` sentinels, 0, 1-wei, max decimals, empty set | fuzz |
| 16 | **Lifecycle / init ordering** | Uninitialized-then-used; init front-run; upgrade storage layout; two-step admin gaps | manual |
| 17 | **Standard conformance** | ERC-20/4626/721 or SEP-41 deviation an integrator assumes (rebasing-without-event, fee-on-transfer, return-value) | `token-integration-analyzer` |
| 18 | **Constant / unit consistency** | WAD-where-RAY, bps-where-WAD, off-by-10^decimals, wrong SECONDS_PER_YEAR | `dimensional-analysis` |
| 19 | **Spec-vs-code** | Documented behavior (errors/structs/comments) that is never actually enforced | `spec-to-code-compliance` |
| 20 | **Low-chaining** | Two individually-Low issues that compose into a Med/High | manual |

---

## Agent orchestration patterns ("passar o rodo a limpo")

1. **Parallel investigators.** One agent per lens / per scanner, launched in a single batch.
   Each gets a self-contained brief (it has no memory of the conversation): protocol summary, the
   invariant, the dedup list, the specific lens, the output format, a word cap.
2. **Context protection.** Heavy searches/scans run in sub-agents so their raw output never
   floods the main thread — they report a short distilled result.
3. **The verification gate is serial.** Sweep wide in parallel (Phases 1–2), then funnel every
   candidate through the *single* `fp-check`/`second-opinion` gate (Phase 4) before it counts.
4. **The continuous loop (contest).** A heartbeat re-arms a "run next novel lens" tick; each tick
   spawns one fresh-lens agent and logs the result. Stop when lenses are exhausted — and *say so*,
   don't spin empty cycles.
5. **Honesty rule.** Negative results are logged, not hidden. Inflated severity that gets
   downgraded = ineligible. Trust-but-verify every agent's claim against the actual code before reporting.
6. **Backup discipline.** If your notes live in a gitignored/ephemeral dir, back them up out-of-band
   (e.g. email a copy) — the container is reclaimed on inactivity.

---

## Severity calibration (don't inflate)

- **Critical:** direct theft, permanent freeze, protocol insolvency, unauthorized mint, manipulated RNG.
- **High:** theft of unclaimed yield/royalties, temporary freeze of funds, recoverable insolvency.
- **Medium:** value-at-risk under specific conditions; function-level DoS; (contest-only $).
- **Low / QA / Info:** best-practice, observability, griefing with no fund impact (contest-only $; **$0 in bounty**).
- A Med/High the judge downgrades becomes ineligible. When in doubt, **state the honest severity**
  and let the PoC carry it.

---

## One-line summary of the method (per candidate)
Map entry points + auth → state the invariant in English → build the breaking input as concrete
numbers → **dedup-check** → write the PoC in the provided harness → run it → `fp-check` →
only then draft the finding at its honest severity.
