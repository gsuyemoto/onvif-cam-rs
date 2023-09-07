/*!

ONVIF is a industry protocol used widely by many IP cameras. If a camera is ONVIF compliant, you can discover it's IP address and query it for various capabilities and specifications.

This Rust lib provides, at the moment, a very barebones implementation of some of the protocol. More is planned.

# Getting Started

When creating a new Client object, the Client will first look to see if there is a file in the base directory to provide information about cameras and IP addresses. If the file is not present, then the Client will broadcast a predefined message on the network and compliant cameras should reply with their IP address. With the IP address in hand, you can then continue to query the devices for more information.

```Rust
use onvif_cam_rs::client::{Client, Message};

#[tokio::main]
async fn main() -> Result<()> {
    let mut onvif_client = Client::new().await;
    let rtsp_addr = onvif_client.send(Messages::GetStreamURI, 0).await?;

    // Provide rtsp_addr to your choice of RTSP clients
}

```

Logging is integrated into the lib and you can get some useful information by using it.
Let's use pretty_env_logger crate:

```Rust
use onvif_cam_rs::client::{Client, Message};

#[tokio::main]
async fn main() -> Result<()> {
    pretty_env_logger::init();

    let mut onvif_client = Client::new().await;
    let _ = onvif_client.send(Messages::Capabilities, 0).await?;
    let _ = onvif_client.send(Messages::DeviceInfo, 0).await?;
    let _ = onvif_client.send(Messages::Profiles, 0).await?;
    let rtsp_addr = onvif_client.send(Messages::GetStreamURI, 0).await?;

    // Provide rtsp_addr to your choice of RTSP clients
}

```

*/

pub mod camera;
pub mod client;
pub mod device;
pub mod io;
