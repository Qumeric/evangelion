use crate::relay_endpoint::{RelayEndpoint, Validator};
use crate::signing::sign_builder_message;
use crate::types::{ExecutionPayload, PayloadAttributes, SignedBidSubmission};
use anyhow::Result;
use ethereum_consensus::crypto::SecretKey;
use ethereum_consensus::primitives::{BlsPublicKey, ExecutionAddress, Hash32};
use mev_rs::types::BidTrace;
use reth_primitives::{sign_message, Block, U256};
use ruint::aliases::B384;

// TODO default signing domain (originally in boost-utils, possibly in mev-rs now?)

struct Config {
    pub endpoints: Vec<RelayEndpoint>,
    pub builder_public_key: B384,
}

// TODO: rename
struct Coordinator {
    // TODO singing_domain
    all_endpoints: Vec<RelayEndpoint>, // TODO shall be immutable...
    // TODO had syncer
    last_slot: u64,
    ready_relays: Vec<ReadyRelay>,
    // TODO beacon_client (not real client, redis connection),
    builder_public_key: B384,
    secret_key: SecretKey,
}

struct ReadyRelay {
    pub index: usize,
    pub validator: Validator,
}

// TODO:
// metric -- what slots we found
// metric -- how many we send per slot per validator

impl Coordinator {
    fn get_ready_relays(&self, slot: u64) -> Vec<ReadyRelay> {
        self.all_endpoints
            .iter()
            .enumerate()
            .filter_map(|(i, endpoint)| match endpoint.get_validators() {
                Ok(validators) => {
                    let validator = validators
                        .into_iter()
                        .find(|validator| validator.slot == slot);
                    if validator.is_some() {
                        Some(ReadyRelay {
                            index: i,
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
        self.ready_relays.iter().for_each(|relay| {
            // TODO may be slow to rebuild it for each relay. Execution payload is always the same
            let bid: SignedBidSubmission = self.create_bid(relay, &block, value).unwrap();
            self.all_endpoints[relay.index].post_block(&bid);
        })
    }

    // TODO check if all fields are correct
    fn create_bid(
        &self,
        relay: &ReadyRelay,
        block: &Block,
        value: U256,
    ) -> Result<SignedBidSubmission> {
        let block_hash = block.hash_slow();

        // parent_hash_bytevector = block.parent_hash.as_bytes()

        let parent_hash = Hash32::try_from(block.parent_hash.as_bytes());

        let pk_bytes: [u8; 48] = self.builder_public_key.to_le_bytes();
        let pk_slice = &pk_bytes[..];

        let propeser_pk_bytes: [u8; 48] = relay.validator.entry.message.pubkey.to_le_bytes();
        let proposer_pk_slice = &propeser_pk_bytes[..];

        let mut message = BidTrace {
            slot: self.last_slot,
            parent_hash: Hash32::try_from(block.parent_hash.as_bytes())?,
            block_hash: Hash32::try_from(block_hash.as_bytes())?,
            builder_public_key: BlsPublicKey::try_from(pk_slice)?,
            proposer_public_key: BlsPublicKey::try_from(proposer_pk_slice)?,
            proposer_fee_recipient: ExecutionAddress::try_from(
                relay.validator.entry.message.fee_recipient.as_bytes(),
            )?,
            gas_limit: block.gas_limit,
            gas_used: block.gas_used,
            value: ssz_rs::U256::from_bytes_le(value.to_le_bytes()),
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
            extra_data: block.extra_data.clone(), // TODO
            base_fee_per_gas: block.base_fee_per_gas.unwrap().into(),
            block_hash: block_hash,
            transactions: block.body.clone(),
            withdrawals: block.withdrawals.clone().unwrap(),
            // data_gas_used: block.blob_gas_used,
            // excess_data_gas: block.excess_blob_gas,
        };

        // TOD:w

        let signature = sign_builder_message(&mut message, &self.secret_key)?;

        return Ok(SignedBidSubmission {
            message: message,
            signature: signature,
            execution_payload: execution_payload,
        });
    }
}

#[cfg(test)]
mod tests {
    use reth_revm_primitives::new;

    use crate::config::get_relay_endpoints;

    use super::*;

    #[test]
    fn create_coordinator() {
        let builder_pk = B384::default();
        let builder_sk = SecretKey::default();

        let coordinator = Coordinator {
            all_endpoints: get_relay_endpoints(),
            last_slot: 0,
            ready_relays: vec![],
            builder_public_key: builder_pk,
            secret_key: builder_sk,
        };
    }

    // #[test]
    // fn test_get_validator_relay_response() -> Result<(), Box<dyn Error>> {
    //     let endpoint = setup_endpoint();
    //     let response: Vec<Validator> = endpoint.get_validators().unwrap();

    //     // Now `response` is a Vec<GetValidatorRelayResponse>
    //     println!("{:#?}", response);

    //     Ok(())
    // }

    // #[test]
    // fn test_send_invalid_block() -> Result<(), Box<dyn Error>> {
    //     let endpoint = setup_endpoint();

    //     let bid: SignedBidSubmission = serde_json::from_str(SEND_BLOCK_REQUEST_EXAMPLE_JSON)?;
    //     let response = endpoint.post_block(&bid);

    //     let expected_response = SendBlockStatus {
    //         code: 400,
    //         message: "submission for past slot".to_string(),
    //     };

    //     match response {
    //         Ok(response_body) => {
    //             println!("{:#?}", response_body);
    //             assert_eq!(response_body, expected_response);
    //         }
    //         Err(e) => panic!("{}", e),
    //     }

    //     Ok(())
    // }
}
