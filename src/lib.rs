use anyhow::{anyhow, Result};
use reqwest::{Client, RequestBuilder};
use std::{io::BufReader, net::SocketAddr, time::Duration};
use tokio::{io::ErrorKind, net::UdpSocket, time::timeout};
use url::Url;
use xml::reader::{EventReader, XmlEvent};
//------ Saving File
use std::fmt;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

const DISCOVER_URI: &'static str = "239.255.255.250:3702";
const CLIENT_LISTEN_IP: &'static str = "0.0.0.0:0"; // notice port is 0
const FILE_FOUND_DEVICES: &'static str = "devices_found.txt";

/// All of the ONVIF requests that this program supports
#[derive(Debug)]
pub enum Messages {
    Discovery,
    Capabilities,
    DeviceInfo,
    Profiles,
    GetStreamURI,
}

struct Device {
    rtsp_uri: Option<String>,
    mac_addr: Option<String>,
    onvif_url: Option<String>,
}

pub struct OnvifClient {
    devices: Vec<Device>,
}

impl OnvifClient {
    pub fn new() -> Self {
        OnvifClient {
            devices: Vec::new(),
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
    /// let onvif_client = OnvifClient::new().discover().await?;
    /// ```
    pub async fn discover(mut self) -> Result<Self> {
        // Discovery is based on ws-discovery
        // Which allows for TCP or UDP
        // We will use a raw UDP socket
        let addr_listen: Result<SocketAddr, _> = CLIENT_LISTEN_IP.parse();
        let addr_listen = match addr_listen {
            Ok(addr) => addr,
            Err(e) => panic!("[Discover] Error creating listen address: {e}"),
        };

        let addr_send: Result<SocketAddr, _> = DISCOVER_URI.parse();
        let addr_send = match addr_send {
            Ok(addr) => addr,
            Err(e) => panic!("[Discover] Error creating send address: {e}"),
        };

        // Bind to "0.0.0.0" by default
        // This is to receive incoming replies
        let udp_client = UdpSocket::bind(addr_listen).await?;

        // Get the XML SOAP message to broadcast
        let msg_discover = soap_msg(&Messages::Discovery);

        // Send the SOAP message over UDP
        // Used default IP and Port
        let success = udp_client.send_to(msg_discover.as_ref(), addr_send).await;

        match success {
            Ok(_) => println!("[Discover] Broadcasting to discover devices..."),
            Err(e) => panic!("[Discover] Error attempting device discovery: {e}"),
        }

        // Get responses to broadcast message
        let mut buf = Vec::with_capacity(4096);
        let mut buf_size: usize = 0;

        let mut try_times = 0;
        let mut fail = false;

        'read: loop {
            try_times += 1;
            if try_times == 5 {
                fail = true;
                break 'read;
            }

            // Wait for the socket to be readable
            if let Err(_) = timeout(Duration::from_millis(1000), udp_client.readable()).await {
                // Send the SOAP message over UDP
                // Used default IP and Port
                let success = udp_client.send_to(msg_discover.as_ref(), addr_send).await;

                match success {
                    Ok(_) => println!("[Discover] Broadcasting to discover devices..."),
                    Err(e) => panic!("[Discover] Error attempting device discovery: {e}"),
                }

                continue;
            }

            // Try to read data, this may still fail with `WouldBlock`
            // if the readiness event is a false positive.
            match udp_client.try_recv_buf_from(&mut buf) {
                Ok((size, addr)) => {
                    println!("[Discover] Received response from: {addr}");
                    buf_size = size;
                    break 'read;
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                    continue;
                }
                Err(e) => {
                    return Err(e.into());
                }
            }
        }

        if fail {
            panic!("[Discover] Tried {try_times} times and unable to find any devices.");
        }

        // The SOAP response should provide an XAddrs which will be the
        // IP address of the device that responded
        let xaddrs = parse_soap(&buf[..buf_size], Some("XAddrs"));
        println!("[Discover] Found xaddrs: {xaddrs}");

        // let onvif_url = Url::parse(&xaddrs);
        // let onvif_url = match onvif_url {
        //     Ok(url) => url,
        //     Err(e) => panic!("[Discover] Error creating Url object from xaddrs: {e}"),
        // };

        self.devices.push(Device {
            onvif_url: Some(xaddrs),
            mac_addr: None,
            rtsp_uri: None,
        });

        Ok(self)
    }

    /// Returns the response received when sending an ONVIF request to a
    /// device found via device discovery
    /// The response is SOAP formatted as byte array
    ///
    /// # Arguments
    ///
    /// * `msg` - The SOAP request as Messages Enum
    ///
    /// # Examples
    ///
    /// ```
    /// let onvif_client = OnvifClient::new().discover().await?;
    /// let streaming_uri = onvif_client.send(Messages::GetStreamURI).await?;
    /// println!("uri: {streaming_uri}");
    /// ```
    pub async fn send(&mut self, msg: Messages) -> Result<String> {
        // After discovery, communication with device via ONVIF
        // will switch to HTTP and use the following url:
        // http://ip.address/onvif/device_service
        if self.devices.len() == 0 {
            return Err(anyhow!("No devices available"));
        }

        let mut try_times = 0;
        let mut fail = false;
        let mut response: String = String::new();

        // Try to send the reqwest try_times (5)
        // with a 1sec timemout for each reqwest
        let soap_msg = soap_msg(&msg);
        let client = Client::new();

        'read: loop {
            try_times += 1;
            if try_times == 5 {
                fail = true;
                break 'read;
            }

            let device_url = self.devices[0].onvif_url.as_deref().unwrap();
            let device_url: Url = Url::parse(device_url)?;
            let request: RequestBuilder = client
                .post(device_url)
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
            panic!("[Discover][send] Tried {try_times} to send {:?}", msg);
        }

        let parsed = match msg {
            Messages::Discovery => panic!("Not implemented."),
            Messages::Capabilities => panic!("Not implemented."),
            Messages::DeviceInfo => panic!("Not implemented."),
            Messages::Profiles => panic!("Not implemented."),
            Messages::GetStreamURI => {
                let uri = parse_soap(response.as_bytes(), Some("Uri"));
                self.devices[0].rtsp_uri = Some(uri.clone());

                // TODO: Check if device IP is already in saved file and if not, save it

                uri
            }
        };

        Ok(parsed)
    }
}

// Save the IP address to a file
// That way, discovery via UDP broadcast can be skipped
// File Format:
// IP: ip_addr_device MAC: mac_addr_device COMPANY: device maker

fn file_save(contents: &[u8]) {
    let path = Path::new(FILE_FOUND_DEVICES);
    let display = path.display();

    // Open a file in write-only mode, returns `io::Result<File>`
    let mut file = match File::create(&path) {
        Ok(file) => file,
        Err(why) => panic!("couldn't create {}: {}", display, why),
    };

    match file.write_all(contents) {
        Ok(_) => println!("successfully wrote to {}", display),
        Err(why) => panic!("couldn't write to {}: {}", display, why),
    }
}

fn file_check() -> Result<Vec<Device>> {
    let open = Path::new(FILE_FOUND_DEVICES);
    let path = open.display();
    let mut contents_str = String::new();

    // Open a file in read-only mode, returns `io::Result<File>`
    let mut file = File::open(&open)?;
    let contents_size = file.read_to_string(&mut contents_str)?;

    if contents_size == 0 {
        return Err(anyhow!("File found, but empty"));
    }
    if !contents_str.contains("IP") {
        return Err(anyhow!("File found, but no devices"));
    }

    let vec_devices = contents_str
        .lines()
        .collect::<Vec<&str>>()
        .iter()
        .map(|line| line.split(' ').collect::<Vec<&str>>())
        .map(|line| {
            line.iter()
                .enumerate()
                .filter(|(i, _)| i % 2 == 0)
                .map(|(_, val)| *val)
                .collect::<Vec<&str>>()
        })
        .map(|vals| Device {
            rtsp_uri: Some(vals[0].to_string()),
            mac_addr: Some(vals[1].to_string()),
            onvif_url: None,
        })
        .collect();

    Ok(vec_devices)
}

fn parse_soap(response: &[u8], find: Option<&str>) -> String {
    let mut element_found = String::new();
    let mut element_start = false;

    let buffer = BufReader::new(response);
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

fn soap_msg(msg_type: &Messages) -> String {
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
            r#"<Envelope xmlns="http://www.w3.org/2003/05/soap-envelope"
                         xmlns:tds="http://www.onvif.org/ver10/device/wsdl">
               <Header/>
               <Body>
                   <tds:GetCapabilities>
                       <tds:Category>All</tds:Category>
                   </tds:GetCapabilities>
               </Body>
            </Envelope>"#,
        ),
        Messages::DeviceInfo => format!(
            r#"
            <s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope"
                         xmlns:tds="http://www.onvif.org/ver10/device/wsdl">
                <s:Body>
                    <tds:GetDeviceInformation/>
                </s:Body>
            </s:Envelope>
        "#,
        ),
        Messages::Profiles => format!(
            r#"
            <s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope"
                         xmlns:tds="http://www.onvif.org/ver10/device/wsdl">
                <soap:Body>
                    <trt:GetProfiles/>
                </soap:Body>
            </s:Envelope>
        "#,
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
        "#,
        ),
    }
}
