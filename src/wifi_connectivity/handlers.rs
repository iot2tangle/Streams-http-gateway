use crate::device_auth::keystore::{authenticate, calculate_hash, KeyManager};
use crate::timestamp_in_sec;
use crate::types::{
    bundle_data::BundleData, channel_state::ChannelState, config::Config, sensor_data::SensorData,
    switch_auth::SwitchAuth,
};

use std::sync::{Arc, Mutex};

use gateway_core::gateway::publisher::Channel;

type GenericError = Box<dyn std::error::Error + Send + Sync>;
type Result<T> = std::result::Result<T, GenericError>;

use hyper::{header, Body, Request, Response, StatusCode, Uri};

///
/// Handles the status request returning status code 200 if the server is online
///
pub async fn status_response() -> Result<Response<Body>> {
    Ok(Response::builder().status(200).body(Body::from("OK"))?)
}

///
/// Handles the reuqest from the sensor by parsing the provieded data into the SensorData Format.
/// It authenticates the device through the "device" attribute, and if successfull published the data to the Tangle
/// through the streams channel
///
pub async fn sensor_data_response(
    req: Request<Body>,
    channel_state: Arc<Mutex<ChannelState>>,
    keystore: Arc<Mutex<KeyManager>>,
) -> Result<Response<Body>> {
    let data = hyper::body::to_bytes(req.into_body()).await?;

    let response;

    let json_data: serde_json::Result<SensorData> = serde_json::from_slice(&data);
    match json_data {
        Ok(mut sensor_data) => {
            let hashes = keystore
                .lock()
                .expect("lock keystore")
                .keystore
                .api_keys_author
                .clone();
            if authenticate(&sensor_data.device, hashes.clone()) {
                sensor_data.device.to_string().push_str("_id");
                sensor_data.device = calculate_hash(sensor_data.device);
                sensor_data.timestamp = serde_json::Value::from(timestamp_in_sec());
                println!(
                    "POST /sensor_data -- {:?} -- authorized request by device",
                    timestamp_in_sec()
                );
                let mut channel_state = channel_state.lock().unwrap();
                match channel_state.channel.write_signed(&sensor_data) {
                    Ok(_) => {
                        response = Response::builder()
                            .status(StatusCode::OK)
                            .header(header::CONTENT_TYPE, "application/json")
                            .body(Body::from(channel_state.channel_id.clone()))?;
                    }
                    Err(_e) => {
                        println!("POST /sensor_data Error: Connection to IOTA Node Error");
                        response = Response::builder()
                            .status(StatusCode::REQUEST_TIMEOUT)
                            .header(header::CONTENT_TYPE, "application/json")
                            .body(Body::from(
                                "Could not connect to IOTA Node, try with another node!",
                            ))?;
                    }
                };
            } else {
                response = Response::builder()
                    .status(StatusCode::UNAUTHORIZED)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        "Unauthorized - Device Name sent by device doesn't match the configuration",
                    ))?;
                println!(
                    "POST /sensor_data -- {:?} -- unauthorized request blocked",
                    timestamp_in_sec()
                );
            }
        }
        Err(_e) => {
            response = Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from("Malformed json - use iot2tangle json format"))?;
        }
    }
    Ok(response)
}

pub async fn send_bundle_response(
    req: Request<Body>,
    channel_state: Arc<Mutex<ChannelState>>,
    keystore: Arc<Mutex<KeyManager>>,
) -> Result<Response<Body>> {
    let data = hyper::body::to_bytes(req.into_body()).await?;

    let response;

    let json_data: serde_json::Result<BundleData> = serde_json::from_slice(&data);
    match json_data {
        Ok(mut bundle_data) => {
            let hashes = keystore
                .lock()
                .expect("lock keystore")
                .keystore
                .api_keys_author
                .clone();

            println!(
                "POST /bundle_data -- {:?} -- authorized request by device",
                timestamp_in_sec()
            );
            let mut status: Vec<&str> = vec![];
            for mut sensor_data in &mut bundle_data.bundle {
                if authenticate(&sensor_data.device, hashes.clone()) {
                    sensor_data.device.to_string().push_str("_id");
                    sensor_data.device = calculate_hash(sensor_data.device.clone());
                    //sensor_data.timestamp = serde_json::Value::from(timestamp_in_sec());
                    status.push("OK");
                } else {
                    status.push("UNAUTHORIZED");
                    println!(
                        "POST /bundle_data -- {:?} -- unauthorized request blocked",
                        timestamp_in_sec()
                    );
                }
            }

            if !status.contains(&"UNAUTHORIZED") {
                let mut channel_state = channel_state.lock().unwrap();
                match channel_state.channel.write_signed(&bundle_data) {
                    Ok(_) => {
                        response = Response::builder()
                            .status(StatusCode::OK)
                            .header(header::CONTENT_TYPE, "application/json")
                            .body(Body::from(channel_state.channel_id.clone()))?;
                    }
                    Err(_e) => {
                        response = Response::builder()
                            .status(StatusCode::REQUEST_TIMEOUT)
                            .header(header::CONTENT_TYPE, "application/json")
                            .body(Body::from(
                                "Could not connect to IOTA Node, try with another node!",
                            ))?;
                    }
                };
            } else {
                response = Response::builder()
                    .status(StatusCode::UNAUTHORIZED)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        "Unauthorized - At least 1 Device Name sent is not whitelisted",
                    ))?;
            }
        }
        Err(_e) => {
            response = Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from("Malformed json - use iot2tangle json format"))?;
        }
    }
    Ok(response)
}

pub async fn switch_channel_response(
    req: Request<Body>,
    channel_state: Arc<Mutex<ChannelState>>,
    keystore: Arc<Mutex<KeyManager>>,
    config: Config,
) -> Result<Response<Body>> {
    let data = hyper::body::to_bytes(req.into_body()).await?;

    let response;

    let json_data: serde_json::Result<SwitchAuth> = serde_json::from_slice(&data);
    match json_data {
        Ok(device_auth) => {
            let hashes = keystore
                .lock()
                .expect("lock keystore")
                .keystore
                .api_keys_author
                .clone();

            if authenticate(&device_auth.device, hashes.clone()) {
                println!(
                    "POST /switch_channel -- {:?} -- authorized request by device",
                    timestamp_in_sec()
                );

                let mut channel = Channel::new(config.node, config.local_pow, None);
                let (addr, msg_id) = match channel.open() {
                    Ok(a) => a,
                    Err(_) => {
                        return Ok(Response::builder()
                            .status(StatusCode::REQUEST_TIMEOUT)
                            .header(header::CONTENT_TYPE, "application/json")
                            .body(Body::from(
                                "Could not connect to IOTA Node, try with another node!",
                            ))?)
                    }
                };
                let channel_id = format!("{}:{}", addr, msg_id);

                let mut channel_state = channel_state.lock().expect("");
                channel_state.channel = channel;
                channel_state.channel_id = channel_id.clone();

                response = Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(channel_id))?;
            } else {
                response = Response::builder()
                    .status(StatusCode::UNAUTHORIZED)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        "Unauthorized - Device Name sent by device doesn't match the configuration",
                    ))?;
                println!(
                    "POST /switch_channel -- {:?} -- unauthorized request blocked",
                    timestamp_in_sec()
                );
            }
        }
        Err(_e) => {
            response = Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from("Malformed json - use iot2tangle json format"))?;
        }
    }
    Ok(response)
}

pub async fn get_current_channel(
    req: Request<Body>,
    channel_state: Arc<Mutex<ChannelState>>,
    keystore: Arc<Mutex<KeyManager>>,
) -> Result<Response<Body>> {
    let req_uri = &req.uri().to_string().parse::<Uri>().unwrap();
    let device_from_query = req_uri.query();

    let data = hyper::body::to_bytes(req.into_body()).await?;

    let response;

    let json_data: serde_json::Result<SwitchAuth> = serde_json::from_slice(&data);

    let hashes = keystore
        .lock()
        .expect("lock keystore")
        .keystore
        .api_keys_author
        .clone();

    match json_data {
        Ok(device_auth) => {
            if authenticate(&device_auth.device, hashes.clone()) {
                println!(
                    "GET /current_channel -- {:?} -- authorized request by device",
                    timestamp_in_sec()
                );

                let channel_state = channel_state.lock().expect("");
                let channel_id = channel_state.channel_id.clone();

                response = Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(channel_id))?;
            } else {
                response = Response::builder()
                    .status(StatusCode::UNAUTHORIZED)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        "Unauthorized - Device Name sent by device doesn't match the configuration",
                    ))?;
                println!(
                    "GET /current_channel -- {:?} -- unauthorized request blocked",
                    timestamp_in_sec()
                );
            }
        }
        // if there is no json in the body => check Uri
        Err(_) => match device_from_query {
            Some(id) => {
                if authenticate(&id, hashes.clone()) {
                    println!(
                        "GET /current_channel -- {:?} -- authorized request by device",
                        timestamp_in_sec()
                    );

                    let channel_state = channel_state.lock().expect("");
                    let channel_id = channel_state.channel_id.clone();

                    response = Response::builder()
                        .status(StatusCode::OK)
                        .header(header::CONTENT_TYPE, "application/json")
                        .body(Body::from(channel_id))?;
                } else {
                    response = Response::builder()
                    .status(StatusCode::UNAUTHORIZED)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        "Unauthorized - Device Name sent by device doesn't match the configuration",
                    ))?;
                    println!(
                        "GET /current_channel -- {:?} -- unauthorized request blocked",
                        timestamp_in_sec()
                    );
                }
            }
            None => {
                response = Response::builder()
                    .status(StatusCode::UNAUTHORIZED)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        "Unauthorized - No device_id provided in Request Body or Uri",
                    ))?;
                println!(
                    "GET /current_channel -- {:?} -- unauthorized request blocked",
                    timestamp_in_sec()
                );
            }
        },
    }
    Ok(response)
}
