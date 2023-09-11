use crate::device::{parse_device_type, Device};
use crate::utils::parse_soap;

use anyhow::{anyhow, Result};
use log::trace;
use reqwest::{RequestBuilder, Response};
use std::{net::SocketAddr, time::Duration};
use tokio::{net::UdpSocket, time::timeout};
use url::Url;
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
    GetNetworkInterfaces,
    GetNetworkProtocols,
    GetNetworkDefaultGateway,
    GetDot11Capabilities,
    GetDot11Status,
    GetSystemUris,
    GetSystemLog,
    GetDiscoveryMode,
    GetGeoLocation,
    GetStorageConfigurations,
    CreatePullPointSubscriptionRequest,
    GetAnalyticsConfigurations,
    GetEventProperties,
    GetProfiles,
    GetEventBrokers,
    PullMessages,
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
/// // Find all IP Devices on local network using ONVIF
/// let mut devices = client::discover().await?;
/// let mut cameras: Vec<Camera> = Vec::new();
///
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
    let uuid = Uuid::new_v4();
    let msg_discover = soap_msg(&Messages::Discovery, uuid);

    // Get responses to broadcast message
    let mut devices_found: Vec<Device> = Vec::new();
    let mut devices_check = String::new();
    let mut try_send = 0;

    while try_send < 2 {
        let mut try_recv = 0;
        try_send += 1;

        // Send the SOAP message over UDP
        // Use default IP and Port
        let success = udp_client.send_to(msg_discover.as_ref(), addr_send).await?;

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
                            println!("[OnvifClient][Discover] Size of response: {size}");

                            // Add to list of devices already found
                            devices_check = format!("{devices_check}:{addr}");

                            // The SOAP response should provide an XAddrs which will be the
                            // ONVIF URL of the device that responded
                            let xaddrs = parse_soap(&buf[..size], "XAddrs", None, true, false);
                            let url_onvif: Url = xaddrs[0].parse()?;

                            // Get device type
                            let mut device_type =
                                parse_soap(&buf[..size], "Types", None, true, false);
                            let device_type = parse_device_type(device_type.remove(0));

                            // Get scope list
                            let scopes = parse_soap(&buf[..size], "Scopes", None, true, false);
                            let scopes = scopes[0]
                                .split(' ')
                                .map(|s| s.to_string())
                                .collect::<Vec<String>>();

                            devices_found.push(Device {
                                url_onvif,
                                device_type,
                                scopes,
                            });
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
/// * `onvif_url` - The main ONVIF service URL to the device
/// * `msg` - The SOAP request as Messages Enum
///
/// # Examples
///
/// ```
/// let mut devices = client::discover().await?;
/// let onvif_url = devices[0].base.url;
///
/// let response = client::send(onvif_url, Messages::GetStreamURI).await?;
/// let stream_url = response.remove(0);
///
/// println!("RTP port for streaming video: {stream_url}");
/// ```
pub async fn send(onvif_url: url::Url, msg: Messages) -> Result<Response> {
    let uuid = Uuid::new_v4();
    let mut try_times = 0;

    // Try to send the reqwest try_times (5)
    // with a 1sec timemout for each reqwest
    let soap_msg = soap_msg(&msg, uuid);
    let client = reqwest::Client::new();

    'read: loop {
        try_times += 1;

        if try_times == 5 {
            break 'read;
        }

        // Create HTTP request using onvif_url
        let request: RequestBuilder = client
            .post(onvif_url.clone())
            .header("Content-Type", "application/soap+xml; charset=utf-8")
            .body(soap_msg.clone());

        // Send the HTTP request and receive the response
        match timeout(Duration::from_secs(1), request.send()).await {
            Ok(resp) => {
                trace!("SOAP reply for {msg:?}: {resp:?}");
                let response = resp?;
                return Ok(response);
            }
            Err(_) => println!("[Discover][send] Error waiting for response, trying again..."),
        };
    }

    Err(anyhow!("[Client] Error getting response from message"))
}

pub fn soap_msg(msg_type: &Messages, uuid: Uuid) -> String {
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
        Messages::GetNetworkInterfaces => format!(
            "
                {prefix}
                <tds:GetNetworkInterfaces/>
                {suffix}
            "
        ),
        Messages::GetNetworkProtocols => format!(
            "
                {prefix}
                <tds:GetNetworkProtocols/>
                {suffix}
            "
        ),
        Messages::GetNetworkDefaultGateway => format!(
            "
                {prefix}
                <tds:GetNetworkDefaultGateway/>
                {suffix}
            "
        ),
        Messages::GetDot11Capabilities => format!(
            "
                {prefix}
                <tds:GetDot11Capabilities/>
                {suffix}
            "
        ),
        Messages::GetDot11Status => format!(
            "
                {prefix}
                <tds:GetDot11Status/>
                {suffix}
            "
        ),
        Messages::GetSystemUris => format!(
            "
                {prefix}
                <tds:GetSystemUris/>
                {suffix}
            "
        ),
        Messages::GetSystemLog => format!(
            "
                {prefix}
                <tds:GetSystemLog/>
                {suffix}
            "
        ),
        Messages::GetDiscoveryMode => format!(
            "
                {prefix}
                <tds:GetDiscoveryMode/>
                {suffix}
            "
        ),
        Messages::GetGeoLocation => format!(
            "
                {prefix}
                <tds:GetGeoLocation/>
                {suffix}
            "
        ),
        Messages::GetStorageConfigurations => format!(
            "
                {prefix}
                <tds:GetStorageConfigurations/>
                {suffix}
            "
        ),
        // CREATE PULL POINT WITH OPTIONAL PARAMS
        // Messages::CreatePullPointSubscriptionRequest => format!(
        //     "
        //         {prefix}
        //         <wsnt:CreatePullPointSubscription>
        //             <wsnt:Filter>
        //                 <wsnt:TopicExpression Dialect=\"http://www.onvif.org/ver10/tev/topicExpression/ConcreteSet\">
        //                     tns1:Device/tnsaxis:VMD/Camera1
        //                 </wsnt:TopicExpression>
        //                 <!-- Add more Filter elements if needed -->
        //             </wsnt:Filter>
        //             <wsnt:InitialTerminationTime>PT3600S</wsnt:InitialTerminationTime>
        //             <!-- Add more subscription parameters if needed -->
        //         </wsnt:CreatePullPointSubscription>
        //         {suffix}
        //     "
        // ),
        Messages::CreatePullPointSubscriptionRequest => format!(
            "
                {prefix}
                <tev:CreatePullPointSubscription/>
                {suffix}
            "
        ),
        Messages::GetAnalyticsConfigurations => format!(
            "
                {prefix}
                <tns:GetAnalyticsConfigurations/>
                {suffix}
            "
        ),
        Messages::GetEventProperties => format!(
            "
                {prefix}
                <tds:GetEventProperties/>
                {suffix}
            "
        ),
        Messages::GetProfiles => format!(
            "
                {prefix}
                <tr2:GetProfiles/>
                {suffix}
            "
        ),
        Messages::GetEventBrokers => format!(
            "
                {prefix}
                <tds:GetEventBrokers/>
                {suffix}
            "
        ),
        Messages::PullMessages => format!(
            "
                {prefix}
                <wsnt:PullMessages>
                    <wsnt:Timeout>PT5S</wsnt:Timeout>
                    <wsnt:MessageLimit>10</wsnt:MessageLimit>
                </wsnt:PullMessages>
                {suffix}
            "
        ),
    }
}
