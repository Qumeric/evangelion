use std::time::{Duration, Instant};
use dashmap::DashMap;
use reth_payload_builder::{
    BuiltPayload, PayloadBuilderAttributes,
};

use reth_primitives::{Block, H256, U256, BlockHash};
use reth_revm::primitives::B256;

use crate::relay_endpoint::RelayEndpoint;

#[derive(PartialEq, PartialOrd, Eq, Ord)]
struct Bid {
    value: U256,
    is_ours: bool,
}

enum Status {
    NeverSent,
    Sealed,
    Winning,
    Losing,
    Error(String),
}

struct PayloadWithMetadata {
    // Fields...
}

impl PayloadWithMetadata {
    fn set_status(&mut self, status: Status) {
        match status {
            Status::Sealed => {
                // this.add_payback_txs_to_mempool();
            }
            _ => {}
        }
        // this.update_metrics_and_logs(status);
    }
}

type Hash = B256;
type Time = Instant;

// TODO shall be something like H384
type PubkeyHex = String;

struct Executor {
    endpoint: RelayEndpoint,
    public_key: PubkeyHex,
    secret_key: H256,
    extra_data: String, // TODO wtf is it?
    signing_domain: H256,
    
    last_block_hash: BlockHash, // ??
    blocks: DashMap<BlockHash, BlockWithMetadata>,
    slot_start_time: Time,
    max_bid: Bid,
    best_block: Option<BlockWithMetadata>,
    attributes: PayloadBuilderAttributes,
}

struct BlockWithMetadata {
    inner: Block,
    value: U256,
}

impl Executor {
    async fn on_new_block(&mut self, block: BlockWithMetadata) {
        let hash = block.inner.header.hash_slow();
        if self.blocks.contains_key(&hash) {
            return;
        }

        // self.blocks[hash] = block;
        // self.submit(Some(&block));
    }

    async fn on_new_bid(&mut self, bid: Bid) {
        if bid <= self.max_bid {
            return;
        }
        self.max_bid = bid;

        // self.submit(None);
    }

    fn send_to_relay_and_wait_for_answer() -> Status {
        unimplemented!();
    }

    fn submit(&self, block: Option<&BlockWithMetadata>) {
        if Instant::now() < self.slot_start_time + Duration::from_secs(10) {
            return;
        }
        if Instant::now() > self.slot_start_time + Duration::from_secs(12) {
            return;
        }

        // let b = match block {
        //     None => &self.best_block,
        //     Some(value) => Some(value),
        // };
        let b: Option<BlockWithMetadata> = None;

        if b.is_none() {
            // Nothing to send
            return;
        }

        let bid = self.calculate_bid(b.as_ref().unwrap());

        match bid {
            Ok(bid_value) => {
                let sealed_block = b.unwrap().inner.seal_slow();
                let payload= BuiltPayload::new(self.attributes.id, sealed_block, bid_value);

            }
            Err(_) => {
                return
            }
        }

    }


    fn calculate_bid(&self, block: &BlockWithMetadata) -> Result<U256, String> {
        if self.max_bid.is_ours {
            return Ok(self.max_bid.value + U256::from(1));
        }

        // TODO: currently it's the simplest possible algorithm
        return if block.value > self.max_bid.value {
            Ok((block.value + self.max_bid.value) / U256::from(2))
        } else {
            Err("Block value is lower than max bid".to_string())
        }
    }

    // bid = this.calculateBid(payload.value)
    // if bid <= maxBid {
    // // assuming no cancellations
    // return
    // }
    // this.maxBid = bid
    // relayResponse = this.sendToRelayAndWaitForAnswer()
    // payload.setStatus(relayResponse)
    // }

}

#[tokio::main]
async fn main() {
    // Initialization and function calls
}
