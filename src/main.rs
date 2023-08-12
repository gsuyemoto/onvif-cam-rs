use anyhow::Result;
use bytes::Bytes;
use reqwest::{Client, RequestBuilder};
use std::io::BufReader;
use std::net::{SocketAddr, UdpSocket};
use xml::reader::{EventReader, XmlEvent};

#[derive(Debug)]
enum Messages {
    Discovery,
    Capabilities,
    DeviceInfo,
}

const BIND_TO_ANY_IP: &'static str = "0.0.0.0:0";

#[tokio::main]
async fn main() -> Result<()> {
    let socket = UdpSocket::bind(BIND_TO_ANY_IP)?;

    //------------------- DISCOVER ALL ONVIF DEVICES
    //-------------------

    // discovery is using SOAP UDP based in ws-discovery
    // and therefore just uses a raw udpsocket
    let soap_message = get_message(Messages::Discovery);

    // Don't make these const as they are used only once
    // and therefore after use we can drop from memory
    let send_ip = "239.255.255.250";
    let send_port = 3702;

    let (socket_buffer, size_msg) = discover_devices(&socket, send_ip, send_port, soap_message);

    // main thing we need here is the xaddrs
    // which is an HTTP URL to which we call later
    // for "device management"
    let xaddrs = parse_xaddrs(socket_buffer, size_msg);

    println!("XAddrs: {xaddrs}");

    //------------------- GET DEVICE CAPABILITIES
    //-------------------

    let soap_message = get_message(Messages::DeviceInfo);

    // after discovery, the xaddrs in the reply from each device
    // will reveal the url needed for device management
    // here the communication switches to requests sent via
    // HTTP, but still using SOAP
    // we are going to use reqwest to create HTTP requests
    let response_bytes = onvif_message(xaddrs, soap_message).await?;
    let response = String::from_utf8_lossy(response_bytes.as_ref());

    println!("Received response: {}", response);

    Ok(())
}

async fn onvif_message(device_url: String, soap_msg: String) -> Result<Bytes> {
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

fn discover_devices(
    socket: &UdpSocket,
    send_ip: &str,
    send_port: u16,
    message: String,
) -> ([u8; 4096], usize) {
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
    let mut socket_buffer = [0; 4096];
    let success = socket.recv_from(&mut socket_buffer);
    let mut size_msg: usize = 0;

    match success {
        Ok((size, _)) => {
            size_msg = size;
            println!("Successfully received message of size {size}")
        }
        Err(e) => eprintln!("Error receiving {e}"),
    }

    (socket_buffer, size_msg)
}

fn parse_xaddrs(socket_buffer: [u8; 4096], size_msg: usize) -> String {
    // get XAddrs
    let mut xaddrs = String::new();
    let mut xaddrs_start = false;

    let buffer = BufReader::new(&socket_buffer[..size_msg]);
    let parser = EventReader::new(buffer);

    for e in parser {
        match e {
            Ok(XmlEvent::StartElement { name, .. }) => {
                if name.local_name == "XAddrs".to_owned() {
                    xaddrs_start = true
                }
            }
            Ok(XmlEvent::EndElement { name, .. }) => {
                if name.local_name == "XAddrs".to_owned() {
                    xaddrs_start = false
                }
            }
            Ok(XmlEvent::Characters(chars)) => {
                if xaddrs_start {
                    xaddrs = chars;
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

    xaddrs
}

fn get_message(msg_type: Messages) -> String {
    match msg_type {
        Messages::Discovery => format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
            <s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope"
                        xmlns:a="http://schemas.xmlsoap.org/ws/2004/08/addressing"
                        xmlns:d="http://schemas.xmlsoap.org/ws/2005/04/discovery"
                        xmlns:dn="http://www.onvif.org/ver10/network/wsdl">
                <s:Header>
                    <a:Action d:mustUnderstand="1">http://schemas.xmlsoap.org/ws/2005/04/discovery/Probe</a:Action>
                    <a:MessageID>uuid:72d76f2a-23d5-4181-9ea2-1ade1ca198b9</a:MessageID>
                    <a:ReplyTo>
                        <a:Address>http://schemas.xmlsoap.org/ws/2004/08/addressing/role/anonymous</a:Address>
                    </a:ReplyTo>
                    <a:To>urn:schemas-xmlsoap-org:ws:2005:04:discovery</a:To>
                </s:Header>
                <s:Body>
                    <d:Probe>
                        <d:Types>dn:NetworkVideoTransmitter</d:Types>
                    </d:Probe>
                </s:Body>
            </s:Envelope>"#
        ),

        // Messages::Capabilities => format!(
        //     r#"<Envelope xmlns="http://www.w3.org/2003/05/soap-envelope"
        //                  xmlns:tds="http://www.onvif.org/ver10/device/wsdl">
        //        <Header/>
        //        <Body>
        //            <tds:GetCapabilities>
        //                <tds:Category>All</tds:Category>
        //            </tds:GetCapabilities>
        //        </Body>
        //     </Envelope>"#
        // ),
        Messages::Capabilities => format!(
            r#"
            <Envelope xmlns="http://www.w3.org/2003/05/soap-envelope" xmlns:trt="http://www.onvif.org/ver10/media/wsdl">
                <Header/>
                <Body>
                    <trt:GetCapabilities>
                        <trt:Category>Media</trt:Category>
                    </trt:GetCapabilities>
                </Body>
            </Envelope>
        "#
        ),
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
    }
}
