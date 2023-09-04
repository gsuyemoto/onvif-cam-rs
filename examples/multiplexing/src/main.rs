use anyhow::Result;
use onvif_cam_rs::client::{Client, Messages};
use opencv::{
    core::{hconcat, Size},
    highgui::{self, imshow, wait_key},
    imgproc::{self, cvt_color, rectangle, COLOR_BGR2GRAY},
    objdetect,
    prelude::*,
    videoio::{VideoCapture, CAP_FFMPEG, CAP_PROP_FOURCC},
};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    println!("----------------------- DEVICE DISCOVERY ----------------------");

    let mut onvif_client = Client::new().await;

    println!("----------------------- GET STREAM URI ----------------------");

    let _ = onvif_client.send(Messages::Capabilities, 0).await?;
    let _ = onvif_client.send(Messages::DeviceInfo, 0).await?;
    let _ = onvif_client.send(Messages::Profiles, 0).await?;
    let stream_url_01 = onvif_client.send(Messages::GetStreamURI, 0).await?;
    let stream_url_02 = onvif_client.send(Messages::GetStreamURI, 1).await?;

    println!("----------------------- OPEN CAMERA STREAM! ----------------------");

    // Open the RTSP stream
    let mut capture_01 = VideoCapture::from_file(&stream_url_01, CAP_FFMPEG)?;
    let mut capture_02 = VideoCapture::from_file(&stream_url_02, CAP_FFMPEG)?;

    // Get the FourCC codec code
    let codec_code_01 = capture_01.get(CAP_PROP_FOURCC).unwrap() as i32;
    let codec_code_02 = capture_02.get(CAP_PROP_FOURCC).unwrap() as i32;

    println!("Codec cam 01: {}", get_codec(codec_code_01));
    println!("Codec cam 02: {}", get_codec(codec_code_02));

    // Capture and display video frames
    let mut frame = Mat::default();

    // Detect face every nth frame
    let mut frame_skip = 10;

    // Detect faces in the image
    let mut faces = opencv::types::VectorOfRect::new();

    const WIDTH: i32 = 640;
    const HEIGHT: i32 = 352;

    loop {
        let mut frame1 = Mat::default();
        let mut frame2 = Mat::default();

        capture_01.read(&mut frame1)?;
        capture_02.read(&mut frame2)?;

        let mut resized1 = Mat::default();
        if frame1.size()?.width > 0 {
            // resize image
            imgproc::resize(
                &frame1,
                &mut resized1,
                Size {
                    width: WIDTH,
                    height: HEIGHT,
                },
                0.0,
                0.0,
                opencv::imgproc::INTER_LINEAR,
            )?;
        }

        let mut resized2 = Mat::default();
        if frame2.size()?.width > 0 {
            // resize image
            imgproc::resize(
                &frame2,
                &mut resized2,
                Size {
                    width: WIDTH,
                    height: HEIGHT,
                },
                0.0,
                0.0,
                opencv::imgproc::INTER_LINEAR,
            )?;
        }

        // Multiplex frames and display in a single window
        let mut multiplexed_frame = Mat::default();

        // Combine and multiplex frames (e.g., using hconcat)
        // Example: core::hconcat(&vec![&frame1, &frame2], &mut multiplexed_frame)?;
        let mut vec_of_mat: opencv::core::Vector<Mat> = opencv::core::Vector::new();
        vec_of_mat.push(resized1.clone());
        vec_of_mat.push(resized2.clone());

        hconcat(&vec_of_mat, &mut multiplexed_frame)?;
        opencv::highgui::imshow("Multiplexed Video", &multiplexed_frame)?;

        if highgui::wait_key(1)? > 0 {
            break;
        }
    }

    Ok(())
}

fn get_codec(codec_code: i32) -> String {
    // Convert the codec code to a human-readable string
    let codes = &[
        (codec_code & 0xFF) as u8,
        ((codec_code >> 8) & 0xFF) as u8,
        ((codec_code >> 16) & 0xFF) as u8,
        ((codec_code >> 24) & 0xFF) as u8,
    ];

    String::from_utf8_lossy(codes).to_string()
}
