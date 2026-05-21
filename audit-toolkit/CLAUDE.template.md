# {{PROTOCOL}} Audit — Working Brief

> Copy this file to the target repo's `CLAUDE.md`, fill every `{{...}}`, delete what doesn't apply.
> This is the operating context that drives the whole engagement. The full methodology lives in
> the toolkit `PLAYBOOK.md`; the executable driver is the `/bughunt` skill.

You are assisting a security review of **{{PROTOCOL}}** — {{ONE-LINE WHAT IT IS}} on
**{{CHAIN / LANGUAGE}}** (e.g. EVM/Solidity, Solana/Rust, Soroban/Rust, CosmWasm). Forked from
**{{UPSTREAM or "original"}}**. Scope ≈ {{SLoC}}. Read this fully before touching code.

## ENGAGEMENT MODE — set this, everything branches on it
- [ ] **CONTEST** (time-boxed; all severities pay; QA pool exists; ends **{{DATE}}**)
- [ ] **BOUNTY** (ongoing; **Critical/High ONLY pay**; {{25 USDC deposit, no edits after submit}})

> If BOUNTY: ignore Low/QA breadth entirely. Only theft / freeze / insolvency / unauthorized-mint
> count. Run `fp-check` + `second-opinion` on every candidate before submitting. Exploit must be
> code-only (no privileged access / social engineering unless scope says otherwise).

## Auditor profile (calibrate explanations to this)
- {{e.g. "Mid-level dev, strong on lending business logic; new to <platform>'s account/storage
  model — explain platform-specific mechanics, don't assume."}}
- Coded **runnable PoC required for every High/Medium** (and every bounty submission). No PoC = $0.

## Hard constraints — read before reporting anything
1. **Prior AI-auditor output is OUT OF SCOPE / ineligible:** {{link the V12-style files}}. Check
   every candidate against it first; if found → drop.
2. **Already audited by:** {{Halborn / WatchPug / ToB / …}}. Assume the obvious is gone; hunt what
   survived multiple passes — subtle rounding, state-transition edges, cross-contract assumptions.
3. **Known issues (ineligible, do NOT report):**
   - {{e.g. self-liquidation allowed (upstream precedent)}}
   - {{e.g. memory/budget limit at N reserves — platform limit, not a bug}}
   - {{...}}
4. **PoC mechanics:** extend `{{path/to/provided/harness}}`; run with `{{test command}}`. The PoC
   must compile + run against the provided suite or the finding is rejected.
5. **Out of scope (bounty-typical):** centralization, governance/economic (51%), Sybil, external
   depeg, best-practices, test/config files, oracle-data-errors, already-exploited, privileged access.
   Plus repo-specific: {{paths}}.
6. **Severity honesty:** a Med/High downgraded to Low = ineligible. Don't inflate.

## Setup / commands
```
{{clone / toolchain / build}}
{{test command for the PoC harness}}        # OUR submission harness
{{full unit / integration test commands}}
```

## Architecture (one paragraph)
{{Single entry point? Core accounting model (scaled balances / indexes / shares)? User-state
representation (bitmap / mapping)? Oracle resolution? Admin roles & who can pause/unpause? Where
the value-math precision lives (WAD/RAY/decimals)?}}

## The core invariant (the crown jewel)
{{State it in one sentence. E.g. lending: `aToken_supply_scaled × liquidity_index` must not exceed
`underlying_balance + total_debt_scaled × borrow_index` (mod fees). Any sequence that breaks it = insolvency = High/Crit.}}

## Target ranking (where we dig, in order — by signal-to-effort)
Spend time top-down. For each: the file(s), the invariant it can break, the bug shape to look for.

### T1 — {{highest-signal target}}
- Files: {{file:line}}
- Invariant at risk: {{...}}
- Bug shape: {{concrete description — what input sequence to try}}

### T2 — {{...}}  (repeat T3, T4, T5 …)

## Method per target (the loop — do this for each)
1. Map every external/state-changing entry point in the file + its `require_auth`/access coverage.
2. State the relevant invariant in plain English; ask "what input sequence breaks it?"
3. Build the candidate as a **concrete numeric scenario** (specific amounts, decimals, order).
4. **Dedup-check** vs the AI-auditor output + prior audits + known issues. If duplicate → drop.
5. If it survives, write the PoC in the provided harness and run it.
6. Only if the PoC demonstrates the broken invariant → run `fp-check` → draft the finding.

## Finding format
- **Contest:** one finding per Med/High (Title · Severity · file:line · Description · Impact→invariant ·
  Coded PoC · Mitigation). All Lows → one QA report, labeled `L-01, L-02…`.
- **Bounty:** Crit/High only — impact ($ theft/freeze), coded PoC, exact repro, fix. Submit once, complete.

## Mindset
Don't try to read {{N}}k lines. Break ONE invariant with a runnable PoC, ideally in T1–T3. Depth
over breadth. If after deep work nothing holds: contest → consolidate a strong QA report; bounty →
move to the next target / watch for new deploys. Log negatives; never inflate severity.
