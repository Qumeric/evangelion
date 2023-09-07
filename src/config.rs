use std::collections::HashMap;

use crate::relay_endpoint::RelayEndpoint;

struct Config {
    pub main_secret_key: &'static str,
}

pub fn get_relay_endpoints() -> Vec<RelayEndpoint> {
    // TODO somebody doesn't support gzip
    return vec![
        RelayEndpoint::new("ultrasound", "https://relay.ultrasound.money", true, None),
        RelayEndpoint::new(
            "bloxroute.max-profit",
            "https://bloxroute.max-profit.blxrbdn.com",
            true,
            None,
        ),
        RelayEndpoint::new(
            "bloxroute.regulated",
            "https://bloxroute.regulated.blxrbdn.com",
            true,
            None,
        ),
        RelayEndpoint::new("flashbots", "https://boost-relay.flashbots.net", true, None),
        RelayEndpoint::new("gnosis", "https://agnostic-relay.net", true, None),
        RelayEndpoint::new(
            "blocknative",
            "https://builder-relay-mainnet.blocknative.com",
            true,
            None,
        ),
        RelayEndpoint::new("aestus", "https://aestus.live", true, None),
        RelayEndpoint::new("edennetwork", "https://relay.edennetwork.io", true, None),
        RelayEndpoint::new(
            "securerpc",
            "https://mainnet-relay.securerpc.com",
            true,
            None,
        ),
    ];
}
