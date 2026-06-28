#![cfg(test)]

use soroban_sdk::{
    contract, contractimpl, testutils::Address as _, Address, Env, Map,
};

use crate::{PriceOracleContract, PriceOracleContractClient};

// ---------------------------------------------------------------------------
// Mock external reference oracle
// ---------------------------------------------------------------------------

#[contract]
pub struct MockReferenceOracle;

#[contractimpl]
impl MockReferenceOracle {
    pub fn set_price(env: Env, asset: Address, price: i128) {
        env.storage().temporary().set(&asset, &price);
    }

    pub fn lastprice(env: Env, asset: Address) -> i128 {
        env.storage().temporary().get(&asset).unwrap_or(0)
    }
}

fn deploy_mock_oracle(e: &Env) -> (Address, MockReferenceOracleClient<'_>) {
    let contract_id = e.register(MockReferenceOracle, ());
    let client = MockReferenceOracleClient::new(e, &contract_id);
    (contract_id, client)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn setup_oracle(e: &Env) -> (Address, PriceOracleContractClient<'_>) {
    let admin = Address::generate(e);
    let contract_id = e.register(PriceOracleContract, ());
    let client = PriceOracleContractClient::new(e, &contract_id);
    e.mock_all_auths();
    client.initialize(
        &admin,
        &1,
        &10,
        &7,
        &soroban_sdk::String::from_str(e, "Test Oracle"),
        &None,
        &None,
        &None,
        &None,
    );
    (admin, client)
}

fn store_aggregate(e: &Env, contract_id: &Address, asset: &Address, price: i128) {
    use crate::types::{AggregatePrice, DataKey};
    let aggregate = AggregatePrice {
        price,
        timestamp: 1_000_000,
        num_sources: 1,
        decimals: 7,
        is_override: false,
    };
    e.as_contract(contract_id, || {
        e.storage().persistent().set(&DataKey::Aggregate(asset.clone()), &aggregate);
    });
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn test_add_reference_oracle_stores_entry() {
    let e = Env::default();
    let (_, client) = setup_oracle(&e);
    let (mock_id, _) = deploy_mock_oracle(&e);

    let asset_a = Address::generate(&e);
    let asset_b = Address::generate(&e);
    let mut mapping = Map::new(&e);
    mapping.set(asset_a, asset_b);

    client.add_reference_oracle(&mock_id, &mapping);

    let oracles = client.get_reference_oracles();
    assert_eq!(oracles.len(), 1);
    assert_eq!(oracles.get_unchecked(0), mock_id);
}

#[test]
#[should_panic]
fn test_add_reference_oracle_unauthorized() {
    let e = Env::default();
    let (_, client) = setup_oracle(&e);
    let (mock_id, _) = deploy_mock_oracle(&e);

    e.set_auths(&[]);

    let asset_a = Address::generate(&e);
    let asset_b = Address::generate(&e);
    let mut mapping = Map::new(&e);
    mapping.set(asset_a, asset_b);

    client.add_reference_oracle(&mock_id, &mapping);
}

#[test]
fn test_remove_reference_oracle() {
    let e = Env::default();
    let (_, client) = setup_oracle(&e);
    let (mock_id, _) = deploy_mock_oracle(&e);

    let asset_a = Address::generate(&e);
    let asset_b = Address::generate(&e);
    let mut mapping = Map::new(&e);
    mapping.set(asset_a, asset_b);

    client.add_reference_oracle(&mock_id, &mapping);
    assert_eq!(client.get_reference_oracles().len(), 1);

    client.remove_reference_oracle(&mock_id);
    assert_eq!(client.get_reference_oracles().len(), 0);
}

#[test]
fn test_cross_ref_deviation_threshold_default() {
    let e = Env::default();
    let (_, client) = setup_oracle(&e);
    assert_eq!(client.get_cross_ref_deviation_threshold(), 500);
}

#[test]
fn test_set_cross_ref_deviation_threshold() {
    let e = Env::default();
    let (_, client) = setup_oracle(&e);
    client.set_cross_ref_deviation_threshold(&200);
    assert_eq!(client.get_cross_ref_deviation_threshold(), 200);
}

#[test]
#[should_panic]
fn test_set_cross_ref_deviation_threshold_unauthorized() {
    let e = Env::default();
    let (_, client) = setup_oracle(&e);
    e.set_auths(&[]);
    client.set_cross_ref_deviation_threshold(&200);
}

#[test]
#[should_panic]
fn test_set_cross_ref_deviation_threshold_too_high() {
    let e = Env::default();
    let (_, client) = setup_oracle(&e);
    client.set_cross_ref_deviation_threshold(&100_001);
}

#[test]
fn test_get_cross_reference_no_oracle_returns_none() {
    let e = Env::default();
    let (_, client) = setup_oracle(&e);
    let asset = Address::generate(&e);
    assert!(client.get_cross_reference(&asset).is_none());
}

#[test]
fn test_get_cross_reference_no_local_price_returns_none() {
    let e = Env::default();
    let (_, client) = setup_oracle(&e);
    let (mock_id, mock_client) = deploy_mock_oracle(&e);

    let our_asset = Address::generate(&e);
    let ref_asset = Address::generate(&e);
    let mut mapping = Map::new(&e);
    mapping.set(our_asset.clone(), ref_asset.clone());
    client.add_reference_oracle(&mock_id, &mapping);
    mock_client.set_price(&ref_asset, &1_000_000);

    // No local aggregate stored — should return None.
    assert!(client.get_cross_reference(&our_asset).is_none());
}

#[test]
fn test_get_cross_reference_returns_prices_no_deviation() {
    let e = Env::default();
    let (_, client) = setup_oracle(&e);
    let (mock_id, mock_client) = deploy_mock_oracle(&e);

    let our_asset = Address::generate(&e);
    let ref_asset = Address::generate(&e);
    let mut mapping = Map::new(&e);
    mapping.set(our_asset.clone(), ref_asset.clone());
    client.add_reference_oracle(&mock_id, &mapping);

    let price = 1_000_000_i128;
    mock_client.set_price(&ref_asset, &price);
    store_aggregate(&e, &client.address, &our_asset, price);

    let result = client.get_cross_reference(&our_asset).unwrap();
    assert_eq!(result.our_price, price);
    assert_eq!(result.ref_price, price);
    assert_eq!(result.deviation_bps, 0);
    assert_eq!(result.ref_contract, mock_id);
}

#[test]
fn test_get_cross_reference_detects_deviation_below_threshold() {
    let e = Env::default();
    let (_, client) = setup_oracle(&e);
    let (mock_id, mock_client) = deploy_mock_oracle(&e);

    let our_asset = Address::generate(&e);
    let ref_asset = Address::generate(&e);
    let mut mapping = Map::new(&e);
    mapping.set(our_asset.clone(), ref_asset.clone());
    client.add_reference_oracle(&mock_id, &mapping);

    // 1% deviation, threshold is 5% — should not emit an event.
    let ref_price = 1_000_000_i128;
    let our_price = 1_010_000_i128; // +1%
    mock_client.set_price(&ref_asset, &ref_price);
    store_aggregate(&e, &client.address, &our_asset, our_price);

    let result = client.get_cross_reference(&our_asset).unwrap();
    assert_eq!(result.deviation_bps, 100); // 1% = 100 bps
    // No event should be emitted (deviation < threshold).
    assert_eq!(e.events().all().len(), 0);
}

#[test]
fn test_get_cross_reference_emits_event_on_deviation() {
    let e = Env::default();
    let (_, client) = setup_oracle(&e);
    let (mock_id, mock_client) = deploy_mock_oracle(&e);

    let our_asset = Address::generate(&e);
    let ref_asset = Address::generate(&e);
    let mut mapping = Map::new(&e);
    mapping.set(our_asset.clone(), ref_asset.clone());
    client.add_reference_oracle(&mock_id, &mapping);

    // 10% deviation, threshold is 5% — should emit CrossRefDeviationEvent.
    let ref_price = 1_000_000_i128;
    let our_price = 1_100_000_i128; // +10%
    mock_client.set_price(&ref_asset, &ref_price);
    store_aggregate(&e, &client.address, &our_asset, our_price);

    let result = client.get_cross_reference(&our_asset).unwrap();
    assert_eq!(result.deviation_bps, 1_000); // 10% = 1000 bps

    let events = e.events().all();
    assert!(!events.is_empty(), "expected CrossRefDeviationEvent to be emitted");
}

#[test]
fn test_get_cross_reference_no_mapping_for_asset_returns_none() {
    let e = Env::default();
    let (_, client) = setup_oracle(&e);
    let (mock_id, _) = deploy_mock_oracle(&e);

    let registered_asset = Address::generate(&e);
    let unregistered_asset = Address::generate(&e);
    let ref_asset = Address::generate(&e);
    let mut mapping = Map::new(&e);
    mapping.set(registered_asset, ref_asset);
    client.add_reference_oracle(&mock_id, &mapping);

    // No mapping for `unregistered_asset`.
    assert!(client.get_cross_reference(&unregistered_asset).is_none());
}

#[test]
fn test_multiple_reference_oracles_uses_first_match() {
    let e = Env::default();
    let (_, client) = setup_oracle(&e);
    let (mock_id_1, mock_client_1) = deploy_mock_oracle(&e);
    let (mock_id_2, mock_client_2) = deploy_mock_oracle(&e);

    let our_asset = Address::generate(&e);
    let ref_asset_1 = Address::generate(&e);
    let ref_asset_2 = Address::generate(&e);

    let mut mapping_1 = Map::new(&e);
    mapping_1.set(our_asset.clone(), ref_asset_1.clone());
    client.add_reference_oracle(&mock_id_1, &mapping_1);

    let mut mapping_2 = Map::new(&e);
    mapping_2.set(our_asset.clone(), ref_asset_2.clone());
    client.add_reference_oracle(&mock_id_2, &mapping_2);

    mock_client_1.set_price(&ref_asset_1, &1_000_000);
    mock_client_2.set_price(&ref_asset_2, &2_000_000);
    store_aggregate(&e, &client.address, &our_asset, 1_000_000);

    let result = client.get_cross_reference(&our_asset).unwrap();
    // First matching oracle wins.
    assert_eq!(result.ref_price, 1_000_000);
    assert_eq!(result.ref_contract, mock_id_1);
}
