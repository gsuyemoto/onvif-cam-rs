use anyhow::Result;
use onvif_client_rs::{Messages, OnvifClient};
use opencv::{
    highgui::{imshow, wait_key},
    imgproc::{cvt_color, rectangle, COLOR_BGR2GRAY},
    objdetect,
    prelude::*,
    videoio::{VideoCapture, CAP_FFMPEG},
};

#[tokio::main]
async fn main() -> Result<()> {
    println!("----------------------- DEVICE DISCOVERY ----------------------");

    let mut onvif_client = OnvifClient::new().discover().await?;

    println!("----------------------- GET STREAM URI ----------------------");

    let streaming_uri = onvif_client.send(Messages::GetStreamURI).await?;
    println!("socket uri: {streaming_uri}");

    println!("----------------------- OPEN CAMERA STREAM! ----------------------");

    // Initialize OpenCV
    opencv::highgui::named_window("Video", 1)?;

    // Load the Haarcascade classifier for face detection
    let mut face_cascade = objdetect::CascadeClassifier::new(
        "/usr/share/opencv4/haarcascades/haarcascade_frontalface_default.xml",
    )?;

    println!("Loaded haarcascade...");

    // Open the RTSP stream
    let mut capture = VideoCapture::from_file(&streaming_uri, CAP_FFMPEG)?;

    // Capture and display video frames
    let mut frame = Mat::default();

    // Detect face every nth frame
    let mut frame_skip = 10;

    // Detect faces in the image
    let mut faces = opencv::types::VectorOfRect::new();

    loop {
        if capture.read(&mut frame)? {
            // Decrement frame_skip
            frame_skip -= 1;

            if frame_skip <= 0 {
                frame_skip = 10;

                // Convert the image to grayscale (required for detection)
                let mut gray = Mat::default();
                cvt_color(&mut frame, &mut gray, COLOR_BGR2GRAY, 0)?;

                face_cascade.detect_multi_scale(
                    &gray,
                    &mut faces,
                    1.6, // scale for faster speed but less accurate
                    3,
                    0,
                    Default::default(),
                    Default::default(),
                )?;
            }

            if !faces.is_empty() {
                // Draw rectangles around detected faces
                for face in faces.iter() {
                    // let top_left = face.tl();
                    // let bottom_right = face.br();
                    rectangle(
                        &mut frame,
                        face,
                        opencv::core::Scalar::new(0.0, 0.0, 255.0, 0.0),
                        2,
                        8,
                        0,
                    )?;
                }
            }

            imshow("Video", &frame)?;

            let key = wait_key(10)?;
            if key > 0 && key != 255 {
                break;
            }
        } else {
            break;
        }
    }

    Ok(())
}
