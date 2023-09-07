use crate::types::as_string;
use crate::types::SignedBidSubmission;
use anyhow::{Context, Result};
use ethereum_consensus::primitives::BlsPublicKey;
use flate2::{write::GzEncoder, Compression};
use reqwest::header;
use reth_primitives::{hex, Address};
use ruint::aliases::B384;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    error::Error,
    io::Write,
    ops::Add,
    time::Duration,
};
use tokio::time::sleep;

type PublicKey = B384;

// TODO only deserialze?
#[derive(Deserialize, Serialize, Debug)]
pub struct EntryMessage {
    pub fee_recipient: Address,
    #[serde(with = "as_string")]
    pub gas_limit: u64,
    #[serde(with = "as_string")]
    pub timestamp: u64,
    pub pubkey: PublicKey,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Entry {
    pub message: EntryMessage,
    pub signature: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Validator {
    #[serde(with = "as_string")]
    pub slot: u64,
    #[serde(with = "as_string")]
    pub validator_index: u64,
    pub entry: Entry,
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct SendBlockStatus {
    pub code: u64,
    pub message: String,
}

pub struct RelayEndpoint {
    name: String,
    url: String,
    client: reqwest::blocking::Client,
    is_gzip_enabled: bool,
    autorization_header: Option<String>,
    //tags: Vec<String>, // TODO support blacklist etc.
}

impl RelayEndpoint {
    pub fn new(
        name: &str,
        url: &str,
        is_gzip_enabled: bool,
        autorization_header: Option<String>,
    ) -> Self {
        RelayEndpoint {
            name: name.to_string(),
            url: url.to_string(),
            client: reqwest::blocking::Client::new(),
            is_gzip_enabled,
            autorization_header,
        }
    }
    pub fn get_validators(&self) -> Result<Vec<Validator>> {
        let endpoint = format!("{}/relay/v1/builder/validators", self.url);
        let response: Vec<Validator> = self
            .client
            .get(endpoint)
            .send()?
            .json()
            .context("get validators request")?;
        Ok(response)
    }

    pub fn post_block(&self, block: &SignedBidSubmission) -> Result<SendBlockStatus> {
        let endpoint = format!("{}/relay/v1/builder/blocks", self.url);
        let (body, encoding) = self.encode(&block)?;

        let mut req_builder = self
            .client
            .post(endpoint)
            .header(header::CONTENT_TYPE, "application/json")
            .body(body);

        if encoding.is_some() {
            req_builder = req_builder.header(header::CONTENT_ENCODING, encoding.unwrap());
        }

        if let Some(auth) = &self.autorization_header {
            req_builder = req_builder.header(header::AUTHORIZATION, auth);
        }

        let response: SendBlockStatus = req_builder.send()?.json().context("send block request")?;

        Ok(response)
    }

    fn encode(&self, bid: &SignedBidSubmission) -> Result<(Vec<u8>, Option<&str>)> {
        let payload = serde_json::to_vec(bid).context("marshal block json")?;

        if self.is_gzip_enabled {
            let mut compressed_buffer = Vec::new();
            let mut gz = GzEncoder::new(&mut compressed_buffer, Compression::fast());
            gz.write_all(&payload).context("write payload bytes")?;
            gz.finish().context("gzip finish")?;
            Ok((compressed_buffer, Some("gzip")))
        } else {
            Ok((payload, None))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::mev_boost_relay_json::SEND_BLOCK_REQUEST_EXAMPLE_JSON;

    use super::*;

    fn setup_endpoint() -> RelayEndpoint {
        RelayEndpoint::new("ultrasound", "https://relay.ultrasound.money", false, None)
    }

    #[test]
    fn test_get_validator_relay_response() -> Result<(), Box<dyn Error>> {
        let endpoint = setup_endpoint();
        let response: Vec<Validator> = endpoint.get_validators().unwrap();

        // Now `response` is a Vec<GetValidatorRelayResponse>
        println!("{:#?}", response);

        Ok(())
    }

    #[test]
    fn test_send_invalid_block() -> Result<(), Box<dyn Error>> {
        let endpoint = setup_endpoint();

        let bid: SignedBidSubmission = serde_json::from_str(SEND_BLOCK_REQUEST_EXAMPLE_JSON)?;
        let response = endpoint.post_block(&bid);

        let expected_response = SendBlockStatus {
            code: 400,
            message: "submission for past slot".to_string(),
        };

        match response {
            Ok(response_body) => {
                println!("{:#?}", response_body);
                assert_eq!(response_body, expected_response);
            }
            Err(e) => panic!("{}", e),
        }

        Ok(())
    }
}
