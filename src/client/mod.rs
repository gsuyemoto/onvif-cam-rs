use crate::builder::Device;
use crate::utils::parse_soap;

use anyhow::Result;
use async_trait::async_trait;
use log::debug;
use reqwest::{RequestBuilder, Response};
use std::{net::SocketAddr, time::Duration};
use tokio::{net::UdpSocket, time::timeout};
use uuid::Uuid;

const DISCOVER_URI: &'static str = "239.255.255.250:3702";
const CLIENT_LISTEN_IP: &'static str = "0.0.0.0:0"; // notice port is 0

/// All of the ONVIF requests that this program plans to support
#[derive(Debug)]
pub enum Messages {
    Discovery,
    Capabilities,
    DeviceInfo,
    Profiles,
    GetStreamURI,
    GetServices, // a summarized version of Capabilities
    GetServiceCapabilities,
    GetDNS,
}

#[async_trait]
pub trait Client {
    /// Sends a multicast request via raw udpsocket on LAN.
    /// Request is in the form of a SOAP message.
    /// Response is also a SOAP message that will contain
    /// the xaddrs of the all the responding devices. Each xaddrs
    /// is a URI to subsequently send ONVIF messages
    ///
    /// # Examples
    ///
    /// ```
    /// // This enumerates a Vec<Device>
    /// let devices = Client::discover().await?;
    /// ```
    async fn discover<T: Device>() -> Result<Vec<T>> {
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
        let uuid = Uuid::new_v4();
        let msg_discover = soap_msg(&Messages::Discovery, uuid);

        // Get responses to broadcast message
        let mut devices_found: Vec<T> = Vec::new();
        let mut devices_check = String::new();
        let mut try_send = 0;

        while try_send < 2 {
            let mut try_recv = 0;
            try_send += 1;

            // Send the SOAP message over UDP
            // Used default IP and Port
            let success = udp_client.send_to(msg_discover.as_ref(), addr_send).await;

            match success {
                Ok(_) => println!("[OnvifClient][Discover] Broadcasting to discover devices..."),
                Err(e) => {
                    eprintln!("[OnvifClient][Discover] Error attempting device discovery: {e}")
                }
            }

            while try_recv < 5 {
                try_recv += 1;
                let mut buf = Vec::with_capacity(4096);

                // Wait 1 sec for a response
                if let Ok(recv) = timeout(
                    Duration::from_millis(2000),
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
                                let xaddrs = parse_soap(&buf[..size], "XAddrs", None, true);

                                println!("[OnvifClient][Discover] Received reply from: {addr}");
                                println!("[OnvifClient][Discover] Size of response: {size}");

                                // Save addr -> String (full ONVIF URL)
                                let onvif_url = xaddrs[0].parse()?;
                                let mut device = T::new();

                                device.set_onvif_url(onvif_url);
                                devices_found.push(device);
                            }
                        }
                        Err(e) => eprintln!("[OnvifClient][Discover] Error in response {e}"),
                    }
                }
            }
        }

        if devices_found.is_empty() {
            panic!("[OnvifClient][Discover] Unable to find any devices.");
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
    async fn send(url: url::Url, msg: Messages) -> Result<Response> {
        let uuid = Uuid::new_v4();
        let mut try_times = 0;
        let mut fail = false;
        let response: Response;

        // Try to send the reqwest try_times (5)
        // with a 1sec timemout for each reqwest
        let soap_msg = soap_msg(&msg, uuid);
        let client = reqwest::Client::new();

        'read: loop {
            try_times += 1;

            if try_times == 5 {
                fail = true;
                break 'read;
            }

            // Create HTTP request using onvif_url
            let request: RequestBuilder = client
                .post(url)
                .header("Content-Type", "application/soap+xml; charset=utf-8")
                .body(soap_msg.clone());

            // Send the HTTP request and receive the response
            match timeout(Duration::from_secs(1), request.send()).await {
                Ok(resp) => {
                    debug!("SOAP reply for {msg:?}: {}", response.text().await?);
                    response = resp?;
                    break 'read;
                }
                Err(_) => println!("[Discover][send] Error waiting for response, trying again..."),
            };
        }

        if fail {
            panic!("[Discover][send] Tried {try_times} to send {msg:?}");
        }

        Ok(response)
    }
}

fn soap_msg(msg_type: &Messages, uuid: Uuid) -> String {
    let prefix = r#"<Envelope xmlns="http://www.w3.org/2003/05/soap-envelope"
                         xmlns:tds="http://www.onvif.org/ver10/device/wsdl">
                 <Body>"#;

    let prefix_discovery = r#"<?xml version="1.0" encoding="UTF-8"?>
                        <e:Envelope xmlns:e="http://www.w3.org/2003/05/soap-envelope"
                        xmlns:w="http://schemas.xmlsoap.org/ws/2004/08/addressing"
                        xmlns:d="http://schemas.xmlsoap.org/ws/2005/04/discovery"
                        xmlns:dn="http://www.onvif.org/ver10/network/wsdl">"#;

    // Insert UUID in the MessageID here
    let header_pt1 = format!("<e:Header><w:MessageID>uuid:{uuid}</w:MessageID>");
    let header_pt2 = r#"<w:To>urn:schemas-xmlsoap-org:ws:2005:04:discovery</w:To>
                     <w:Action>http://schemas.xmlsoap.org/ws/2005/04/discovery/Probe</w:Action>
                     </e:Header>"#;

    let suffix = "</Body></Envelope><Header/>";
    let suffix_discovery = r#"<e:Body>
                                   <d:Probe>
                                       <d:Types>dn:NetworkVideoTransmitter</d:Types>
                                   </d:Probe>
                               </e:Body>
                           </e:Envelope>"#;

    let stream = r#"<trt:GetStreamUri>
           <trt:StreamSetup>
               <tt:Stream>RTP-multicast</tt:Stream>
               <tt:Transport>
                   <tt:Protocol>RTSP</tt:Protocol>
               </tt:Transport>
           </trt:StreamSetup>
       </trt:GetStreamUri>"#;

    match msg_type {
        Messages::Discovery => format!(
            "
                {prefix_discovery}
                {header_pt1}
                {header_pt2}
                {suffix_discovery}
            "
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
        Messages::GetStreamURI => format!(
            "
                {prefix}
                {stream}
                {suffix}
            "
        ),
        Messages::GetServices => format!(
            "
                {prefix}
                <tds:GetServices>
                <tds:IncludeCapability>true</tds:IncludeCapability>
                </tds:GetServices>
                {suffix}
            "
        ),
        Messages::GetServiceCapabilities => format!(
            "
                {prefix}
                <tds:GetServiceCapabilities/>
                {suffix}
            "
        ),
        Messages::GetDNS => format!(
            "
                {prefix}
                <tds:GetDNS/>
                {suffix}
            "
        ),
    }
}
