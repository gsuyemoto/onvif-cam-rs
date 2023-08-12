use std::io::BufReader;
use std::net::{SocketAddr, UdpSocket};
use xml::reader::{EventReader, XmlEvent};

fn main() -> std::io::Result<()> {
    let socket = UdpSocket::bind("0.0.0.0:0")?; // Bind to any available local IP

    // SOAP message or discovery probe to send
    let message_discovery = format!(
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
    );

    let message_capabilities = format!(
        r#"<Envelope xmlns="http://www.w3.org/2003/05/soap-envelope" xmlns:tds="http://www.onvif.org/ver10/device/wsdl">
               <Header/>
               <Body>
                   <tds:GetCapabilities>
                       <tds:Category>All</tds:Category>
                   </tds:GetCapabilities>
               </Body>
           </Envelope>"#
    );

    // Multicast address and port for WS-Discovery
    let multicast_ip = "239.255.255.250";
    let port = 3702;

    // Destination address
    let destination: SocketAddr = format!("{}:{}", multicast_ip, port).parse().unwrap();

    //------------------- DISCOVER ALL ONVIF DEVICES
    //-------------------

    // Send the SOAP message over UDP
    socket.send_to(message_discovery.as_bytes(), destination)?;

    println!("Sent discovery message");

    // Receive response
    let mut socket_buffer = [0; 4096];
    let (size, _) = socket.recv_from(&mut socket_buffer)?;

    println!("Received message: {size}");

    // show raw XML message for testing which includes inner
    // let response = String::from_utf8_lossy(&socket_buffer[..size]);
    // println!("Received response: {}", response);

    let mut depth = 0;
    let buffer = BufReader::new(&socket_buffer[..size]);
    let parser = EventReader::new(buffer);

    // get XAddrs
    let mut xaddrs = String::new();
    let mut xaddrs_start = false;

    for e in parser {
        match e {
            Ok(XmlEvent::StartElement { name, .. }) => {
                depth += 1;
                if name.local_name == "XAddrs".to_owned() {
                    xaddrs_start = true
                }
            }
            Ok(XmlEvent::EndElement { name, .. }) => {
                depth += 1;
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

    println!("XAddrs: {xaddrs}");

    //------------------- GET DEVICE CAPABILITIES
    //-------------------

    Ok(())
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
