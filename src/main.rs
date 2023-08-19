use anyhow::Result;
use bytes::{Bytes, BytesMut};
use opencv::{
    highgui::{imshow, named_window, wait_key},
    imgproc::{get_text_size, rectangle, FONT_HERSHEY_SIMPLEX},
    objdetect,
    prelude::*,
    videoio::{VideoCapture, CAP_FFMPEG},
};
use reqwest::{Client, RequestBuilder};
use std::io::BufReader;
use std::net::{SocketAddr, UdpSocket};
use xml::reader::{EventReader, XmlEvent};

mod yolov8_onnx;

/// All of the ONVIF requests that this program supports
#[derive(Debug)]
enum Messages {
    Discovery,
    Capabilities,
    DeviceInfo,
    Profiles,
    GetStreamURI,
}

#[derive(Debug)]
struct DetectedObject {
    bounding_box: opencv::core::Rect,
    class_id: i32,
    confidence: f32,
}

#[tokio::main]
async fn main() -> Result<()> {
    let socket = UdpSocket::bind("0.0.0.0:0")?;

    //------------------- DISCOVER ALL ONVIF DEVICES
    //-------------------

    // discovery is using SOAP UDP based in ws-discovery
    // and therefore just uses a raw udpsocket
    let soap_message = get_message(Messages::Discovery);

    // Don't make these const as they are used only once
    // and therefore after use we can drop from memory
    let send_ip = "239.255.255.250";
    let send_port = 3702;

    let socket_buffer = discover_devices(&socket, send_ip, send_port, soap_message);

    // main thing we need here is the xaddrs
    // which is an HTTP URL to which we call later
    // for "device management"
    let xaddrs = parse_soap_find(&socket_buffer, Some("XAddrs"));
    // parse_soap_find(&socket_buffer, None);

    println!("----------------------- GET STREAM URI ----------------------");

    // after discovery, the xaddrs in the reply from each device
    // will reveal the url needed for device management
    // here the communication switches to requests sent via
    // HTTP, but still using SOAP
    // we are going to use reqwest to create HTTP requests

    let soap_message = get_message(Messages::GetStreamURI);

    let response_bytes = onvif_message(&xaddrs, soap_message).await?;
    let streaming_uri = parse_soap_find(&response_bytes, Some("Uri"));

    println!("uri: {streaming_uri}");

    println!("----------------------- OPEN CAMERA STREAM! ----------------------");

    // Initialize OpenCV
    named_window("Video", 1)?;

    // Open the RTSP stream
    let mut capture = VideoCapture::from_file(&streaming_uri, CAP_FFMPEG)?;

    // Capture and display video frames
    let mut frame = Mat::default();

    // Detect face every nth frame
    let mut frame_skip = 10;

    loop {
        if capture.read(&mut frame)? {
            // Decrement frame_skip
            frame_skip -= 1;

            if frame_skip <= 0 {
                frame_skip = 10;

                let detected_objects = detect_objects_on_image(&frame);

                // Draw bounding boxes around detected objects
                for obj in detected_objects {
                    let class_id = obj.class_id;
                    let label = format!("Face {:.2}", obj.confidence);

                    let color = Scalar::new(0.0, 255.0, 0.0, 0.0);
                    rectangle(&mut image, obj.bounding_box, color, 2, 8, 0)?;

                    // Draw label text
                    let mut label_size = Size::default();
                    let baseline = 0;
                    get_text_size(
                        &label,
                        FONT_HERSHEY_SIMPLEX,
                        0.5,
                        1,
                        &mut baseline,
                        &mut label_size,
                    )?;
                    let label_origin = opencv::core::Point::new(
                        obj.bounding_box.x,
                        obj.bounding_box.y - label_size.height - baseline,
                    );
                    opencv::imgproc::put_text(
                        &mut image,
                        &label,
                        label_origin,
                        FONT_HERSHEY_SIMPLEX,
                        0.5,
                        color,
                        1,
                        8,
                        false,
                    )?;

                    println!("Detected face at confidence {:.2}", obj.confidence);
                }
            }

            imshow("Video", &frame)?;
            let key = wait_key(0)?;
        } else {
            break;
        }
    }

    Ok(())
}
fn blob_from_image(image: &Mat) -> Result<Mat, opencv::Error> {
    // Prepare the image for YOLO input
    let scale_factor = 1.0 / 255.0;
    let input_size = opencv::core::Size::new(416, 416);
    let input_mean = opencv::core::Scalar::new(0.0, 0.0, 0.0, 0.0);
    let input_swap_rb = true;
    let crop = false;

    let mut blob = opencv::dnn::blob_from_image(
        image,
        scale_factor,
        input_size,
        input_mean,
        input_swap_rb,
        crop,
        opencv::core::CV_32F,
    )?;
    Ok(blob)
}

fn post_process(
    outs: &types::VectorOfMat,
    image: &Mat,
    conf_threshold: f32,
    nms_threshold: f32,
) -> Result<Vec<DetectedObject>, opencv::Error> {
    let class_ids: Vec<i32> = vec![0]; // 0 represents the "person" class
    let num_classes = class_ids.len();

    let mut detected_objects = Vec::new();

    for out in outs.iter() {
        let data = out.at_2d::<f32>(0, 0)?;

        for i in 0..out.rows() {
            let scores = data.col_range(i * num_classes..(i + 1) * num_classes)?;
            let max_score = scores.max()?;
            let class_id = scores.argmax(0)? as i32;

            if max_score > conf_threshold && class_ids.contains(&class_id) {
                let bounding_box = out.at_2d::<f32>(i, 0)?;
                let x = bounding_box.at::<f32>(0)?;
                let y = bounding_box.at::<f32>(1)?;
                let width = bounding_box.at::<f32>(2)?;
                let height = bounding_box.at::<f32>(3)?;

                let left = (x * image.cols() as f32) - (width * image.cols() as f32 / 2.0);
                let top = (y * image.rows() as f32) - (height * image.rows() as f32 / 2.0);
                let right = left + (width * image.cols() as f32);
                let bottom = top + (height * image.rows() as f32);

                let bounding_box = opencv::core::Rect::new(
                    left as i32,
                    top as i32,
                    (right - left) as i32,
                    (bottom - top) as i32,
                );

                detected_objects.push(DetectedObject {
                    bounding_box,
                    class_id,
                    confidence: max_score,
                });
            }
        }
    }
}

/// Returns the response received when sending an ONVIF request to a
/// device found via device discovery
/// The reponse is SOAP formatted as byte array
///
/// # Arguments
///
/// * `device_url` - The URL provided in the xaddrs via device discovery
/// * `soap_msg` - The SOAP request formatted as a String
///
/// # Examples
///
/// ```
/// let soap_message = "<s:Envelope xmlns:s='http://www.w3.org/2003/05/soap-envelope'><soap:Body><trt:GetProfiles/></soap:Body></s:Envelope>";
/// let response_bytes = onvif_message("http://192.168.1.100:8080/onvif/device_services", soap_message.to_string());
/// ```
async fn onvif_message(device_url: &str, soap_msg: String) -> Result<Bytes> {
    let client = Client::new();
    let request: RequestBuilder = client
        .post(device_url)
        .header("Content-Type", "application/soap+xml; charset=utf-8")
        .body(soap_msg);

    // Send the HTTP request and receive the response
    let response = request.send().await?;
    let response_bytes = response.bytes().await?;

    Ok(response_bytes)
}

/// Sends a multicast request via raw udpsocket on LAN.
/// Request is in the form of a SOAP message.
/// Response is also a SOAP message that will contain
/// the xaddrs of the all the responding devices. Use
/// this URL to subsequently make calls via HTTP requests
/// to send commands to the device using ONVIF protocol.
///
/// # Arguments
///
/// * `socket` - Pass in a udpsocket to be used to send request
/// * `send_ip` - The broadcast IP which is normally "239.255.255.250"
/// * `send_port` - The port to send request, normally 3702
/// * `message` - The SOAP message request as a String
///
fn discover_devices(socket: &UdpSocket, send_ip: &str, send_port: u16, message: String) -> Bytes {
    // Destination address
    let destination = format!("{}:{}", send_ip, send_port);
    let destination: SocketAddr = destination.parse().unwrap();

    // Send the SOAP message over UDP
    let success = socket.send_to(message.as_bytes(), destination);

    match success {
        Ok(size) => println!("Successfully sent message of size {size}"),
        Err(e) => eprintln!("Error sending {e}"),
    }

    // Receive response
    let mut socket_buffer = BytesMut::from(&[0; 4096][..]);
    let success = socket.recv_from(&mut socket_buffer);

    match success {
        Ok((size, _)) => {
            println!("Successfully received message of size {size}")
        }
        Err(e) => eprintln!("Error receiving {e}"),
    }

    socket_buffer.into()
}

fn parse_soap_find(socket_buffer: &Bytes, find: Option<&str>) -> String {
    // get XAddrs
    let mut element_found = String::new();
    let mut element_start = false;

    let buffer = BufReader::new(socket_buffer.as_ref());
    let parser = EventReader::new(buffer);
    let mut depth = 0;

    for e in parser {
        match e {
            Ok(XmlEvent::StartElement { name, .. }) => match find {
                Some(el_to_find) => {
                    if name.local_name == el_to_find {
                        element_start = true;
                    }
                }
                None => {
                    depth += 1;
                    println!("{:spaces$}+{name}", "", spaces = depth * 2);
                }
            },
            Ok(XmlEvent::EndElement { name, .. }) => match find {
                Some(el_to_find) => {
                    if name.local_name == el_to_find {
                        element_start = false;
                    }
                }
                None => {
                    depth -= 1;
                    println!("{:spaces$}+{name}", "", spaces = depth * 2);
                }
            },
            Ok(XmlEvent::Characters(chars)) => match find {
                Some(_) => {
                    if element_start {
                        element_found = chars;
                    }
                }
                None => {
                    println!("{chars}");
                }
            },
            Err(e) => {
                eprintln!("Error: {e}");
                break;
            }
            // There's more: https://docs.rs/xml-rs/latest/xml/reader/enum.XmlEvent.html
            _ => {}
        }
    }

    element_found
}

fn get_message(msg_type: Messages) -> String {
    match msg_type {
        Messages::Discovery => format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
                        <e:Envelope xmlns:e="http://www.w3.org/2003/05/soap-envelope"
                        xmlns:w="http://schemas.xmlsoap.org/ws/2004/08/addressing"
                        xmlns:d="http://schemas.xmlsoap.org/ws/2005/04/discovery"
                        xmlns:dn="http://www.onvif.org/ver10/network/wsdl">
                <e:Header>
                    <w:MessageID>uuid:8d6ab73e-280a-4f23-967d-d2ec20b6d893</w:MessageID>
                    <w:To>urn:schemas-xmlsoap-org:ws:2005:04:discovery</w:To>
                    <w:Action>http://schemas.xmlsoap.org/ws/2005/04/discovery/Probe</w:Action>
                </e:Header>
                <e:Body>
                    <d:Probe>
                        <d:Types>dn:NetworkVideoTransmitter</d:Types>
                    </d:Probe>
                </e:Body>
            </e:Envelope>"#
        ),

        // Messages::Discovery => format!(
        //     r#"<?xml version="1.0" encoding="UTF-8"?>
        //     <s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope"
        //                 xmlns:a="http://schemas.xmlsoap.org/ws/2004/08/addressing"
        //                 xmlns:d="http://schemas.xmlsoap.org/ws/2005/04/discovery"
        //                 xmlns:dn="http://www.onvif.org/ver10/network/wsdl">
        //         <s:Header>
        //             <a:Action d:mustUnderstand="1">http://schemas.xmlsoap.org/ws/2005/04/discovery/Probe</a:Action>
        //             <a:MessageID>uuid:72d76f2a-23d5-4181-9ea2-1ade1ca198b9</a:MessageID>
        //             <a:ReplyTo>
        //                 <a:Address>http://schemas.xmlsoap.org/ws/2004/08/addressing/role/anonymous</a:Address>
        //             </a:ReplyTo>
        //             <a:To>urn:schemas-xmlsoap-org:ws:2005:04:discovery</a:To>
        //         </s:Header>
        //         <s:Body>
        //             <d:Probe>
        //                 <d:Types>dn:NetworkVideoTransmitter</d:Types>
        //             </d:Probe>
        //         </s:Body>
        //     </s:Envelope>"#
        // ),
        Messages::Capabilities => format!(
            r#"<Envelope xmlns="http://www.w3.org/2003/05/soap-envelope"
                         xmlns:tds="http://www.onvif.org/ver10/device/wsdl">
               <Header/>
               <Body>
                   <tds:GetCapabilities>
                       <tds:Category>All</tds:Category>
                   </tds:GetCapabilities>
               </Body>
            </Envelope>"#
        ),
        // Messages::Capabilities => format!(
        //     r#"
        //     <Envelope xmlns="http://www.w3.org/2003/05/soap-envelope" xmlns:trt="http://www.onvif.org/ver10/media/wsdl">
        //         <Header/>
        //         <Body>
        //             <trt:GetCapabilities>
        //                 <trt:Category>Media</trt:Category>
        //             </trt:GetCapabilities>
        //         </Body>
        //     </Envelope>
        // "#
        // ),
        Messages::DeviceInfo => format!(
            r#"
            <s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope"
                         xmlns:tds="http://www.onvif.org/ver10/device/wsdl">
                <s:Body>
                    <tds:GetDeviceInformation/>
                </s:Body>
            </s:Envelope>
        "#
        ),
        Messages::Profiles => format!(
            r#"
            <s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope"
                         xmlns:tds="http://www.onvif.org/ver10/device/wsdl">
                <soap:Body>
                    <trt:GetProfiles/>
                </soap:Body>
            </s:Envelope>
        "#
        ),
        Messages::GetStreamURI => format!(
            r#"
            <soap:Envelope
                xmlns:soap="http://www.w3.org/2003/05/soap-envelope"
                xmlns:trt="http://www.onvif.org/ver10/media/wsdl">
                <soap:Body>
                    <trt:GetStreamUri>
                        <trt:StreamSetup>
                            <tt:Stream>RTP-multicast</tt:Stream>
                            <tt:Transport>
                                <tt:Protocol>RTSP</tt:Protocol>
                            </tt:Transport>
                        </trt:StreamSetup>
                    </trt:GetStreamUri>
                </soap:Body>
            </soap:Envelope>
        "#
        ),
    }
}
