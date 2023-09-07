use anyhow::Result;
use onvif_cam_rs::client::{Client, Messages};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    println!("----------------------- DEVICE DISCOVERY ----------------------");

    let mut devices = Client::discover().await?;

    for device in devices {
        onvif_client
            .send(Messages::Capabilities, dev_index)
            .await?
            .send(Messages::DeviceInfo, dev_index)
            .await?
            .send(Messages::Profiles, dev_index)
            .await?
            .send(Messages::GetDNS, dev_index)
            .await?
            .send(Messages::GetStreamURI, dev_index)
            .await?;

        println!("[Main] stream uri: {}", onvif_client.get_stream_uri(0)?);
    }

    Ok(())
}
