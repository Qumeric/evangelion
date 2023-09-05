use anyhow::Result;
use ethers::types::H256;
use ssz_rs::SimpleSerialize;
use std::sync::Arc;

use ethereum_consensus::{
    builder::{compute_builder_domain, ValidatorRegistration},
    clock::get_current_unix_time_in_secs,
    crypto::SecretKey,
    primitives::{BlsPublicKey, BlsSignature, Root, Slot, U256},
    signing::sign_with_domain,
    state_transition::Context,
};
use mev_rs::ValidatorRegistry;

#[derive(Clone)]

pub struct Inner {
    secret_key: SecretKey,
    public_key: BlsPublicKey,
    genesis_validators_root: Root,
    // validator_registry: ValidatorRegistry,
    context: Arc<Context>,
}

pub fn sign_builder_message<T: SimpleSerialize>(
    message: &mut T,
    signing_key: &SecretKey,
) -> Result<BlsSignature> {
    // TODO can be goerli
    let context = Context::for_mainnet();
    let domain = compute_builder_domain(&context)?;
    let signature = sign_with_domain(message, signing_key, domain)?;
    Ok(signature)
}
