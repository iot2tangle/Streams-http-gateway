use gateway_core::gateway::publisher::Channel;
use local::device_auth::keystore::KeyManager;
use local::types::{channel_state::ChannelState, config::Config};
use local::wifi_connectivity::http_server;

use std::fs::File;
use std::sync::{Arc, Mutex};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    //read configuration file
    let config: Config = serde_json::from_reader(File::open("config.json").unwrap()).unwrap();

    let store = KeyManager::new(config.whitelisted_device_ids.clone());

    println!("Starting....");

    let mut channel = Channel::new(config.node.clone(), config.mwm, config.local_pow, None);
    let (addr, msg_id) = match channel.open() {
        Ok(a) => a,
        Err(_) => panic!("Could not connect to IOTA Node, try with another node!"),
    };
    let channel_id = format!("{}:{}", addr, msg_id);

    let channel_state = Arc::new(Mutex::new(ChannelState {
        channel: channel,
        channel_id: channel_id,
    }));

    let store = Arc::new(Mutex::new(store));

    http_server::start(config, channel_state, store).await
}
