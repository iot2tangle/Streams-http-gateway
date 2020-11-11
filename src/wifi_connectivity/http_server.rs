use crate::device_auth::keystore::KeyManager;
use crate::types::{channel_state::ChannelState, config::Config};
use crate::wifi_connectivity::handlers::*;

use hyper::service::{make_service_fn, service_fn};

use std::sync::{Arc, Mutex};

use hyper::{Body, Method, Request, Response, Server, StatusCode};
type GenericError = Box<dyn std::error::Error + Send + Sync>;
type Result<T> = std::result::Result<T, GenericError>;
static NOTFOUND: &[u8] = b"Not Found";

///
/// Starts the server on the provided port, the server will hand over requests to the handler functions
///
pub async fn start(
    config: Config,
    channel_state: Arc<Mutex<ChannelState>>,
    keystore: Arc<Mutex<KeyManager>>,
) -> Result<()> {
    let addr = ([0, 0, 0, 0], config.port).into();

    let service = make_service_fn(move |_| {
        let channel_state = channel_state.clone();
        let keystore = keystore.clone();
        let config = config.clone();
        async {
            Ok::<_, GenericError>(service_fn(move |req| {
                responder(req, channel_state.clone(), keystore.clone(), config.clone())
            }))
        }
    });

    let server = Server::bind(&addr).serve(service);

    println!("Listening on http://{}", addr);

    server.await?;

    Ok(())
}

async fn responder(
    req: Request<Body>,
    channel_state: Arc<Mutex<ChannelState>>,
    keystore: Arc<Mutex<KeyManager>>,
    config: Config,
) -> Result<Response<Body>> {
    match (req.method(), req.uri().path()) {
        (&Method::POST, "/sensor_data") => sensor_data_response(req, channel_state, keystore).await,
        (&Method::POST, "/bundle_data") => send_bundle_response(req, channel_state, keystore).await,
        (&Method::POST, "/switch_channel") => {
            switch_channel_response(req, channel_state, keystore, config).await
        }
        (&Method::GET, "/current_channel") => {
            get_current_channel(req, channel_state, keystore).await
        }
        (&Method::GET, "/status") => status_response().await,
        _ => Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(NOTFOUND.into())
            .unwrap()),
    }
}
