use serde_derive::Deserialize;
use serde_derive::Serialize;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub whitelisted_device_ids: Vec<String>,
    pub port: u16,
    pub node: String,
    pub local_pow: bool,
}
