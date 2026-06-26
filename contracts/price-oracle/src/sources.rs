use soroban_sdk::{panic_with_error, Address, Env, String, Vec};

use crate::events::{SourceAddedEvent, SourceRemovedEvent, SourceSuspendedEvent, SourceUnsuspendedEvent};
use crate::storage::{get_admin, read_oracle_sources, LEDGER_BUMP, LEDGER_THRESHOLD};
use crate::types::{DataKey, ErrorCode, OracleSources};

pub fn add_source(env: &Env, source: Address, name: String) {
    let admin = get_admin(env);
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

    let mut oracle_sources: OracleSources = read_oracle_sources(env);
    oracle_sources.sources.push_back(source.clone());
    let source_name = name.clone();
    oracle_sources.metadata.set(source.clone(), name);
    env.storage()
        .persistent()
        .set(&DataKey::OracleSources, &oracle_sources);
    SourceAddedEvent {
        source: source.clone(),
        admin: admin.clone(),
        name: source_name,
    }
    .publish(env);
}

pub fn remove_source(env: &Env, source: Address) {
    let admin = get_admin(env);
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

    let mut oracle_sources: OracleSources = read_oracle_sources(env);
    let mut new_sources: Vec<Address> = Vec::new(env);
    for i in 0..oracle_sources.sources.len() {
        let s = oracle_sources.sources.get_unchecked(i);
        if s != source {
            new_sources.push_back(s);
        }
    }
    oracle_sources.sources = new_sources;
    let removed_source = source.clone();
    oracle_sources.metadata.remove(source);
    env.storage()
        .persistent()
        .set(&DataKey::OracleSources, &oracle_sources);
    SourceRemovedEvent {
        source: removed_source,
        admin: admin.clone(),
    }
    .publish(env);
}

pub fn is_source(env: &Env, source: Address) -> bool {
    let key = DataKey::Source(source.clone());
    let exists: bool = env.storage().persistent().get(&key).unwrap_or(false);
    if exists {
        env.storage()
            .persistent()
            .extend_ttl(&key, LEDGER_THRESHOLD, LEDGER_BUMP);
    }
    exists
}

pub fn get_oracle_sources(env: &Env) -> OracleSources {
    read_oracle_sources(env)
}

pub fn is_source_suspended(env: &Env, source: Address) -> bool {
    let key = DataKey::SourceSuspended(source.clone());
    let suspended: bool = env.storage().persistent().get(&key).unwrap_or(false);
    if suspended {
        env.storage()
            .persistent()
            .extend_ttl(&key, LEDGER_THRESHOLD, LEDGER_BUMP);
    }
    suspended
}

pub fn record_invalid_submission(env: &Env, source: Address) {
    let count_key = DataKey::SourceInvalidCount(source.clone());
    let current_count: u32 = env.storage().persistent().get(&count_key).unwrap_or(0);
    let new_count = current_count + 1;
    env.storage()
        .persistent()
        .set(&count_key, &new_count);
    
    let max_invalid = crate::admin::get_max_invalid_submissions(env);
    if new_count >= max_invalid {
        env.storage()
            .persistent()
            .set(&DataKey::SourceSuspended(source.clone()), &true);
        let admin = get_admin(env);
        SourceSuspendedEvent {
            source: source.clone(),
            admin,
            reason: new_count,
        }
        .publish(env);
    }
}

pub fn unsuspend_source(env: &Env, source: Address) {
    let admin = get_admin(env);
    admin.require_auth();
    if !env
        .storage()
        .persistent()
        .has(&DataKey::SourceSuspended(source.clone()))
    {
        panic_with_error!(env, ErrorCode::SourceNotFound);
    }
    env.storage()
        .persistent()
        .remove(&DataKey::SourceSuspended(source.clone()));
    env.storage()
        .persistent()
        .remove(&DataKey::SourceInvalidCount(source.clone()));
    SourceUnsuspendedEvent {
        source: source.clone(),
        admin: admin.clone(),
    }
    .publish(env);
}
