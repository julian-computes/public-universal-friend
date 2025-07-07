use anyhow::Result;
use p2panda_core::PrivateKey;
use p2panda_discovery::mdns::LocalDiscovery;
use p2panda_net::{Network, NetworkBuilder};

use crate::p2p::ChatGroup;

/// Initialize a basic p2panda network with mDNS discovery.
pub async fn create_network() -> Result<Network<ChatGroup>> {
    // Peers using the same "network id" will eventually find each other. This
    // is the most global identifier to group peers into multiple networks when
    // necessary.
    let network_id = [1; 32];

    // Generate an Ed25519 private key which will be used to authenticate your peer towards others.
    let private_key = PrivateKey::new();

    // Use mDNS to discover other peers on the local network.
    let mdns_discovery = LocalDiscovery::new();

    // Establish the p2p network which will automatically connect to any discovered peers.
    let network = NetworkBuilder::new(network_id.into())
        .private_key(private_key)
        .discovery(mdns_discovery)
        .build()
        .await?;

    Ok(network)
}
