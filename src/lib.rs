use anyhow::Result;
use reqwest::{Client, RequestBuilder};
use std::io::BufReader;
use std::net::SocketAddr;
use tokio::net::UdpSocket;
use xml::reader::{EventReader, XmlEvent};

const DISCOVER_URI: &'static str = "239.255.255.250:3702";
const CLIENT_LISTEN_IP: &'static str = "0.0.0.0:0"; // notice port is 0

/// All of the ONVIF requests that this program supports
#[derive(Debug)]
pub enum Messages {
    Discovery,
    Capabilities,
    DeviceInfo,
    Profiles,
    GetStreamURI,
}

pub struct OnvifClient {
    addrs_devices: Option<Vec<SocketAddr>>,
}

impl OnvifClient {
    pub fn new() -> Self {
        OnvifClient {
            addrs_devices: None,
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
            Ok(_) => println!("[Discover] Broadcasting discover devices..."),
            Err(e) => panic!("[Discover] Error attempting device discovery: {e}"),
        }

        // Get responses to broadcast message
        let mut resp_buf = vec![];
        let success = udp_client.recv_from(resp_buf.as_mut_slice()).await;

        match success {
            Ok((_, socket_addr)) => println!("[Discover] Received response from: {socket_addr}"),
            Err(e) => eprintln!("[Discover] Error receiving messages {e}"),
        }

        // The SOAP response should provide an XAddrs which will be the
        // IP address of the device that responded
        let xaddrs = parse_soap(resp_buf.as_slice(), Some("XAddrs"))?;
        let addr_device: SocketAddr = xaddrs.parse()?;

        self.addrs_devices = Some(vec![addr_device]);
        println!("Found xaddrs: {xaddrs}");

        Ok(self)
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
    /// let onvif_client = OnvifClient::new().discover().await?;
    /// let streaming_uri = onvif_client.send(Messages::GetStreamURI).await?;
    /// println!("uri: {streaming_uri}");
    /// ```
    pub async fn send(&self, msg: Messages) -> Result<String> {
        // After discovery, communication with device via ONVIF
        // will switch to HTTP and use the following url:
        // http://ip.address/onvif/device_service
        let device_uri = match &self.addrs_devices {
            Some(uris) => format!("http://{}/onvif/device_service", &uris[0].is_ipv4()),
            None => panic!("[Send] No devices have been discovered!"),
        };

        let soap_msg = soap_msg(&msg);
        let client = Client::new();
        let request: RequestBuilder = client
            .post(device_uri)
            .header("Content-Type", "application/soap+xml; charset=utf-8")
            .body(soap_msg);

        // Send the HTTP request and receive the response
        let response = request.send().await?.text().await?;

        match msg {
            Messages::Discovery => panic!("Not implemented."),
            Messages::Capabilities => panic!("Not implemented."),
            Messages::DeviceInfo => panic!("Not implemented."),
            Messages::Profiles => panic!("Not implemented."),
            Messages::GetStreamURI => parse_soap(response.as_bytes(), Some("Uri")),
        }
    }
}

fn parse_soap(response: &[u8], find: Option<&str>) -> Result<String> {
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

    Ok(element_found)
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
