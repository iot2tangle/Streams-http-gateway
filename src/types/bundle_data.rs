use crate::types::sensor_data::SensorData;
use serde_derive::Deserialize;
use serde_derive::Serialize;

#[derive(Serialize, Deserialize, Debug)]
pub struct BundleData {
    pub bundle: Vec<SensorData>,
}
