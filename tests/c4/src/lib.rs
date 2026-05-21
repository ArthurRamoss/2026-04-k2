#![cfg(test)]

//! # Code4rena K2 Audit — PoC Submission Template
//!
//! This crate is the starting point for Proof-of-Concept (PoC) tests
//! submitted to the K2 Lending Protocol audit contest.
//!
//! ## Prerequisites
//!
//! Optimized WASM binaries must be built *before* running tests in this crate,
//! because every contract is loaded via `soroban_sdk::contractimport!`:
//!
//! ```bash
//! ./build.sh
//! ```
//!
//! ## Running
//!
//! ```bash
//! cargo test --package k2-c4
//! ```
//!
//! ## Writing a PoC
//!
//! 1. Extend [`test_submission_validity`] at the bottom of this file.
//! 2. Call `Setup::new(&env)` to get a fully initialized protocol state:
//!    router, price oracle, two reserves (`asset_a` and `asset_b`), interest
//!    rate strategy, seeded LP liquidity, and a funded `user` account whose
//!    balances are pre-approved to the router.
//! 3. Perform whatever sequence of user operations demonstrates the bug.
//! 4. Assert the unexpected/incorrect state (e.g. `health_factor` above `WAD`
//!    when it should not be, funds drained, accounting broken, etc.).
//!
//! All in-scope contracts are imported below, so you can register and wire up
//! additional contracts (incentives, liquidation engine, swap adapters, …)
//! without editing this file's imports.

// =============================================================================
// Contract WASM imports — every in-scope contract is available here.
// =============================================================================

/// Kinetic Router — main lending pool contract and entry point.
pub mod kinetic_router {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32v1-none/release/k2_kinetic_router.optimized.wasm"
    );
}

/// aToken — interest-bearing supply position token.
pub mod a_token {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32v1-none/release/k2_a_token.optimized.wasm"
    );
}

/// Debt Token — non-transferable variable-rate debt position token.
pub mod debt_token {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32v1-none/release/k2_debt_token.optimized.wasm"
    );
}

/// Price Oracle — asset price feeds with circuit breaker protection.
pub mod price_oracle {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32v1-none/release/k2_price_oracle.optimized.wasm"
    );
}

/// Pool Configurator — reserve deployment and parameter management.
pub mod pool_configurator {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32v1-none/release/k2_pool_configurator.optimized.wasm"
    );
}

/// Liquidation Engine — liquidation logic.
pub mod liquidation_engine {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32v1-none/release/k2_liquidation_engine.optimized.wasm"
    );
}

/// Interest Rate Strategy — utilization-based rate curves.
pub mod interest_rate_strategy {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32v1-none/release/k2_interest_rate_strategy.optimized.wasm"
    );
}

/// Incentives — reward distribution.
pub mod incentives {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32v1-none/release/k2_incentives.optimized.wasm"
    );
}

/// Treasury — protocol fee collection.
pub mod treasury {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32v1-none/release/k2_treasury.optimized.wasm"
    );
}

/// Flash Liquidation Helper — validation logic for two-step flash liquidation.
pub mod flash_liquidation_helper {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32v1-none/release/k2_flash_liquidation_helper.optimized.wasm"
    );
}

/// Base Token — standard SEP-41 token implementation.
pub mod base_token {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32v1-none/release/k2_token.optimized.wasm"
    );
}

/// Aquarius DEX swap adapter.
pub mod aquarius_swap_adapter {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32v1-none/release/aquarius_swap_adapter.optimized.wasm"
    );
}

/// Soroswap DEX swap adapter.
pub mod soroswap_swap_adapter {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32v1-none/release/soroswap_swap_adapter.optimized.wasm"
    );
}

// =============================================================================
// Mock Reflector oracle — required by `price_oracle.initialize()`.
// The real Reflector contract lives outside the audit scope; for PoCs we
// only need an object that answers `decimals()`.
// =============================================================================

use soroban_sdk::{contract, contractimpl};

#[contract]
pub struct MockReflector;

#[contractimpl]
impl MockReflector {
    pub fn decimals(_env: Env) -> u32 {
        14
    }
}

// =============================================================================
// Setup scaffold — one call gives wardens a ready-to-use protocol state.
// =============================================================================

use price_oracle::Asset as OracleAsset;
use soroban_sdk::{
    testutils::{Address as _, Ledger, LedgerInfo},
    token, Address, Env, IntoVal, String, Symbol, Vec,
};

/// Convenience constants used by [`Setup::new`]. Exposed so wardens can
/// reference them when asserting expected balances / prices.
pub const ASSET_DECIMALS: u32 = 7;
/// $1.00 expressed in the oracle's 14-decimal fixed-point format.
pub const PRICE_ONE_DOLLAR: u128 = 100_000_000_000_000;
/// Initial LP deposit per reserve (100M whole tokens at 7 decimals).
pub const LP_SEED: i128 = 1_000_000_000_000_000;
/// Starting balance minted to `user` in each reserve (1 000 whole tokens).
pub const USER_STARTING_BALANCE: i128 = 10_000_000_000;

/// Fully-initialized protocol state.
///
/// Two reserves are registered:
/// - **`asset_a`**: higher-quality collateral (LTV 80%, liq. threshold 85%)
/// - **`asset_b`**: lower-quality collateral (LTV 50%, liq. threshold 65%)
///
/// Both priced at $1.00 so dollar-value math lines up trivially.
///
/// Protocol liquidity is seeded by `liquidity_provider` (100M of each asset),
/// and `user` holds [`USER_STARTING_BALANCE`] of each asset with the router
/// already approved as a spender — so `router.supply`, `router.borrow`,
/// `router.repay`, `router.withdraw`, etc. can be called immediately.
pub struct Setup<'a> {
    pub env: &'a Env,

    // --- Principals ---
    pub admin: Address,
    pub emergency_admin: Address,
    pub user: Address,
    pub liquidity_provider: Address,

    // --- Core contracts ---
    pub router: kinetic_router::Client<'a>,
    pub router_addr: Address,
    pub oracle: price_oracle::Client<'a>,
    pub oracle_addr: Address,
    pub pool_configurator: Address,
    pub interest_rate_strategy: Address,
    pub treasury: Address,
    pub dex_router: Address,

    // --- Reserve A (high quality) ---
    pub asset_a: Address,
    pub asset_a_token: token::Client<'a>,
    pub asset_a_mint: token::StellarAssetClient<'a>,
    pub a_token_a: Address,
    pub debt_token_a: Address,

    // --- Reserve B (lower quality) ---
    pub asset_b: Address,
    pub asset_b_token: token::Client<'a>,
    pub asset_b_mint: token::StellarAssetClient<'a>,
    pub a_token_b: Address,
    pub debt_token_b: Address,
}

impl<'a> Setup<'a> {
    /// Build a fully-initialized protocol state.
    ///
    /// Mocks all auths, resets the budget, and sets a fresh ledger — callers
    /// can simply do `let setup = Setup::new(&env);` and start exploiting.
    pub fn new(env: &'a Env) -> Self {
        env.mock_all_auths();
        #[allow(deprecated)]
        env.budget().reset_unlimited();
        env.ledger().set(LedgerInfo {
            sequence_number: 100,
            protocol_version: 23,
            timestamp: 1000,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 10,
            min_persistent_entry_ttl: 10,
            max_entry_ttl: 1_000_000,
        });

        let admin = Address::generate(env);
        let emergency_admin = admin.clone();
        let user = Address::generate(env);
        let liquidity_provider = Address::generate(env);

        // --- Deploy + initialize the price oracle ---
        let oracle_addr = env.register(price_oracle::WASM, ());
        let oracle = price_oracle::Client::new(env, &oracle_addr);
        let reflector_addr = env.register(MockReflector, ());
        let base_currency = Address::generate(env);
        let native_xlm = Address::generate(env);
        oracle.initialize(&admin, &reflector_addr, &base_currency, &native_xlm);

        // --- Deploy + initialize the main router ---
        let router_addr = env.register(kinetic_router::WASM, ());
        let router = kinetic_router::Client::new(env, &router_addr);
        let treasury = Address::generate(env);
        let dex_router = Address::generate(env);
        router.initialize(
            &admin,
            &emergency_admin,
            &oracle_addr,
            &treasury,
            &dex_router,
            &None,
        );

        // The pool configurator is stored as an authorized principal. Existing
        // PoC tests use a raw generated address here (auth passes under
        // `mock_all_auths`). Wardens may replace it with a deployed
        // `pool_configurator::WASM` instance if their scenario requires it.
        let pool_configurator = Address::generate(env);
        router.set_pool_configurator(&pool_configurator);

        // --- Interest rate strategy (shared across reserves) ---
        let interest_rate_strategy = env.register(interest_rate_strategy::WASM, ());
        let mut irs_args = Vec::new(env);
        irs_args.push_back(admin.clone().into_val(env));
        irs_args.push_back((0u128).into_val(env)); // base rate
        irs_args.push_back((40_000_000_000_000_000_000u128).into_val(env)); // slope1
        irs_args.push_back((100_000_000_000_000_000_000u128).into_val(env)); // slope2
        irs_args.push_back((800_000_000_000_000_000_000_000_000u128).into_val(env)); // optimal
        let _: () = env.invoke_contract(
            &interest_rate_strategy,
            &Symbol::new(env, "initialize"),
            irs_args,
        );

        // --- Register two reserves ---
        let (asset_a, a_token_a, debt_token_a) = Self::register_reserve(
            env,
            &admin,
            &pool_configurator,
            &router,
            &router_addr,
            &oracle,
            &interest_rate_strategy,
            8000, // LTV 80%
            8500, // liq threshold 85%
        );
        let (asset_b, a_token_b, debt_token_b) = Self::register_reserve(
            env,
            &admin,
            &pool_configurator,
            &router,
            &router_addr,
            &oracle,
            &interest_rate_strategy,
            5000, // LTV 50%
            6500, // liq threshold 65%
        );

        let asset_a_token = token::Client::new(env, &asset_a);
        let asset_a_mint = token::StellarAssetClient::new(env, &asset_a);
        let asset_b_token = token::Client::new(env, &asset_b);
        let asset_b_mint = token::StellarAssetClient::new(env, &asset_b);

        // --- Seed protocol liquidity from the LP ---
        asset_a_mint.mint(&liquidity_provider, &LP_SEED);
        asset_b_mint.mint(&liquidity_provider, &LP_SEED);
        let approval_expiration = env.ledger().sequence() + 100_000;
        asset_a_token.approve(&liquidity_provider, &router_addr, &LP_SEED, &approval_expiration);
        asset_b_token.approve(&liquidity_provider, &router_addr, &LP_SEED, &approval_expiration);
        router.supply(
            &liquidity_provider,
            &asset_a,
            &(LP_SEED as u128),
            &liquidity_provider,
            &0u32,
        );
        router.supply(
            &liquidity_provider,
            &asset_b,
            &(LP_SEED as u128),
            &liquidity_provider,
            &0u32,
        );

        // --- Fund the user + pre-approve the router ---
        asset_a_mint.mint(&user, &USER_STARTING_BALANCE);
        asset_b_mint.mint(&user, &USER_STARTING_BALANCE);
        asset_a_token.approve(&user, &router_addr, &USER_STARTING_BALANCE, &approval_expiration);
        asset_b_token.approve(&user, &router_addr, &USER_STARTING_BALANCE, &approval_expiration);

        Setup {
            env,
            admin,
            emergency_admin,
            user,
            liquidity_provider,
            router,
            router_addr,
            oracle,
            oracle_addr,
            pool_configurator,
            interest_rate_strategy,
            treasury,
            dex_router,
            asset_a,
            asset_a_token,
            asset_a_mint,
            a_token_a,
            debt_token_a,
            asset_b,
            asset_b_token,
            asset_b_mint,
            a_token_b,
            debt_token_b,
        }
    }

    /// Deploy an underlying Stellar-asset token plus its aToken / debtToken
    /// pair, register the reserve on the router, and publish a $1.00 oracle
    /// price for it.
    fn register_reserve(
        env: &'a Env,
        admin: &Address,
        pool_configurator: &Address,
        router: &kinetic_router::Client<'a>,
        router_addr: &Address,
        oracle: &price_oracle::Client<'a>,
        interest_rate_strategy: &Address,
        ltv: u32,
        liquidation_threshold: u32,
    ) -> (Address, Address, Address) {
        let underlying_admin = Address::generate(env);
        let underlying = env.register_stellar_asset_contract_v2(underlying_admin);
        let underlying_addr = underlying.address();

        let a_token_addr = env.register(a_token::WASM, ());
        a_token::Client::new(env, &a_token_addr).initialize(
            admin,
            &underlying_addr,
            router_addr,
            &String::from_str(env, "aToken"),
            &String::from_str(env, "aTKN"),
            &ASSET_DECIMALS,
        );

        let debt_token_addr = env.register(debt_token::WASM, ());
        debt_token::Client::new(env, &debt_token_addr).initialize(
            admin,
            &underlying_addr,
            router_addr,
            &String::from_str(env, "debtToken"),
            &String::from_str(env, "dTKN"),
            &ASSET_DECIMALS,
        );

        let reserve_treasury = Address::generate(env);
        let params = kinetic_router::InitReserveParams {
            decimals: ASSET_DECIMALS,
            ltv,
            liquidation_threshold,
            liquidation_bonus: 500,
            reserve_factor: 1000,
            supply_cap: 0,
            borrow_cap: 0,
            borrowing_enabled: true,
            flashloan_enabled: true,
        };
        router.init_reserve(
            pool_configurator,
            &underlying_addr,
            &a_token_addr,
            &debt_token_addr,
            interest_rate_strategy,
            &reserve_treasury,
            &params,
        );

        let asset_enum = OracleAsset::Stellar(underlying_addr.clone());
        oracle.add_asset(admin, &asset_enum);
        oracle.set_manual_override(
            admin,
            &asset_enum,
            &Some(PRICE_ONE_DOLLAR),
            &Some(env.ledger().timestamp() + 604_800), // 7-day expiry
        );

        (underlying_addr, a_token_addr, debt_token_addr)
    }
}

// =============================================================================
// PoC template
// =============================================================================

/// ## Code4rena warden template — extend me.
///
/// This test must remain a `#[test]` that PASSES by default. Wardens submit
/// their PoCs by modifying the body below to trigger and assert the bug.
///
/// The sanity-check block at the start validates that the scaffold is wired
/// correctly. Leave it in place — if it starts failing, the build is broken
/// and *no* PoC in the repo will run. Add your exploit steps after the
/// "WARDEN: add your PoC below this line" marker.
#[test]
fn test_submission_validity() {
    let env = Env::default();
    let setup = Setup::new(&env);

    // ---------- Sanity check: protocol is in a known, functional state ----------

    // The user has starting balances of both assets, pre-approved to the router.
    assert_eq!(
        setup.asset_a_token.balance(&setup.user),
        USER_STARTING_BALANCE,
        "user should start with USER_STARTING_BALANCE of asset_a",
    );
    assert_eq!(
        setup.asset_b_token.balance(&setup.user),
        USER_STARTING_BALANCE,
        "user should start with USER_STARTING_BALANCE of asset_b",
    );

    // A plain supply round-trips through the router cleanly, proving that the
    // router, oracle, reserves, aToken, debtToken, and interest-rate strategy
    // are all correctly wired up.
    let deposit: u128 = 5_000_000_000; // 500 whole tokens of asset_a
    setup
        .router
        .supply(&setup.user, &setup.asset_a, &deposit, &setup.user, &0u32);

    let account = setup.router.get_user_account_data(&setup.user);
    assert!(
        account.total_collateral_base > 0,
        "collateral should be tracked after supply",
    );
    assert_eq!(account.total_debt_base, 0, "no debt yet");
    assert_eq!(
        account.health_factor,
        u128::MAX,
        "health factor should be infinite with no debt",
    );

    // ---------- WARDEN: add your PoC below this line ----------
    //
    // Example skeleton:
    //
    //     // 1. Put the protocol in a state that triggers the bug.
    //     setup.router.borrow(
    //         &setup.user,
    //         &setup.asset_b,
    //         &borrow_amount,
    //         &1u32,        // interest rate mode: variable
    //         &0u32,        // referral code
    //         &setup.user,
    //     );
    //
    //     // 2. Perform the exploit step.
    //     let result = setup.router.try_withdraw(
    //         &setup.user,
    //         &setup.asset_a,
    //         &withdraw_amount,
    //         &setup.user,
    //     );
    //
    //     // 3. Assert the incorrect outcome.
    //     assert!(result.is_ok(), "bug: unsafe withdraw succeeded");
    //     let post = setup.router.get_user_account_data(&setup.user);
    //     assert!(post.health_factor < k2_shared::WAD, "bug: HF below 1.0");
}

// =============================================================================
// PoC — T1 bitmap reserve-index collision re-verification (fresh eyes).
//
// Hypothesis under test (cross-pollinated from EVM swap-and-pop index reuse and
// Solana account re-init after close): after drop_reserve frees a reserve, can a
// newly registered reserve REUSE the freed UserConfiguration bitmap bit position,
// so a user's STALE bit (set on the dropped reserve) reads as participation in the
// NEW reserve -> phantom collateral / hidden debt -> broken HF?
//
// We deliberately exercise the weakest link: set_user_use_reserve_as_coll lets a
// user set the collateral bit with ZERO aToken balance (router.rs:1276), so the
// stale bit survives drop_reserve's zero-scaled-supply guard. We then register a
// fresh reserve and check whether its bit position collides with the stale bit.
//
// EXPECTED (if the prior "collision dead" conclusion is correct): the new reserve
// gets a strictly higher monotonic id (different bit), and the stale bit maps to a
// removed RESERVE_ID_TO_ADDRESS entry so HF accounting ignores it entirely.
// =============================================================================

/// Register a brand-new reserve at runtime (mirrors Setup::register_reserve but
/// callable mid-test). Returns (underlying, a_token, debt_token).
fn register_extra_reserve<'a>(
    env: &'a Env,
    setup: &Setup<'a>,
    ltv: u32,
    liquidation_threshold: u32,
) -> (Address, Address, Address) {
    let underlying_admin = Address::generate(env);
    let underlying = env.register_stellar_asset_contract_v2(underlying_admin);
    let underlying_addr = underlying.address();

    let a_token_addr = env.register(a_token::WASM, ());
    a_token::Client::new(env, &a_token_addr).initialize(
        &setup.admin,
        &underlying_addr,
        &setup.router_addr,
        &String::from_str(env, "aToken"),
        &String::from_str(env, "aTKN"),
        &ASSET_DECIMALS,
    );
    let debt_token_addr = env.register(debt_token::WASM, ());
    debt_token::Client::new(env, &debt_token_addr).initialize(
        &setup.admin,
        &underlying_addr,
        &setup.router_addr,
        &String::from_str(env, "debtToken"),
        &String::from_str(env, "dTKN"),
        &ASSET_DECIMALS,
    );
    let reserve_treasury = Address::generate(env);
    let params = kinetic_router::InitReserveParams {
        decimals: ASSET_DECIMALS,
        ltv,
        liquidation_threshold,
        liquidation_bonus: 500,
        reserve_factor: 1000,
        supply_cap: 0,
        borrow_cap: 0,
        borrowing_enabled: true,
        flashloan_enabled: true,
    };
    setup.router.init_reserve(
        &setup.pool_configurator,
        &underlying_addr,
        &a_token_addr,
        &debt_token_addr,
        &setup.interest_rate_strategy,
        &reserve_treasury,
        &params,
    );
    let asset_enum = OracleAsset::Stellar(underlying_addr.clone());
    setup.oracle.add_asset(&setup.admin, &asset_enum);
    setup.oracle.set_manual_override(
        &setup.admin,
        &asset_enum,
        &Some(PRICE_ONE_DOLLAR),
        &Some(env.ledger().timestamp() + 604_800),
    );
    (underlying_addr, a_token_addr, debt_token_addr)
}

#[test]
fn poc_bitmap_drop_reregister_index_reuse() {
    let env = Env::default();
    let setup = Setup::new(&env);

    // Reserves A and B already exist with ids 0 and 1 (bits 0 and 1).
    let id_a = setup.router.get_reserve_data(&setup.asset_a).id;
    let id_b = setup.router.get_reserve_data(&setup.asset_b).id;
    assert_eq!(id_a, 0, "asset_a should be reserve id 0");
    assert_eq!(id_b, 1, "asset_b should be reserve id 1");

    // ---- Step 1: register reserve C (id 2, bit 2). ----
    let (asset_c, _a_c, _d_c) = register_extra_reserve(&env, &setup, 8000, 8500);
    let id_c = setup.router.get_reserve_data(&asset_c).id;
    assert_eq!(id_c, 2, "asset_c should be reserve id 2");

    // ---- Step 2: user sets C's COLLATERAL bit with ZERO balance. ----
    // This is the crack: set_user_use_reserve_as_coll does not require any aToken
    // balance, so the bit can be set on a reserve the user never supplied to.
    // It therefore survives drop_reserve's zero-scaled-supply guard.
    setup
        .router
        .set_user_use_reserve_as_coll(&setup.user, &asset_c, &true);

    // Sanity: with no balance and no debt, HF is still MAX (bit set but worthless).
    let acct_before = setup.router.get_user_account_data(&setup.user);
    assert_eq!(acct_before.total_collateral_base, 0, "no real C collateral");
    assert_eq!(acct_before.health_factor, u128::MAX, "no debt -> HF MAX");

    // ---- Step 3: drop reserve C (scaled supply == 0, so guard passes). ----
    setup
        .router
        .drop_reserve(&setup.pool_configurator, &asset_c);
    // C's id->address mapping is now removed; the user's stale bit-2 persists.
    assert!(
        setup.router.try_get_reserve_data(&asset_c).is_err(),
        "reserve C should be gone after drop",
    );

    // ---- Step 4: register a NEW reserve D and see which bit it takes. ----
    let (asset_d, _a_d, _d_d) = register_extra_reserve(&env, &setup, 8000, 8500);
    let id_d = setup.router.get_reserve_data(&asset_d).id;

    // CORE ASSERTION: the freed id/bit (2) is NOT reused. D gets a strictly higher,
    // fresh monotonic id. No bit-position collision is possible.
    assert_eq!(
        id_d, 3,
        "BITMAP COLLISION: new reserve reused freed id {} (got id {}). \
         If this were id 2, the user's stale bit would phantom-collateralize D.",
        id_c, id_d
    );
    assert_ne!(id_d, id_c, "new reserve must not reuse the dropped reserve's id");

    // ---- Step 5: confirm the new reserve D also starts clean (index == RAY). ----
    let rd_d = setup.router.get_reserve_data(&asset_d);
    assert_eq!(rd_d.liquidity_index, RAY_U128, "D liquidity_index == RAY");
    assert_eq!(
        rd_d.variable_borrow_index,
        RAY_U128,
        "D variable_borrow_index == RAY"
    );

    // ---- Step 6: prove the stale bit-2 cannot inflate borrowing power against D. ----
    // Give the user REAL collateral in D, borrow against it to the limit, and verify
    // the stale C bit (bit 2, now mapping to nothing) contributes ZERO collateral.
    // We supply to D by minting D's underlying to the user and approving the router.
    let asset_d_mint = token::StellarAssetClient::new(&env, &asset_d);
    let asset_d_token = token::Client::new(&env, &asset_d);
    asset_d_mint.mint(&setup.user, &USER_STARTING_BALANCE);
    let exp = env.ledger().sequence() + 100_000;
    asset_d_token.approve(&setup.user, &setup.router_addr, &USER_STARTING_BALANCE, &exp);
    let d_supply: u128 = 1_000_000_000; // 100 tokens @ $1
    setup
        .router
        .supply(&setup.user, &asset_d, &d_supply, &setup.user, &0u32);

    let acct_after = setup.router.get_user_account_data(&setup.user);
    // total_collateral_base is denominated in the oracle's value base (14-dp $),
    // not raw token units. 100 tokens @ $1 == 100 * 1e18 in WAD base value.
    // The exact single-count value proves the stale bit-2 (set on dropped C) adds
    // nothing: if it were mis-attributed to D we'd see ~2x this, and if it resolved
    // to a live-but-wrong asset we'd see a different magnitude or a panic.
    let expected_collateral_base: u128 = 100 * WAD_U128; // 100 tokens * $1 in WAD
    assert_eq!(
        acct_after.total_collateral_base, expected_collateral_base,
        "stale dropped-reserve bit must contribute ZERO extra collateral; \
         got {} expected {} (phantom collateral would inflate this ~2x)",
        acct_after.total_collateral_base, expected_collateral_base
    );

    // And the stale bit didn't break HF math either (still finite & sane with no debt).
    assert_eq!(acct_after.health_factor, u128::MAX, "no debt -> HF MAX, no corruption");
}

// =============================================================================
// Dynamic invariant fuzz (audit Round 5 — dynamic lens).
// Drives a deterministic pseudo-random sequence of supply/borrow/withdraw/repay
// across both reserves with interest accrual between ops, asserting the crown
// solvency invariant after every step:
//     aToken_scaled * liquidity_index / RAY  <=  underlying_balance + debt_scaled * borrow_index / RAY  (+ rounding slack)
// and that no *successful* borrow/withdraw leaves the actor with HF < 1.0 WAD.
// =============================================================================

const RAY_U128: u128 = 1_000_000_000_000_000_000_000_000_000; // 1e27
const WAD_U128: u128 = 1_000_000_000_000_000_000; // 1e18

fn u256(env: &Env, v: u128) -> soroban_sdk::U256 {
    soroban_sdk::U256::from_u128(env, v)
}

fn nn(v: i128) -> u128 {
    if v < 0 { 0 } else { v as u128 }
}

fn assert_conservation(
    env: &Env,
    router: &kinetic_router::Client,
    asset: &Address,
    a_addr: &Address,
    d_addr: &Address,
    underlying: &token::Client,
    label: &str,
    step: u32,
) {
    let rd = router.get_reserve_data(asset);
    let li = u256(env, rd.liquidity_index);
    let bi = u256(env, rd.variable_borrow_index);
    let ray = u256(env, RAY_U128);

    let a_scaled = nn(a_token::Client::new(env, a_addr).scaled_total_supply());
    let d_scaled = nn(debt_token::Client::new(env, d_addr).scaled_total_supply());
    let und = nn(underlying.balance(a_addr));

    // LHS = total aToken claims valued at the current liquidity index
    let lhs = u256(env, a_scaled).mul(&li).div(&ray);
    // RHS = real underlying held by the aToken + outstanding debt valued at the borrow index
    let debt_val = u256(env, d_scaled).mul(&bi).div(&ray);
    let rhs = u256(env, und).add(&debt_val);

    // Allow a few units of rounding slack (scaled mint/burn round in the protocol's favor).
    let tol = u256(env, 10_000u128);
    assert!(
        lhs <= rhs.add(&tol),
        "SOLVENCY INVARIANT BROKEN [{}] at step {}: aToken_value exceeds underlying+debt_value (a_scaled={}, d_scaled={}, underlying={})",
        label, step, a_scaled, d_scaled, und
    );
}

#[test]
fn invariant_fuzz_solvency_and_hf() {
    let env = Env::default();
    let setup = Setup::new(&env);

    // Top up the user and refresh approvals so a long op sequence isn't starved.
    setup.asset_a_mint.mint(&setup.user, &1_000_000_000_000i128);
    setup.asset_b_mint.mint(&setup.user, &1_000_000_000_000i128);
    let exp = env.ledger().sequence() + 100_000;
    setup.asset_a_token.approve(&setup.user, &setup.router_addr, &2_000_000_000_000i128, &exp);
    setup.asset_b_token.approve(&setup.user, &setup.router_addr, &2_000_000_000_000i128, &exp);

    // Second actor so two independent accounts churn shared reserve state.
    setup.asset_a_mint.mint(&setup.liquidity_provider, &1_000_000_000_000i128);
    setup.asset_b_mint.mint(&setup.liquidity_provider, &1_000_000_000_000i128);
    setup.asset_a_token.approve(&setup.liquidity_provider, &setup.router_addr, &2_000_000_000_000i128, &exp);
    setup.asset_b_token.approve(&setup.liquidity_provider, &setup.router_addr, &2_000_000_000_000i128, &exp);

    let actors = [setup.user.clone(), setup.liquidity_provider.clone()];
    let asset_a_enum = OracleAsset::Stellar(setup.asset_a.clone());
    let asset_b_enum = OracleAsset::Stellar(setup.asset_b.clone());
    let seeds: [u64; 8] = [
        0x9E3779B97F4A7C15, 0xD1B54A32D192ED03, 0xCBF29CE484222325, 0x0000000100000001,
        0xA0761D6478BD642F, 0xE7037ED1A0B428DB, 0x8EBC6AF09C88C6E3, 0x589965CC75374CC3,
    ];
    let mut seq: u32 = 200;
    let mut ts: u64 = 1_000;

    for &s0 in seeds.iter() {
        let mut seed = s0;
        for i in 0..120u32 {
            // Advance the ledger, then refresh both oracle overrides so prices stay fresh
            // at the new timestamp (otherwise staleness checks reject every price-dependent op).
            seq += 1;
            ts += 1_800;
            env.ledger().set(LedgerInfo {
                sequence_number: seq,
                protocol_version: 23,
                timestamp: ts,
                network_id: Default::default(),
                base_reserve: 10,
                min_temp_entry_ttl: 10,
                min_persistent_entry_ttl: 10,
                max_entry_ttl: 1_000_000,
            });
            setup.oracle.set_manual_override(&setup.admin, &asset_a_enum, &Some(PRICE_ONE_DOLLAR), &Some(ts + 86_400));
            setup.oracle.set_manual_override(&setup.admin, &asset_b_enum, &Some(PRICE_ONE_DOLLAR), &Some(ts + 86_400));

            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let op = (seed >> 41) % 4;
            let actor = &actors[((seed >> 5) & 1) as usize];
            let asset = if ((seed >> 3) & 1) == 0 { &setup.asset_a } else { &setup.asset_b };
            // Mostly modest amounts; sometimes dust (1) or the u128::MAX "all" sentinel.
            let amt: u128 = match (seed >> 9) % 8 {
                0 => 1,
                1 => u128::MAX,
                _ => ((seed >> 13) % 2_000_000_000u64) as u128 + 1,
            };

            let mut did_risky_ok = false;
            match op {
                0 => { let _ = setup.router.try_supply(actor, asset, &amt, actor, &0u32); }
                1 => { let r = setup.router.try_borrow(actor, asset, &amt, &1u32, &0u32, actor); did_risky_ok = matches!(r, Ok(Ok(_))); }
                2 => { let r = setup.router.try_withdraw(actor, asset, &amt, actor); did_risky_ok = matches!(r, Ok(Ok(_))); }
                _ => { let _ = setup.router.try_repay(actor, asset, &amt, &1u32, actor); }
            }

            // A SUCCESSFUL borrow/withdraw must leave the actor with HF >= 1.0 WAD.
            if did_risky_ok {
                let acct = setup.router.get_user_account_data(actor);
                assert!(
                    acct.health_factor >= WAD_U128,
                    "HF INVARIANT BROKEN (seed {:#x}, step {}): successful borrow/withdraw left HF={} < 1.0 WAD",
                    s0, i, acct.health_factor
                );
            }

            // Crown solvency invariant must hold after every op (success or revert).
            assert_conservation(&env, &setup.router, &setup.asset_a, &setup.a_token_a, &setup.debt_token_a, &setup.asset_a_token, "A", i);
            assert_conservation(&env, &setup.router, &setup.asset_b, &setup.a_token_b, &setup.debt_token_b, &setup.asset_b_token, "B", i);
        }
    }
}

// =============================================================================
// PoC — Blend-H-01 class (Soroban same-tx stale-reserve / update_state_without_store).
// swap_collateral updates both reserves with `update_state_without_store` (the new index
// is held in memory and NOT persisted until the end of the call). This is exactly the
// Soroban load/cache/store discipline whose violation caused Blend V2 H-01. We route a
// swap through a whitelisted handler and assert the per-reserve solvency invariant holds
// across the un-stored window — any stale-stored-index accounting drift would break it.
// =============================================================================

#[soroban_sdk::contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum MockHandlerError {
    Fail = 1,
}

#[contract]
pub struct MockSwapHandler;

#[contractimpl]
impl MockSwapHandler {
    /// Benign 1:1 swap: delivers `amount_in` of `to_token` to `recipient`
    /// (both Setup assets are $1.00 at 7 decimals, so 1:1 is fair).
    pub fn execute_swap(
        env: Env,
        _from_token: Address,
        to_token: Address,
        amount_in: u128,
        min_amount_out: u128,
        recipient: Address,
    ) -> Result<u128, MockHandlerError> {
        let out = amount_in;
        if out < min_amount_out {
            return Err(MockHandlerError::Fail);
        }
        token::Client::new(&env, &to_token).transfer(
            &env.current_contract_address(),
            &recipient,
            &(out as i128),
        );
        Ok(out)
    }
}

#[test]
fn poc_swap_collateral_conservation_via_handler() {
    let env = Env::default();
    let setup = Setup::new(&env);

    // User supplies a large asset_a position to swap from.
    let supply_amt: u128 = 5_000_000_000; // 500 tokens @7dp
    setup
        .router
        .supply(&setup.user, &setup.asset_a, &supply_amt, &setup.user, &0u32);

    // Deploy + fund + whitelist a benign 1:1 swap handler (holds asset_b to deliver).
    let handler = env.register(MockSwapHandler, ());
    setup.asset_b_mint.mint(&handler, &100_000_000_000i128);
    let mut wl = Vec::new(&env);
    wl.push_back(handler.clone());
    setup.router.set_swap_handler_whitelist(&wl);

    assert_conservation(&env, &setup.router, &setup.asset_a, &setup.a_token_a, &setup.debt_token_a, &setup.asset_a_token, "A-pre", 0);
    assert_conservation(&env, &setup.router, &setup.asset_b, &setup.a_token_b, &setup.debt_token_b, &setup.asset_b_token, "B-pre", 0);

    // swap_collateral asset_a -> asset_b through the handler (uses update_state_without_store).
    let swap_amt: u128 = 2_000_000_000; // 200 tokens
    let min_out: u128 = 1_000_000_000; // 100 tokens (1:1 minus fee)
    let r = setup.router.try_swap_collateral(
        &setup.user,
        &setup.asset_a,
        &setup.asset_b,
        &swap_amt,
        &min_out,
        &Some(handler.clone()),
    );
    assert!(matches!(r, Ok(Ok(_))), "swap_collateral via handler should succeed; got {:?}", r);

    // KEY: solvency invariant must still hold on BOTH reserves after the without_store swap.
    assert_conservation(&env, &setup.router, &setup.asset_a, &setup.a_token_a, &setup.debt_token_a, &setup.asset_a_token, "A-post", 1);
    assert_conservation(&env, &setup.router, &setup.asset_b, &setup.a_token_b, &setup.debt_token_b, &setup.asset_b_token, "B-post", 1);
}
