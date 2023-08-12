use std::io::BufReader;
use std::net::{SocketAddr, UdpSocket};
use xml::reader::{EventReader, XmlEvent};

#[derive(Debug)]
enum Messages {
    Discovery,
    Capabilities,
}

const BIND_TO_ANY_IP: &'static str = "0.0.0.0:0";

fn main() -> std::io::Result<()> {
    let socket = UdpSocket::bind(BIND_TO_ANY_IP)?;

    //------------------- DISCOVER ALL ONVIF DEVICES
    //-------------------

    let message = get_message(Messages::Discovery);

    // Don't make these const as they are used only once
    let multicast_ip = "239.255.255.250";
    let multicast_port = 3702;

    let (socket_buffer, size_msg) =
        send_and_get_response(&socket, multicast_ip, multicast_port, message);

    let xaddrs = parse_xaddrs(socket_buffer, size_msg);

    println!("XAddrs: {xaddrs}");

    //------------------- GET DEVICE CAPABILITIES
    //-------------------

    Ok(())
}

fn send_and_get_response(
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

    // show raw XML message for testing which includes inner
    // let response = String::from_utf8_lossy(&socket_buffer[..size]);
    // println!("Received response: {}", response);

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
    }
}

fn get_capabilities() {
    // // Send the SOAP message over UDP
    // socket.send_to(message_capabilities.as_bytes(), destination)?;

    // println!("Sent capabilties message");

    // // Receive response
    // let mut socket_buffer = [0; 4096];
    // let (size, _) = socket.recv_from(&mut socket_buffer)?;

    // println!("Received message: {size}");

    // let mut depth = 0;
    // let buffer = BufReader::new(&socket_buffer[..size]);
    // let parser = EventReader::new(buffer);

    // for e in parser {
    //     match e {
    //         Ok(XmlEvent::StartElement { name, .. }) => {
    //             // println!("{:spaces$}+{name}", "", spaces = depth * 2);
    //             depth += 1;
    //         }
    //         Ok(XmlEvent::EndElement { name }) => {
    //             depth -= 1;
    //             // println!("{:spaces$}-{name}", "", spaces = depth * 2);
    //         }
    //         Ok(XmlEvent::Characters(chars)) => {
    //             println!("{chars}");
    //         }
    //         Err(e) => {
    //             eprintln!("Error: {e}");
    //             break;
    //         }
    //         // There's more: https://docs.rs/xml-rs/latest/xml/reader/enum.XmlEvent.html
    //         _ => {}
    //     }
    // }
}
