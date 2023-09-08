/*!

ONVIF is a industry protocol used widely by many IP cameras. If a camera is ONVIF compliant, you can discover it's IP address and query it for various capabilities and specifications.

This Rust lib provides, at the moment, a very barebones implementation of some of the protocol. More is planned.

# Getting Started

When creating a new Client object, the Client will first look to see if there is a file in the base directory to provide information about cameras and IP addresses. If the file is not present, then the Client will broadcast a predefined message on the network and compliant cameras should reply with their IP address. With the IP address in hand, you can then continue to query the devices for more information.

```Rust
use anyhow::Result;
use onvif_cam_rs::builder::camera::CameraBuilder;
use onvif_cam_rs::client::{self, Messages};
use onvif_cam_rs::device::camera::Camera;

#[tokio::main]
async fn main() -> Result<()> {
    // Find all IP Devices on local network using ONVIF

    let mut devices = client::discover().await?;
    let mut cameras: Vec<Camera> = Vec::new();

    // Enumerate all Camera devices found
    for device in devices {
        let mut camera = Camera::new(device);
        camera.build_all().await?;
        cameras.push(camera);
    }

    // Get the RTSP streaming URL for the first camera
    match &cameras[0].stream.uri {
        Some(url) => println!("Stream uri: {url}"),
        None => panic!("Ooops"),
    }
}

```


*/

pub mod builder;
pub mod client;
pub mod device;
pub(crate) mod utils;
