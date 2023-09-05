use crate::relay::{RelaySubmitter};
use crate::relay_endpoint::{RelayEndpoint, Validator};
use crate::types::{PayloadAttributes, SignedBidSubmission, BidTrace, ExecutionPayload};
use crate::validator_getter::{ValidatorGetter, ValidatorData};
use anyhow::Result;
use dashmap::DashMap;
use ethers::types::Sign;
use reqwest::Client;
use reth_primitives::hex::decode;
use reth_primitives::{Address, hex, Block, U256, sign_message};
use reth_transaction_pool::PoolTransaction;
use serde::Deserialize;
use tokio::time::{sleep, timeout};
use std::collections::{HashSet, HashMap};
use std::fs;
use std::time::Duration;

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
    // TODO beacon_client (not real client, redis connection)
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
        self.all_endpoints.iter().filter_map(|endpoint| {
            match endpoint.get_validators() {
                Ok(validators) => {
                    let validator = validators.iter().find(|&validator| {
                        validator.slot == slot
                    });
                    if validator.is_some() {
                        Some(ReadyRelay {
                            endpoint,
                            validator: validator.unwrap(),
                        })
                    } else {
                        None
                    }
                },
                Err(err) => {
                    println!("GetValidatorForSlot: {}", err);
                    None
                }
            }
        }).collect()
    }

    fn on_payload_attributes(&mut self, pa: PayloadAttributes) {
        if pa.slot < self.last_slot {
            // TODO: log error? we shall only get new ones, right? Or maybe it's reorg?
            return;
        }
        // TODO: is it reorg?
        if pa.slot == self.last_slot {
            let validator = self.block_attributes.validator;
            self.block_attributes = BlockAttributes {
                validator,
                // TODO there were construct params here but I guess we don't need it?
                slot: pa.slot,
            };
            return;
        }

        self.last_slot = pa.slot;
        // TODO: previously wrote slot metric (???)

        // TODO: log new payload attributes

        self.ready_relays = self.get_ready_relays(pa.slot);
    }

    fn on_new_block(&self, block: Block) {
        let bid: SignedBidSubmission = self.create_bid(block);
        for relay in self.ready_relays {
            relay.endpoint.post_block(&bid);
        }
    }

    // TODO check if all fields are correct
    fn create_bid(&self, relay: ReadyRelay, block: Block, value: U256) -> SignedBidSubmission {
        let block_hash = block.hash_slow();

        let message = BidTrace {
            slot: self.last_slot,
            parent_hash: block.parent_hash,
            block_hash: block_hash,
            builder_public_key: todo!("get from config I guess"),
            proposer_public_key: relay.validator.entry.message.pubkey.parse().unwrap(), // TODO convert
            proposer_fee_recipient: relay.validator.entry.message.fee_recipient,
            gas_limit: block.gas_limit,
            gas_used: block.gas_used,
            value: value,
        };

        let execution_payload = ExecutionPayload {
            parent_hash: block.parent_hash,
            fee_recipient: block.beneficiary,
            state_root: block.state_root,
            receipts_root: block.receipts_root,
            logs_bloom: block.logs_bloom,
            prev_randao: block.mix_hash,
            block_number: block.number,
            gas_limit: block.gas_limit,
            gas_used: block.gas_used,
            timestamp: block.timestamp,
            extra_data: block.extra_data,
            base_fee_per_gas: block.base_fee_per_gas.unwrap(),
            block_hash: block_hash,
            transactions: block.body,
            withdrawals: block.withdrawals.unwrap(),
            // data_gas_used: block.blob_gas_used,
            // excess_data_gas: block.excess_blob_gas,
        };

        let signature = sign_message(secret, message)
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
                    let _blacklist: Vec<[u8; 20]> =
                        serde_json::from_str(&file).map_err(|e| {
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