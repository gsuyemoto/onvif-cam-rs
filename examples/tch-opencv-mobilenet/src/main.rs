use anyhow::{bail, Result};
use onvif_cam_rs::client::{Client, Messages};
use opencv::videoio::{self, VideoCapture, CAP_FFMPEG, CAP_PROP_FOURCC};
use opencv::{core, highgui, imgproc, prelude::*};
use std::collections::HashMap;
use std::{process, time};

#[tokio::main]
pub async fn main() -> Result<()> {
    env_logger::init();

    println!("----------------------- DEVICE DISCOVERY ----------------------");

    let mut onvif_client = Client::new().await;

    println!("----------------------- GET STREAM URI ----------------------");

    let _ = onvif_client.send(Messages::Capabilities, 0).await?;
    let _ = onvif_client.send(Messages::DeviceInfo, 0).await?;
    let _ = onvif_client.send(Messages::Profiles, 0).await?;
    let stream_url = onvif_client.send(Messages::GetStreamURI, 0).await?;

    println!("[Main] stream uri: {stream_url}");
    println!("----------------------- OPEN CAMERA STREAM! ----------------------");

    // Initialize OpenCV
    let win_name = "Video";
    highgui::named_window(win_name, 1)?;
    highgui::resize_window(win_name, 640, 352)?;

    // Open the RTSP stream
    let mut capture = VideoCapture::from_file(&stream_url, CAP_FFMPEG)?;

    // Get the FourCC codec code
    let codec_code = capture.get(CAP_PROP_FOURCC).unwrap() as i32;

    // Convert the codec code to a human-readable string
    let codes = &[
        (codec_code & 0xFF) as u8,
        ((codec_code >> 8) & 0xFF) as u8,
        ((codec_code >> 16) & 0xFF) as u8,
        ((codec_code >> 24) & 0xFF) as u8,
    ];
    let codec_fourcc = String::from_utf8_lossy(codes);

    println!("Codec FourCC: {}", codec_fourcc);

    // cifar 10 classes and their indexes
    let class_names = vec![
        "plane", "car", "bird", "cat", "deer", "dog", "frog", "horse", "ship", "truck",
    ];
    let mut class_map = HashMap::new();
    for (idx, class_name) in class_names.iter().enumerate() {
        let idx = idx as i32;
        class_map.insert(idx, class_name);
    }

    // model trained on 32x32 images. (CIFAR10)
    const CIFAR_WIDTH: i32 = 32;
    const CIFAR_HEIGHT: i32 = 32;

    // time that a frame will stay on screen in ms
    const DELAY: i32 = 30;

    // create empty Mat to store image data
    let mut frame = Mat::default();

    // load jit model and put it to cuda
    let mut model = tch::CModule::load("cifar10_mobilenet_v3_small.pt")?;
    model.set_eval();
    // model.to(tch::Device::Cuda(0), tch::Kind::Float, false);
    model.to(tch::Device::Cpu, tch::Kind::Float, false);

    let is_video_on = capture.is_opened()?;
    let mut frame_num = 0;

    if !is_video_on {
        println!("Could'not open video.");
        process::exit(0);
    } else {
        loop {
            // read frame to empty mat
            capture.read(&mut frame)?;

            // Only run inference every 20 frames
            frame_num += 1;
            if frame_num == 20 {
                frame_num = 0;

                // resize image
                let mut resized = Mat::default();
                imgproc::resize(
                    &frame,
                    &mut resized,
                    core::Size {
                        width: CIFAR_WIDTH,
                        height: CIFAR_HEIGHT,
                    },
                    0.0,
                    0.0,
                    opencv::imgproc::INTER_LINEAR,
                )?;

                // convert bgr image to rgb
                let mut rgb_resized = Mat::default();
                imgproc::cvt_color(&resized, &mut rgb_resized, imgproc::COLOR_BGR2RGB, 0)?;

                // get data from Mat
                let h = resized.size()?.height;
                let w = resized.size()?.width;
                let resized_data = resized.data_bytes_mut()?;

                // convert bytes to tensor
                // let tensor = tch::Tensor::from_data_size(resized_data, &[h as i64, w as i64, 3], tch::Kind::Uint8);
                // let tensor = tch::Tensor::f_of_data_size(
                let tensor = tch::Tensor::f_from_data_size(
                    resized_data,
                    &[h as i64, w as i64, 3],
                    tch::Kind::Uint8,
                )?;
                // normalize image tensor
                let tensor = tensor.to_kind(tch::Kind::Float) / 255;
                // carry tensor to cuda
                // let tensor = tensor.to_device(tch::Device::Cuda(0));
                let tensor = tensor.to_device(tch::Device::Cpu);
                // convert (H, W, C) to (C, H, W)
                let tensor = tensor.permute(&[2, 0, 1]);
                // add batch dim (convert (C, H, W) to (N, C, H, W))
                let normalized_tensor = tensor.unsqueeze(0);

                // make prediction and time it.
                let start = time::Instant::now();
                let probabilites = model
                    .forward_ts(&[normalized_tensor])?
                    .softmax(-1, tch::Kind::Float);
                let predicted_class = i32::try_from(probabilites.argmax(None, false)).unwrap();
                let probability_of_class = f32::try_from(probabilites.max());
                let duration = start.elapsed();
                println!(
                    "Predicted class: {:?}, probability of it: {:?}, prediction time: {:?}",
                    class_map[&predicted_class], probability_of_class, duration
                );
            }

            // show image
            highgui::imshow(win_name, &frame)?;
            let key = highgui::wait_key(DELAY)?;

            // if button q pressed, abort.
            if key == 113 {
                highgui::destroy_all_windows()?;
                println!("Pressed q. Aborting program.");
                break;
            }
        }
    }

    Ok(())
}
