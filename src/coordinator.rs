use crate::relay_endpoint::{RelayEndpoint, Validator};
use crate::signing::sign_builder_message;
use anyhow::Result;
use dashmap::DashMap;
use ethereum_consensus::primitives::{Hash32, BlsPublicKey};
use ethereum_consensus::ssz::ByteVector;
use ethers::abi::Hash;
use mev_rs::types::{capella, BidTrace, ExecutionPayload, SignedBidSubmission};
use reqwest::Client;
use reth_primitives::hex::decode;
use reth_primitives::{hex, sign_message, Address, Block, U256};
use reth_transaction_pool::PoolTransaction;
use serde::Deserialize;
use ssz_rs::DeserializeError;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::time::Duration;
use tokio::time::{sleep, timeout};

// TODO default signing domain (originally in boost-utils, possibly in mev-rs now?)

struct Endpoint {
    pub url: String,
    pub alias: String,
    pub authorization_header: Option<String>,
    pub disable_gzip: bool,
    pub secret_key: String, // TODO isn't it used everywhere
    pub blacklist_file: Option<String>,
    pub extra_data: String, // TODO what is it?
}

struct Config {
    pub endpoints: Vec<Endpoint>,
    pub endpoints_validator_data: Vec<String>, // TODO: maybe bad name
}

// TODO: rename
struct Coordinator {
    // TODO singing_domain
    all_endpoints: Vec<RelayEndpoint>, // TODO shall be immutable...
    // TODO had syncer
    last_slot: u64,
    ready_relays: Vec<ReadyRelay>,
    // TODO beacon_client (not real client, redis connection),
    builder_public_key: B160,
}

struct ReadyRelay {
    pub endpoint: &'static RelayEndpoint,
    pub validator: &'static Validator,
}

// TODO:
// metric -- what slots we found
// metric -- how many we send per slot per validator

impl Coordinator {
    fn get_ready_relays(&self, slot: u64) -> Vec<ReadyRelay> {
        self.all_endpoints
            .iter()
            .filter_map(|endpoint| match endpoint.get_validators() {
                Ok(validators) => {
                    let validator = validators.iter().find(|&validator| validator.slot == slot);
                    if validator.is_some() {
                        Some(ReadyRelay {
                            endpoint,
                            validator: validator.unwrap(),
                        })
                    } else {
                        None
                    }
                }
                Err(err) => {
                    println!("GetValidatorForSlot: {}", err);
                    None
                }
            })
            .collect()
    }

    fn on_payload_attributes(&mut self, pa: PayloadAttributes) {
        // TODO: is it reorg? also, cases < and == were treated separately before
        if pa.slot <= self.last_slot {
            // TODO: log error? we shall only get new ones, right? Or maybe it's reorg?
            return;
        }

        self.last_slot = pa.slot;

        // TODO: previously wrote slot metric (???)
        // TODO: log new payload attributes

        self.ready_relays = self.get_ready_relays(pa.slot);
    }

    fn on_new_block(&self, block: Block, value: U256) {
        for relay in self.ready_relays {
            // TODO may be slow to rebuild it for each relay. Execution payload is always the same
            let bid: SignedBidSubmission = self.create_bid(relay, block, value).unwrap();
            relay.endpoint.post_block(&bid);
        }
    }

    // TODO check if all fields are correct
    fn create_bid(
        &self,
        relay: ReadyRelay,
        block: Block,
        value: U256,
    ) -> Result<SignedBidSubmission> {
        let block_hash = block.hash_slow();

        // parent_hash_bytevector = block.parent_hash.as_bytes()

        let parent_hash = Hash32::try_from(block.parent_hash.as_bytes());


        let mut message = BidTrace {
            slot: self.last_slot,
            parent_hash: parent_hash,
            block_hash: Hash32::try_from(block_hash.as_bytes())?,
            builder_public_key: ByteVector::<20>::try_from(self.builder_public_key.as_bytes())?,
            proposer_public_key: relay.validator.entry.message.pubkey.parse().unwrap(), // TODO convert
            proposer_fee_recipient: ByteVector::<20>::try_from(relay.validator.entry.message.fee_recipient.as_bytes())?,
            gas_limit: block.gas_limit,
            gas_used: block.gas_used,
            value: ssz_rs::U256::from_bytes_le(value.to_le_bytes()),
        }

        let execution_payload = capella::ExecutionPayload {
            parent_hash: parent_hash,
            fee_recipient: ByteVector::<20>::try_from(block.beneficiary.as_bytes())?,
            state_root: block.state_root,
            receipts_root: block.receipts_root,
            logs_bloom: ByteVector::<256>::try_from(block.logs_bloom.as_bytes())?,
            prev_randao: block.mix_hash,
            block_number: block.number,
            gas_limit: block.gas_limit,
            gas_used: block.gas_used,
            timestamp: block.timestamp,
            extra_data: block.extra_data,
            base_fee_per_gas: block.base_fee_per_gas.unwrap().into(),
            block_hash: block_hash,
            transactions: block.body,
            withdrawals: block.withdrawals.unwrap(),
            // data_gas_used: block.blob_gas_used,
            // excess_data_gas: block.excess_blob_gas,
        };

        let signature = sign_builder_message(message, secret);
        // pub struct PayloadAttributes {
        //   pub timestamp: u64,
        //   pub random: H256,
        //   pub suggested_fee_receiptient: Address,
        //   pub withdrawals: Vec<Withdrawal>,
        //   pub slot: u64,
        //   pub head_hash: BlockHash,
        //   pub gas_limit: u64,
        // }
        unimplemented!();
    }
}

fn new_coordinator(config: Config) {
    let get_validators = config.endpoints_validator_data.iter().map(|&endpoint| {
        ValidatorGetter {
            endpoint: endpoint.clone(),
            alias: endpoint, // TODO to hostname with port
            // is_validator_sync_ongoing: false,
            last_requested_slot: 0,
            slot_to_validator: HashMap::new(),
        };
    });

    let relay_submitters: Result<Vec<RelaySubmitter>, anyhow::Error> = config
        .endpoints
        .iter()
        .map(|&endpoint| {
            let mut blacklist = HashSet::new();

            match endpoint.blacklist_file {
                Some(file) => {
                    let file = fs::read_to_string(&file).map_err(|e| {
                        anyhow::anyhow!("Failed to read the blacklist file: {:?}", e)
                    })?;
                    let _blacklist: Vec<[u8; 20]> = serde_json::from_str(&file).map_err(|e| {
                        anyhow::anyhow!("Failed to parse the blacklist JSON: {:?}", e)
                    })?;
                    for addr in _blacklist.iter() {
                        blacklist.insert(Address::from_slice(addr));
                    }
                }
                None => {}
            }
            // Verify that the alias is to an existing endpoint
            if !endpoint.alias.is_empty() {
                let matched = config.endpoints.iter().any(|&e| e.url == endpoint.alias);
                if !matched {
                    return Err(anyhow::anyhow!(
                        "The endpoint alias: {} doesn't match any existing endpoint URL",
                        endpoint.alias
                    ));
                }
            }

            // TODO this prob needed from bid trace and stuff
            // config: RelaySubmitterConfig {
            //     secret_key: endpoint.secret_key.clone(),
            //     signing_domain: builder_signing_domain.clone(),
            // },

            return anyhow::Ok(RelaySubmitter {
                endpoint: endpoint.url,
                alias: endpoint.alias,
                authorization_header: endpoint.authorization_header,
                disable_gzip: endpoint.disable_gzip,
                client: Client::new(),
                blacklist: blacklist,
            });
        })
        .collect();
}
