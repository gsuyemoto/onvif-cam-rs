use crate::device::*;

use anyhow::{anyhow, Result};
use reqwest::RequestBuilder;
use std::{io::BufReader, net::SocketAddr, time::Duration};
use tokio::{net::UdpSocket, time::timeout};
use xml::reader::{EventReader, XmlEvent};
//------ Saving File
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
//------ For Debugging Only
#[cfg(debug_assertions)]
use log::{debug, info};

const DISCOVER_URI: &'static str = "239.255.255.250:3702";
const CLIENT_LISTEN_IP: &'static str = "0.0.0.0:0"; // notice port is 0
const FILE_FOUND_DEVICES: &'static str = "devices_found.txt";

/// All of the ONVIF requests that this program plans to support
#[derive(Debug)]
pub enum Messages {
    Discovery,
    Capabilities,
    DeviceInfo,
    Profiles,
    GetStreamURI,
}

pub struct Client {
    device_file_exists: bool,
    pub devices: Vec<Device>,
}

impl Client {
    pub async fn new() -> Self {
        #[cfg(debug_assertions)]
        pretty_env_logger::init();

        let mut result = Client {
            devices: Vec::new(),
            device_file_exists: false,
        };

        // Check if a file of saved devices exists already
        if let Ok(already_found_devices) = file_load() {
            println!(
                "[OnvifClient] Found {} devices in local file.",
                already_found_devices.len()
            );

            result.device_file_exists = true;
            result.devices = already_found_devices;
        // Otherwise, search for devices using UDP requests
        } else {
            let find_devices = Self::discover().await;
            match find_devices {
                Ok(devices) => {
                    // save discovered devices to a local file
                    if let Err(e) = file_save(&devices) {
                        eprintln!("[OnvifClient] Found devices, but error saving to file: {e}");
                    }

                    println!("[OnvifClient] Found {} devices!", &devices.len());
                    result.devices = devices;
                }
                Err(e) => eprintln!("[OnvifClient] Failed {e}"),
            }
        }

        result
    }

    /// Returns the response received when sending an ONVIF request to a
    /// device found via device discovery
    /// The response is SOAP formatted as byte array
    ///
    /// # Arguments
    ///
    /// * `camera_index` - Which device to get RTP stream URI
    ///
    /// # Examples
    ///
    /// ```
    /// let onvif_client = OnvifClient::new().await?;
    /// onvif_client.send(Messages::GetStreamURI, 0).await?;
    ///
    /// println!("RTP port for streaming video: {}", onvif_client.devices[0].port_rtp);
    /// ```
    pub fn get_stream_uri(&self, camera_index: usize) -> Result<String> {
        if self.devices.len() == 0 {
            return Err(anyhow!("[OnvifClient][get_stream_uri] No devices found"));
        }

        if camera_index >= self.devices.len() {
            return Err(anyhow!(
                "[OnvifClient][get_stream_uri] No devices for index"
            ));
        }

        match self.devices[camera_index].url_rtsp.as_ref() {
            Some(url_rtsp) => Ok(url_rtsp.to_string()),
            None => Err(anyhow!(
                "[OnvifClient][get_stream_uri] No RTSP URL for this device"
            )),
        }
    }

    /// Sends a multicast request via raw udpsocket on LAN.
    /// Request is in the form of a SOAP message.
    /// Response is also a SOAP message that will contain
    /// the xaddrs of the all the responding devices. Each xaddrs
    /// is a URI to subsequently send ONVIF messages
    ///
    /// # Examples
    ///
    /// ```
    /// let onvif_client = OnvifClient::discover().await?;
    /// ```
    pub async fn discover() -> Result<Vec<Device>> {
        // Discovery is based on ws-discovery
        // Which allows for TCP or UDP
        // We will use a raw UDP socket
        let addr_listen: Result<SocketAddr, _> = CLIENT_LISTEN_IP.parse();
        let addr_listen = match addr_listen {
            Ok(addr) => addr,
            Err(e) => panic!("[OnvifClient][Discover] Error creating listen address: {e}"),
        };

        let addr_send: Result<SocketAddr, _> = DISCOVER_URI.parse();
        let addr_send = match addr_send {
            Ok(addr) => addr,
            Err(e) => panic!("[OnvifClient][Discover] Error creating send address: {e}"),
        };

        // Bind to "0.0.0.0" by default
        // This is to receive incoming replies
        let udp_client = UdpSocket::bind(addr_listen).await?;

        // Get the XML SOAP message to broadcast
        let msg_discover = soap_msg(&Messages::Discovery);

        // Send the SOAP message over UDP
        // Use broadcast IP and Port
        let success = udp_client.send_to(msg_discover.as_ref(), addr_send).await;

        match success {
            Ok(_) => println!("[OnvifClient][Discover] Broadcasting to discover devices..."),
            Err(e) => panic!("[OnvifClient][Discover] Error attempting device discovery: {e}"),
        }

        // Get responses to broadcast message
        let mut buf = Vec::with_capacity(4096);
        let mut buf_size: usize = 0;

        let mut try_times = 0;
        let mut fail = false;

        let mut devices_found: Vec<Device> = Vec::new();
        let mut devices_check = String::new();

        // Discover devices using UDP broadcast
        'read: loop {
            try_times += 1;
            if try_times == 10 {
                // Fail if no devices found
                if devices_found.is_empty() {
                    fail = true;
                }

                break 'read;
            }

            // Send the SOAP message over UDP
            // Used default IP and Port
            let success = udp_client.send_to(msg_discover.as_ref(), addr_send).await;

            match success {
                Ok(_) => println!("[OnvifClient][Discover] Broadcasting to discover devices..."),
                Err(e) => panic!("[OnvifClient][Discover] Error attempting device discovery: {e}"),
            }

            // Wait 1 sec for a response
            if let Ok(recv) = timeout(
                Duration::from_millis(1000),
                udp_client.recv_buf_from(&mut buf),
            )
            .await
            {
                match recv {
                    Ok((size, addr)) => {
                        println!("[OnvifClient][Discover] Received response from: {addr}");

                        if !devices_check.contains(&addr.to_string()) {
                            println!("[OnvifClient][Discover] Found a new device: {addr}");
                            devices_check = format!("{devices_check}:{addr}");

                            // The SOAP response should provide an XAddrs which will be the
                            // ONVIF URL of the device that responded
                            let xaddrs = parse_soap(&buf[..size], "XAddrs", None);
                            println!("[OnvifClient][Discover] Received reply from: {xaddrs}");

                            // Save addr -> String (full ONVIF URL)
                            let onvif_url = xaddrs.parse()?;
                            let mut device = Device::new();

                            device.url_onvif = onvif_url;
                            devices_found.push(device);
                        }

                        buf.clear();
                    }
                    Err(e) => println!("[OnvifClient][Discover] Error in response {e}"),
                }
            }
        }

        if fail {
            panic!(
                "[OnvifClient][Discover] Tried {try_times} times and unable to find any devices."
            );
        }

        Ok(devices_found)
    }

    /// Returns the response received when sending an ONVIF request to a
    /// device found via device discovery
    /// The response is SOAP formatted as byte array
    ///
    /// # Arguments
    ///
    /// * `msg` - The SOAP request as Messages Enum
    /// * `device_index` - Which device to send message
    ///
    /// # Examples
    ///
    /// ```
    /// let onvif_client = OnvifClient::new().await?;
    /// let rtsp_uri = onvif_client.send(Messages::GetStreamURI, 0).await?;
    ///
    /// println!("RTP port for streaming video: {rtsp_uri}");
    /// ```
    pub async fn send(&mut self, msg: Messages, device_index: usize) -> Result<String> {
        if self.devices.len() == 0 {
            return Err(anyhow!("[OnvifClient][send] No devices available"));
        }

        let mut try_times = 0;
        let mut fail = false;
        let mut response: String = String::new();

        // Try to send the reqwest try_times (5)
        // with a 1sec timemout for each reqwest
        let soap_msg = soap_msg(&msg);
        let client = reqwest::Client::new();

        'read: loop {
            try_times += 1;
            if try_times == 5 {
                fail = true;
                break 'read;
            }

            // Create HTTP request using onvif_url
            let device_onvif = self.devices[device_index].url_onvif.clone();
            let request: RequestBuilder = client
                .post(device_onvif)
                .header("Content-Type", "application/soap+xml; charset=utf-8")
                .body(soap_msg.clone());

            // Send the HTTP request and receive the response
            match timeout(Duration::from_secs(1), request.send()).await {
                Ok(resp) => {
                    response = resp?.text().await?;
                    break 'read;
                }
                Err(_) => println!("[Discover][send] Error waiting for response, trying again..."),
            };
        }

        if fail {
            panic!("[Discover][send] Tried {try_times} to send {msg:?}");
        } else {
            #[cfg(debug_assertions)]
            debug!("SOAP reply for {msg:?}: {response:?}");
        }

        // Parse SOAP response from HTTP request
        // Depending on method type, parse for
        // certain values only
        let response = response.as_bytes();
        let result = match msg {
            // UDP broadcast to discover devices
            Messages::Discovery => String::new(),
            // Get the Image service URL used to get still images directly from device
            #[rustfmt::skip]
            Messages::Capabilities => {
                let media_service         = parse_soap(response, "XAddr", Some("Media"));
                let event_service         = parse_soap(response, "XAddr", Some("Events"));
                let analytics_service     = parse_soap(response, "XAddr", Some("Analytics"));
                let ptz_service           = parse_soap(response, "XAddr", Some("PTZ"));
                let image_service         = parse_soap(response, "XAddr", Some("Imaging"));

                let this_device                 = &mut self.devices[device_index];
                this_device.service_media       = Some(media_service);
                this_device.service_event       = Some(event_service);
                this_device.service_analytics   = Some(analytics_service);
                this_device.service_ptz         = Some(ptz_service);
                this_device.service_image       = Some(image_service.clone());

                #[cfg(debug_assertions)]
                info!("Imaging service: {:?}", this_device.service_image);

                image_service
            }
            #[rustfmt::skip]
            Messages::DeviceInfo => {
                let firmware_version     = parse_soap(response, "FirmwareVersion", None);
                let serial_number        = parse_soap(response, "SerialNumber", None);
                let hardware_id          = parse_soap(response, "HardwareId", None);
                let model                = parse_soap(response, "Model", None);
                let manufacturer         = parse_soap(response, "Manufacturer", None);

                let this_device                 = &mut self.devices[device_index];
                this_device.firmware_version    = Some(firmware_version);
                this_device.serial_number       = Some(serial_number);
                this_device.hardware_id         = Some(hardware_id);
                this_device.model               = Some(model);
                this_device.manufacturer        = Some(manufacturer.clone());

                #[cfg(debug_assertions)]
                info!("Manufacturer: {:?}", this_device.manufacturer);

                #[cfg(debug_assertions)]
                info!("Model: {:?}", this_device.model);

                manufacturer
            }
            #[rustfmt::skip]
            Messages::Profiles => {
                let width             = parse_soap(response, "Width", None);
                let height            = parse_soap(response, "Height", None);
                let video_codec       = parse_soap(response, "Encoding", Some("VideoEncoderConfiguration"));
                let audio_codec       = parse_soap(response, "Encoding", Some("AudioEncoderConfiguration"));
                let h264_profile      = parse_soap(response, "H264Profile", None);

                let this_device             = &mut self.devices[device_index];
                this_device.video_dim       = Some((width.parse().unwrap(), height.parse().unwrap()));
                this_device.audio_codec     = Some(audio_codec);
                this_device.h264_profile    = Some(h264_profile);
                this_device.video_codec     = Some(video_codec.clone());

                #[cfg(debug_assertions)]
                info!(
                    "Video dimensions: {} x {}",
                    this_device.video_dim.unwrap().0,
                    this_device.video_dim.unwrap().1
                );

                #[cfg(debug_assertions)]
                info!("Video Codec: {video_codec}");
                
                #[cfg(debug_assertions)]
                info!("Audio Codec: {:?}", this_device.audio_codec);

                #[cfg(debug_assertions)]
                info!("H264 Profile: {:?}", this_device.h264_profile);

                video_codec
            }
            // Get the RTSP URI from the device
            #[rustfmt::skip]
            Messages::GetStreamURI => {
                let invalid_after_connect     = parse_soap(response, "InvalidAfterConnect", None);
                let timeout                   = parse_soap(response, "Timeout", None);
                let url_string                = parse_soap(response, "Uri", None);
                let url                       = url_string.parse()?;

                let this_device                     = &mut self.devices[device_index];
                this_device.url_rtsp                = Some(url);
                this_device.invalid_after_connect   = Some(invalid_after_connect);
                this_device.timeout                 = Some(timeout);

                #[cfg(debug_assertions)]
                debug!("RTSP URI: {url_string}");

                let _ = file_save(&self.devices)?;
                url_string
            }
        };

        Ok(result)
    }
}

// Save the IP address to a file
// That way, discovery via UDP broadcast can be skipped
// File Format:
// RTSP: URL for device streaming ONVIF: URL for Onvif commands

fn file_save(devices: &Vec<Device>) -> Result<()> {
    if devices.len() == 0 {
        return Err(anyhow!(
            "[OnvifClient][file_save] Provided empty list of devices"
        ));
    }

    let path = Path::new(FILE_FOUND_DEVICES);
    let display = path.display();

    // Open a file in write-only mode, returns `io::Result<File>`
    let mut file = match File::create(&path) {
        Ok(file) => file,
        Err(why) => panic!(
            "[OnvifClient][file_save] couldn't create {}: {}",
            display, why
        ),
    };

    let mut contents = String::new();
    for device in devices {
        let url_rtsp = match device.url_rtsp.as_ref() {
            Some(url) => url.to_string(),
            None => String::new(),
        };

        let device_line = format!("IP: {} ONVIF: {}", url_rtsp, device.url_onvif);
        contents = format!("{contents}{device_line}\n");
    }

    file.write_all(contents.as_bytes())?;

    Ok(())
}

fn file_load() -> Result<Vec<Device>> {
    let open = Path::new(FILE_FOUND_DEVICES);
    let path = open.display();
    let mut contents_str = String::new();

    // Open a file in read-only mode, returns `io::Result<File>`
    let mut file = File::open(&open)?;
    let contents_size = file.read_to_string(&mut contents_str)?;

    if contents_size == 0 {
        return Err(anyhow!(
            "[OnvifClient][file_check] File found at {path}, but empty"
        ));
    }
    if !contents_str.contains("IP") {
        return Err(anyhow!(
            "[OnvifClient][file_check] File found at {path}, but no devices"
        ));
    }

    let vec_devices: Vec<Device> = contents_str
        .lines()
        .map(|line| line.split(' ').collect::<Vec<&str>>())
        .map(|line| {
            line.iter()
                .enumerate()
                .filter(|(i, _)| i % 2 == 1)
                .map(|(_, val)| *val)
                .collect::<Vec<&str>>()
        })
        .map(|vals| {
            let url_rtsp = match vals[0].is_empty() {
                true => None,
                false => Some(
                    vals[0]
                        .parse()
                        .expect("[OnvifClient][file_check] Parse error on IP"),
                ),
            };

            let mut device = Device::new();
            device.url_rtsp = url_rtsp;
            device.url_onvif = vals[1]
                .parse()
                .expect("[OnvifClient][file_check] Parse error on onvif url");

            device
        })
        .collect();

    if vec_devices.len() == 0 {
        return Err(anyhow!(
            "[OnvifClient][file_check] Error parsing devices at {path}."
        ));
    }

    Ok(vec_devices)
}

fn parse_soap(response: &[u8], element_to_find: &str, parent: Option<&str>) -> String {
    let mut element_found = false;
    let mut result = String::new();

    let buffer = BufReader::new(response);
    let parser = EventReader::new(buffer);

    let mut parent_found = match parent {
        Some(_) => false,
        None => true,
    };

    for e in parser {
        match e {
            Ok(XmlEvent::StartElement { name, .. }) => {
                let element = name.local_name;

                if !parent_found && element == parent.unwrap() {
                    parent_found = true;
                }
                if parent_found && element == element_to_find {
                    element_found = true;
                }
            }
            Ok(XmlEvent::EndElement { name: _, .. }) => {
                // if element_start && name.local_name == element_to_find {
                //     element_start = false;
                //     element_found = true;
                // }
            }
            Ok(XmlEvent::Characters(chars)) => {
                if element_found {
                    result = chars;
                    break;
                }
            }
            Err(e) => {
                eprintln!("Error: {e}");
                break;
            }
            // There's more: https://docs.rs/xml-rs/latest/xml/reader/enum.XmlEvent.html
            _ => {}
        }
    }

    result
}

fn soap_msg(msg_type: &Messages) -> String {
    let prefix = r#"<Envelope xmlns="http://www.w3.org/2003/05/soap-envelope"
                         xmlns:tds="http://www.onvif.org/ver10/device/wsdl">
                 <Body>"#;

    let suffix = "</Body></Envelope><Header/>";

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
            </e:Envelope>"#,
        ),
        Messages::Capabilities => format!(
            "
            {prefix}
            <tds:GetCapabilities>
            <tds:Category>All</tds:Category>
            </tds:GetCapabilities>
            {suffix}
        "
        ),
        Messages::DeviceInfo => format!(
            "
            {prefix}
            <tds:GetDeviceInformation/>
            {suffix}
        "
        ),
        Messages::Profiles => format!(
            "
            {prefix}
            <trt:GetProfiles/>
            {suffix}
        "
        ),
        Messages::GetStreamURI => {
            let stream = r#"<trt:GetStreamUri>
                   <trt:StreamSetup>
                       <tt:Stream>RTP-multicast</tt:Stream>
                       <tt:Transport>
                           <tt:Protocol>RTSP</tt:Protocol>
                       </tt:Transport>
                   </trt:StreamSetup>
               </trt:GetStreamUri>"#;

            format!("{prefix}{stream}{suffix}")
        }
    }
}
