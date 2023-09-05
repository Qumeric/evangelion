use std::collections::HashMap;

struct Config {
    pub main_secret_key: &'static str,
}


pub fn get_relay_urls() -> HashMap<&'static str, &'static str> {
    vec![
      ("ultrasound", "https://relay.ultrasound.money"),
      ("bloxroute.max-profit", "https://bloxroute.max-profit.blxrbdn.com"),
      ("flashbots", "https://boost-relay.flashbots.net"),
      ("gnosis", "https://agnostic-relay.net"),
      ("bloxroute.regulated", "https://bloxroute.regulated.blxrbdn.com"),
      ("blocknative", "https://builder-relay-mainnet.blocknative.com"),
      ("aestus", "https://aestus.live"), // Not used in original code for some reason?
      ("edennetwork", "https://relay.edennetwork.io"),
      ("securerpc", "https://mainnet-relay.securerpc.com"),
    ].into_iter().collect()
}