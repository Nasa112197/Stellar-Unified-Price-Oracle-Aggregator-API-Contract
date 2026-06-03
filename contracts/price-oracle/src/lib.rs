#![no_std]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, Address, Env, Map, String, Vec,
    panic_with_error,
};

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub enum DataKey {
    Admin,
    Source(Address),
    AssetRegistered(Address),
    Submission(Address, Address),
    Aggregate(Address),
    PriceHistory(Address, u32),
    OracleSources,
    MinSourcesRequired,
    MaxHistoryLength,
    Decimals,
    Description,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct PriceEntry {
    pub price: i128,
    pub timestamp: u64,
    pub source: Address,
    pub decimals: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct AggregatePrice {
    pub price: i128,
    pub timestamp: u64,
    pub num_sources: u32,
    pub decimals: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct PriceHistoryEntry {
    pub price: i128,
    pub timestamp: u64,
    pub ledger: u32,
    pub num_sources: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct OracleSources {
    pub sources: Vec<Address>,
    pub metadata: Map<Address, String>,
}

#[contracterror]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ErrorCode {
    NotAuthorized = 0,
    AlreadyInitialized = 1,
    AssetNotRegistered = 2,
    AssetAlreadyRegistered = 3,
    SourceAlreadyExists = 4,
    SourceNotFound = 5,
    InsufficientSources = 6,
    InvalidPrice = 7,
    NoData = 8,
}

const DEFAULT_MAX_HISTORY: u32 = 100;
const DEFAULT_MIN_SOURCES: u32 = 1;
const DEFAULT_DECIMALS: u32 = 18;

fn get_admin(env: &Env) -> Address {
    env.storage().persistent().get(&DataKey::Admin).unwrap()
}

fn check_source(env: &Env, addr: &Address) {
    let is_source: bool = env
        .storage()
        .persistent()
        .get(&DataKey::Source(addr.clone()))
        .unwrap_or(false);
    if !is_source {
        panic_with_error!(env, ErrorCode::NotAuthorized);
    }
}

fn check_registered_asset(env: &Env, asset: &Address) {
    let is_registered: bool = env
        .storage()
        .persistent()
        .get(&DataKey::AssetRegistered(asset.clone()))
        .unwrap_or(false);
    if !is_registered {
        panic_with_error!(env, ErrorCode::AssetNotRegistered);
    }
}

fn sort_prices(prices: &mut Vec<i128>) {
    let n = prices.len();
    if n <= 1 {
        return;
    }
    for i in 0..n {
        for j in (i + 1)..n {
            if prices.get_unchecked(i) > prices.get_unchecked(j) {
                let tmp = prices.get_unchecked(i);
                prices.set(i, prices.get_unchecked(j));
                prices.set(j, tmp);
            }
        }
    }
}

fn compute_median(prices: &Vec<i128>) -> i128 {
    let n = prices.len();
    if n == 0 {
        return 0;
    }
    let mut sorted = prices.clone();
    sort_prices(&mut sorted);
    if n % 2 == 0 {
        let mid = n / 2;
        let a = sorted.get_unchecked(mid - 1);
        let b = sorted.get_unchecked(mid);
        (a + b) / 2
    } else {
        sorted.get_unchecked(n / 2)
    }
}

#[contract]
pub struct PriceOracleContract;

#[contractimpl]
impl PriceOracleContract {
    pub fn __constructor(_env: Env) {
    }

    pub fn initialize(
        env: Env,
        admin: Address,
        min_sources_required: u32,
        max_history_length: u32,
        decimals: u32,
        description: String,
    ) {
        if env.storage().persistent().has(&DataKey::Admin) {
            panic_with_error!(env, ErrorCode::AlreadyInitialized);
        }
        admin.require_auth();
        env.storage().persistent().set(&DataKey::Admin, &admin);
        env.storage().persistent().set(
            &DataKey::MinSourcesRequired,
            &if min_sources_required > 0 {
                min_sources_required
            } else {
                DEFAULT_MIN_SOURCES
            },
        );
        env.storage().persistent().set(
            &DataKey::MaxHistoryLength,
            &if max_history_length > 0 {
                max_history_length
            } else {
                DEFAULT_MAX_HISTORY
            },
        );
        env.storage().persistent().set(&DataKey::Decimals, &decimals);
        env.storage().persistent().set(&DataKey::Description, &description);
        env.storage().persistent().set(
            &DataKey::OracleSources,
            &OracleSources {
                sources: Vec::new(&env),
                metadata: Map::new(&env),
            },
        );
    }

    pub fn upgrade(env: Env, new_wasm_hash: soroban_sdk::BytesN<32>) {
        let admin = get_admin(&env);
        admin.require_auth();
        env.deployer().update_current_contract_wasm(new_wasm_hash);
    }

    pub fn set_admin(env: Env, new_admin: Address) {
        let admin = get_admin(&env);
        admin.require_auth();
        env.storage().persistent().set(&DataKey::Admin, &new_admin);
    }

    pub fn get_admin_address(env: Env) -> Address {
        get_admin(&env)
    }

    pub fn set_min_sources_required(env: Env, new_min: u32) {
        let admin = get_admin(&env);
        admin.require_auth();
        env.storage()
            .persistent()
            .set(&DataKey::MinSourcesRequired, &new_min);
    }

    pub fn get_min_sources_required(env: Env) -> u32 {
        env.storage()
            .persistent()
            .get(&DataKey::MinSourcesRequired)
            .unwrap_or(DEFAULT_MIN_SOURCES)
    }

    pub fn set_max_history_length(env: Env, new_max: u32) {
        let admin = get_admin(&env);
        admin.require_auth();
        env.storage()
            .persistent()
            .set(&DataKey::MaxHistoryLength, &new_max);
    }

    pub fn get_max_history_length(env: Env) -> u32 {
        env.storage()
            .persistent()
            .get(&DataKey::MaxHistoryLength)
            .unwrap_or(DEFAULT_MAX_HISTORY)
    }

    pub fn set_decimals(env: Env, new_decimals: u32) {
        let admin = get_admin(&env);
        admin.require_auth();
        env.storage().persistent().set(&DataKey::Decimals, &new_decimals);
    }

    pub fn get_decimals(env: Env) -> u32 {
        env.storage()
            .persistent()
            .get(&DataKey::Decimals)
            .unwrap_or(DEFAULT_DECIMALS)
    }

    pub fn set_description(env: Env, new_description: String) {
        let admin = get_admin(&env);
        admin.require_auth();
        env.storage()
            .persistent()
            .set(&DataKey::Description, &new_description);
    }

    pub fn get_description(env: Env) -> String {
        env.storage()
            .persistent()
            .get(&DataKey::Description)
            .unwrap_or(String::from_str(&env, "Stellar Price Oracle"))
    }

    pub fn register_asset(env: Env, asset: Address) {
        let admin = get_admin(&env);
        admin.require_auth();
        if env
            .storage()
            .persistent()
            .has(&DataKey::AssetRegistered(asset.clone()))
        {
            panic_with_error!(env, ErrorCode::AssetAlreadyRegistered);
        }
        env.storage()
            .persistent()
            .set(&DataKey::AssetRegistered(asset.clone()), &true);
        env.storage()
            .persistent()
            .set(&DataKey::Aggregate(asset.clone()), &AggregatePrice {
                price: 0,
                timestamp: 0,
                num_sources: 0,
                decimals: Self::get_decimals(env.clone()),
            });
    }

    pub fn unregister_asset(env: Env, asset: Address) {
        let admin = get_admin(&env);
        admin.require_auth();
        check_registered_asset(&env, &asset);
        env.storage()
            .persistent()
            .remove(&DataKey::AssetRegistered(asset.clone()));
        env.storage()
            .persistent()
            .remove(&DataKey::Aggregate(asset.clone()));
    }

    pub fn is_asset_registered(env: Env, asset: Address) -> bool {
        env.storage()
            .persistent()
            .get(&DataKey::AssetRegistered(asset))
            .unwrap_or(false)
    }

    pub fn add_source(env: Env, source: Address, name: String) {
        let admin = get_admin(&env);
        admin.require_auth();
        if env
            .storage()
            .persistent()
            .has(&DataKey::Source(source.clone()))
        {
            panic_with_error!(env, ErrorCode::SourceAlreadyExists);
        }
        env.storage()
            .persistent()
            .set(&DataKey::Source(source.clone()), &true);

        let mut oracle_sources: OracleSources = env
            .storage()
            .persistent()
            .get(&DataKey::OracleSources)
            .unwrap();
        oracle_sources.sources.push_back(source.clone());
        oracle_sources.metadata.set(source.clone(), name);
        env.storage()
            .persistent()
            .set(&DataKey::OracleSources, &oracle_sources);
    }

    pub fn remove_source(env: Env, source: Address) {
        let admin = get_admin(&env);
        admin.require_auth();
        if !env
            .storage()
            .persistent()
            .has(&DataKey::Source(source.clone()))
        {
            panic_with_error!(env, ErrorCode::SourceNotFound);
        }
        env.storage()
            .persistent()
            .remove(&DataKey::Source(source.clone()));

        let mut oracle_sources: OracleSources = env
            .storage()
            .persistent()
            .get(&DataKey::OracleSources)
            .unwrap();
        let mut new_sources: Vec<Address> = Vec::new(&env);
        for i in 0..oracle_sources.sources.len() {
            let s = oracle_sources.sources.get_unchecked(i);
            if s != source {
                new_sources.push_back(s);
            }
        }
        oracle_sources.sources = new_sources;
        oracle_sources.metadata.remove(source);
        env.storage()
            .persistent()
            .set(&DataKey::OracleSources, &oracle_sources);
    }

    pub fn is_source(env: Env, source: Address) -> bool {
        env.storage()
            .persistent()
            .get(&DataKey::Source(source))
            .unwrap_or(false)
    }

    pub fn get_oracle_sources(env: Env) -> OracleSources {
        env.storage()
            .persistent()
            .get(&DataKey::OracleSources)
            .unwrap()
    }

    pub fn submit_price(
        env: Env,
        source: Address,
        asset: Address,
        price: i128,
        timestamp: u64,
    ) {
        source.require_auth();
        check_source(&env, &source);
        check_registered_asset(&env, &asset);

        if price <= 0 {
            panic_with_error!(env, ErrorCode::InvalidPrice);
        }

        let decimals = Self::get_decimals(env.clone());

        let entry = PriceEntry {
            price,
            timestamp,
            source: source.clone(),
            decimals,
        };

        env.storage()
            .persistent()
            .set(&DataKey::Submission(asset.clone(), source.clone()), &entry);

        let min_required = Self::get_min_sources_required(env.clone());
        let oracle_sources: OracleSources = env
            .storage()
            .persistent()
            .get(&DataKey::OracleSources)
            .unwrap();
        let total_sources = oracle_sources.sources.len();

        let mut valid_prices: Vec<i128> = Vec::new(&env);
        let mut latest_timestamp: u64 = 0;
        let mut contributing_sources: u32 = 0;

        for i in 0..total_sources {
            let src = oracle_sources.sources.get_unchecked(i);
            let sub: Option<PriceEntry> = env
                .storage()
                .persistent()
                .get(&DataKey::Submission(asset.clone(), src));
            if let Some(entry_data) = sub {
                if entry_data.timestamp > latest_timestamp {
                    latest_timestamp = entry_data.timestamp;
                }
                valid_prices.push_back(entry_data.price);
                contributing_sources += 1;
            }
        }

        if contributing_sources >= min_required && valid_prices.len() > 0 {
            let median_price = compute_median(&valid_prices);

            let current_ledger = env.ledger().sequence();
            let prev_aggregate: AggregatePrice = env
                .storage()
                .persistent()
                .get(&DataKey::Aggregate(asset.clone()))
                .unwrap();

            let aggregate = AggregatePrice {
                price: median_price,
                timestamp: latest_timestamp,
                num_sources: contributing_sources,
                decimals,
            };
            env.storage()
                .persistent()
                .set(&DataKey::Aggregate(asset.clone()), &aggregate);

            if prev_aggregate.price != median_price || prev_aggregate.timestamp != latest_timestamp {
                let history_entry = PriceHistoryEntry {
                    price: median_price,
                    timestamp: latest_timestamp,
                    ledger: current_ledger,
                    num_sources: contributing_sources,
                };
                env.storage()
                    .temporary()
                    .set(&DataKey::PriceHistory(asset.clone(), current_ledger), &history_entry);
            }
        }
    }

    pub fn get_price(env: Env, asset: Address) -> AggregatePrice {
        check_registered_asset(&env, &asset);
        env.storage()
            .persistent()
            .get(&DataKey::Aggregate(asset))
            .unwrap()
    }

    pub fn get_source_price(
        env: Env,
        asset: Address,
        source: Address,
    ) -> PriceEntry {
        check_registered_asset(&env, &asset);
        check_source(&env, &source);
        env.storage()
            .persistent()
            .get(&DataKey::Submission(asset, source))
            .unwrap()
    }

    pub fn get_all_prices(env: Env, asset: Address) -> Vec<PriceEntry> {
        check_registered_asset(&env, &asset);
        let oracle_sources: OracleSources = env
            .storage()
            .persistent()
            .get(&DataKey::OracleSources)
            .unwrap();
        let mut prices: Vec<PriceEntry> = Vec::new(&env);
        for i in 0..oracle_sources.sources.len() {
            let src = oracle_sources.sources.get_unchecked(i);
            let sub: Option<PriceEntry> = env
                .storage()
                .persistent()
                .get(&DataKey::Submission(asset.clone(), src));
            if let Some(entry) = sub {
                prices.push_back(entry);
            }
        }
        prices
    }

    pub fn get_historical_price(
        env: Env,
        asset: Address,
        ledger: u32,
    ) -> PriceHistoryEntry {
        check_registered_asset(&env, &asset);
        let key = DataKey::PriceHistory(asset, ledger);
        env.storage()
            .temporary()
            .get(&key)
            .unwrap()
    }

    pub fn has_historical_price(
        env: Env,
        asset: Address,
        ledger: u32,
    ) -> bool {
        if !env
            .storage()
            .persistent()
            .has(&DataKey::AssetRegistered(asset.clone()))
        {
            return false;
        }
        let key = DataKey::PriceHistory(asset, ledger);
        env.storage().temporary().has(&key)
    }

    pub fn get_historical_prices(
        env: Env,
        asset: Address,
        start_ledger: u32,
        end_ledger: u32,
    ) -> Vec<PriceHistoryEntry> {
        check_registered_asset(&env, &asset);
        let mut entries: Vec<PriceHistoryEntry> = Vec::new(&env);
        let mut ledger = start_ledger;
        while ledger <= end_ledger {
            let key = DataKey::PriceHistory(asset.clone(), ledger);
            if env.storage().temporary().has(&key) {
                let entry: PriceHistoryEntry = env.storage().temporary().get(&key).unwrap();
                entries.push_back(entry);
            }
            ledger += 1;
        }
        entries
    }

    pub fn get_latest_ledger(env: Env) -> u32 {
        env.ledger().sequence()
    }
}

mod test;
