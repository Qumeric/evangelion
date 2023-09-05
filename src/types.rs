use reth_primitives::{Withdrawal, Address, H256, BlockHash, U256, Transaction, Bytes, TransactionSigned, Bloom};
use reth_revm_primitives::primitives::ruint::aliases::{B256, B384};
use serde::{Deserialize, Serialize};
use anyhow::{Result, anyhow};

// From ethereum-consensus, converted to anyhow from thiserror
const HEX_ENCODING_PREFIX: &str = "0x";

pub fn try_bytes_from_hex_str(s: &str) -> Result<Vec<u8>> {
  let target = s
      .strip_prefix(HEX_ENCODING_PREFIX)
      .ok_or_else(|| anyhow!("missing prefix `{}` when deserializing hex data", HEX_ENCODING_PREFIX))?;
  let data = hex::decode(target)
    .map_err(|e| anyhow!("Failed to decode hex: {}", e))?;
  Ok(data)
}

// From alexstokes' ethereum-consensus
pub mod as_hex {
    use super::*;
    use serde::de::Deserialize;

    pub fn serialize<S, T: AsRef<[u8]>>(data: T, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let encoding = hex::encode(data.as_ref());
        let output = format!("{HEX_ENCODING_PREFIX}{encoding}");
        serializer.collect_str(&output)
    }

    pub fn deserialize<'de, D, T>(deserializer: D) -> Result<T, D::Error>
    where
        D: serde::Deserializer<'de>,
        T: TryFrom<Vec<u8>>,
    {
        let s = <String>::deserialize(deserializer)?;

        let data = try_bytes_from_hex_str(&s).map_err(serde::de::Error::custom)?;

        let inner = T::try_from(data)
            .map_err(|_| serde::de::Error::custom("type failed to parse bytes from hex data"))?;
        Ok(inner)
    }
}

pub mod as_string {
    use serde::de::Deserialize;
    use std::{fmt, str::FromStr};

    pub fn serialize<S, T: fmt::Display>(data: T, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let output = format!("{data}");
        serializer.collect_str(&output)
    }

    pub fn deserialize<'de, D, T: FromStr>(deserializer: D) -> Result<T, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s: String = <String>::deserialize(deserializer)?;
        let inner: T = s
            .parse()
            // TODO fix error situation
            // FromStr::Err has no bounds
            .map_err(|_| serde::de::Error::custom("failure to parse string data"))?;
        Ok(inner)
    }
}

pub struct PayloadAttributes {
  pub timestamp: u64,
  pub random: H256,
  pub suggested_fee_receiptient: Address,
  pub withdrawals: Vec<Withdrawal>,
  pub slot: u64,
  pub head_hash: BlockHash,
  pub gas_limit: u64,
}

const BLS_PUBLIC_KEY_BYTES_LEN: usize = 48;

// TODO: This is mostly from mev-rs
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ValidatorRegistration {
    pub fee_recipient: Address,
    #[serde(with = "as_string")]
    pub gas_limit: u64,
    #[serde(with = "as_string")]
    pub timestamp: u64,
    #[serde(rename = "pubkey")]
    pub public_key: BlsPublicKey,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SignedValidatorRegistration {
    pub message: ValidatorRegistration,
    pub signature: BlsSignature,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ProposerSchedule {
    #[serde(with = "as_string")]
    pub slot: u64,
    #[serde(with = "as_string")]
    pub validator_index: u64,
    pub entry: SignedValidatorRegistration,
}

type BlsPublicKey = B384;
type BlsSignature = B768;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct BidTrace {
    #[serde(with = "as_string")]
    pub slot: u64,
    pub parent_hash: BlockHash,
    pub block_hash: BlockHash,
    #[serde(rename = "builder_pubkey")]
    pub builder_public_key: BlsPublicKey,
    #[serde(rename = "proposer_pubkey")]
    pub proposer_public_key: BlsPublicKey,
    pub proposer_fee_recipient: Address,
    #[serde(with = "as_string")]
    pub gas_limit: u64,
    #[serde(with = "as_string")]
    pub gas_used: u64,
    pub value: U256,
}


// TODO: From ruint, remove when ruint PR merged
use ruint::{Uint, Bits};
macro_rules! alias {
    ($($uname:ident $bname:ident ($bits:expr, $limbs:expr);)*) => {$(
        #[doc = concat!("[`Uint`] for `", stringify!($bits),"` bits.")]
        pub type $uname = Uint<$bits, $limbs>;
        #[doc = concat!("[`Bits`] for `", stringify!($bits),"` bits.")]
        pub type $bname = Bits<$bits, $limbs>;
    )*};
}

alias! {
    U768 B768 (768, 12);
}

type Bytes32 = H256;

// From alexstokes' ethereum-consensus
pub const BYTES_PER_LOGS_BLOOM: usize = 256;
pub const MAX_EXTRA_DATA_BYTES: usize = 32;
pub const MAX_BYTES_PER_TRANSACTION: usize = 1_073_741_824;
pub const MAX_TRANSACTIONS_PER_PAYLOAD: usize = 1_048_576;
pub const MAX_WITHDRAWALS_PER_PAYLOAD: usize = 16;

// It is deneb
#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionPayload {
    pub parent_hash: BlockHash,
    pub fee_recipient: Address,
    pub state_root: Bytes32,
    pub receipts_root: Bytes32,
    pub logs_bloom: Bloom,
    pub prev_randao: Bytes32,
    #[serde(with = "as_string")]
    pub block_number: u64,
    #[serde(with = "as_string")]
    pub gas_limit: u64,
    #[serde(with = "as_string")]
    pub gas_used: u64,
    #[serde(with = "as_string")]
    pub timestamp: u64,
    pub extra_data: Bytes, // TODO: should never be more that MAX_EXTRA_DATA_BYTES
    #[serde(with = "as_string")]
    pub base_fee_per_gas: u64,
    pub block_hash: BlockHash,
    pub transactions: Vec<TransactionSigned>,
    pub withdrawals: Vec<Withdrawal>,
    // deben stuff
    // #[serde(with = "as_string")]
    // pub data_gas_used: u64,
    // #[serde(with = "as_string")]
    // pub excess_data_gas: u64,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct SignedBidSubmission {
    pub message: BidTrace,
    // TODO: support multiple forks
    pub execution_payload: ExecutionPayload,
    pub signature: BlsSignature,
}
