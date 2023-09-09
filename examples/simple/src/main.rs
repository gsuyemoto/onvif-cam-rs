use anyhow::Result;
use onvif_cam_rs::builder::camera::CameraBuilder;
use onvif_cam_rs::client::{self, Messages};
use onvif_cam_rs::device::camera::Camera;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    println!("----------------------- DEVICE DISCOVERY ----------------------");

    // let mut devices = client::discover().await?;
    // let mut cameras: Vec<Camera> = Vec::new();

    // for device in devices {
    //     let mut camera = Camera::new(device);
    //     camera.build_all().await?;
    //     cameras.push(camera);
    // }

    // match &cameras[0].stream.uri {
    //     Some(url) => println!("Stream uri: {url}"),
    //     None => panic!("Ooops"),
    // }

    let mut camera = Camera::from("http://192.168.86.218:8080/onvif/device_service");
    camera.build_all().await?;

    Ok(())
}
