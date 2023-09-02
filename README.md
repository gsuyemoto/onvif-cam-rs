# ONVIF Rust LIB

ONVIF is a industry protocol used widely by many IP cameras. If a camera is ONVIF compliant, you can discover it's IP address and query it for various capabilities and specifications.

This Rust lib provides, at the moment, a very barebones implementation of some of the protocol. More is planned.

This is a very bare bones implementation of the ONVIF protocol. The following messages are implemented:

* Discovery
* Capabilities
* DeviceInfo
* Profiles
* GetStreamURI

Implementation of those messages are bare basics and don't store or parse the entire SOAP response in many cases. This whole lib is really in support of an RTSP/RTP/H264 streaming client I wrote at https://github.com/gsuyemoto/rtsp-rtp-rs, which will become a Rust crate soon.

The example discovers an IP camera (only tested on a single Topodome) and then uses OpenCV to stream via RTP and detect faces via Haar cascades.

In order to run the example, you will need Clang and OpenCV. On Debian Linux:
```bash
sudo apt-get install libclang-dev libopencv-dev
```

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
