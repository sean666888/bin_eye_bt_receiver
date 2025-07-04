use bluer::{
    agent::Agent,
    rfcomm::{Profile, Role},
};
use clap::Parser;
use enigo::{Enigo, Keyboard, Settings};
use env_logger;
use futures::StreamExt;
use log::{debug, error, info, warn};
use percent_encoding_rfc3986::{self as rfc3986, utf8_percent_encode, AsciiSet};
use std::str;
use tokio::io::AsyncReadExt;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// UUID of scaner
    #[arg(short, long, default_value = "8a8478c9-2ca8-404b-a0de-101f34ab71ae")]
    uuid: String,
    /// Enable keyboard output
    #[arg(short, long, default_value_t = false)]
    keyboard: bool,
}

const LOG_UNSAFE_CHARS: &'static AsciiSet = &rfc3986::CONTROLS.add(b'\\').add(b'"');

#[tokio::main]
async fn main() -> bluer::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args = Args::parse();
    let my_uuid: uuid::Uuid = bluer::Uuid::parse_str(&args.uuid).unwrap();
    debug!("My UUID: {}", my_uuid.urn());
    let mut enigo = Enigo::new(&Settings::default()).unwrap();

    let session = bluer::Session::new().await?;
    let adapter = session.default_adapter().await?;
    if !(adapter.is_powered().await.unwrap_or(false)) {
        info!("Bluetooth adapter powered off, powering it on.");
        adapter.set_powered(true).await?;
    }
    //adapter.set_discoverable(true).await?;
    //adapter.set_discoverable_timeout(0).await?;
    //adapter.set_pairable(false).await?;
    let agent = Agent::default();
    let _agent_hndl = session.register_agent(agent).await?;
    let profile: Profile = Profile {
        uuid: my_uuid,
        name: Some("Binary Eye Bluetooth receiver".to_string()),
        channel: Some(0),
        role: Some(Role::Server),
        require_authentication: Some(false),
        require_authorization: Some(false),
        auto_connect: Some(true),
        ..Default::default()
    };
    let mut hndl = session.register_profile(profile).await?;
    loop {
        info!("Waiting for connection on RFCOMM channel 0?");
        let req = hndl.next().await.expect("Received no connect request");

        info!("Accepted connection from: {}", req.device());
        let mut stream = req.accept()?;
        loop {
            let buf_size = 1024;
            let mut buf = vec![0; buf_size as _];

            let n = match stream.read(&mut buf).await {
                Ok(0) => {
                    error!("Stream ended");
                    break;
                }
                Ok(n) => n,
                Err(err) => {
                    error!("Read failed: {}", &err);
                    break;
                }
            };
            let buf = &buf[..n];

            let s = match str::from_utf8(buf) {
                Ok(s) => s,
                Err(e) => {
                    warn!("Invalid UTF-8 sequence: {}", e);
                    continue;
                }
            };
            info!(
                "Received String: \"{}\"",
                utf8_percent_encode(s, LOG_UNSAFE_CHARS)
            );

            if args.keyboard {
                enigo.text(s).unwrap_or_else(|e| error!("{}", e));
            }
        }
    }
}
