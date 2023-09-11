use crate::device::{Services, Capabilities, DeviceInfo, Profiles, StreamUri, EventCapabilities, ServiceCapabilities, AnalyticsConfigList};
use crate::utils::parse_soap;
use crate::client::{self, Messages};

use log::{error, trace, debug, info};
use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait CameraBuilder {
    #[rustfmt::skip]
    async fn set_capabilities(onvif_url: url::Url) -> Result<Capabilities> {
        let response              = client::send(onvif_url, Messages::Capabilities).await?;
        let response              = response.bytes().await?;
        let mut media_service     = parse_soap(&response[..], "XAddr", Some("Media"),       true, false);
        let mut event_service     = parse_soap(&response[..], "XAddr", Some("Events"),      true, false);
        let mut analytics_service = parse_soap(&response[..], "XAddr", Some("Analytics"),   true, false);
        let mut ptz_service       = parse_soap(&response[..], "XAddr", Some("PTZ"),         true, false);
        let mut image_service     = parse_soap(&response[..], "XAddr", Some("Imaging"),     true, false);

        info!("media_service: {}", media_service[0]);
        info!("event_service: {}", event_service[0]);
        info!("analytics_service: {}", analytics_service[0]);
        info!("ptz_service: {}", ptz_service[0]);
        info!("image_service: {}", image_service[0]);

        let mut result         = Capabilities::default();
        result.url_media       = Some(media_service.remove(0).parse()?);
        result.url_events      = Some(event_service.remove(0).parse()?);
        result.url_analytics   = Some(analytics_service.remove(0).parse()?);
        result.url_ptz         = Some(ptz_service.remove(0).parse()?);
        result.url_imaging     = Some(image_service.remove(0).parse()?);

        Ok(result)
    }

    #[rustfmt::skip]
    async fn set_device_info(onvif_url: url::Url) -> Result<DeviceInfo> {
        let response                 = client::send(onvif_url, Messages::DeviceInfo).await?;
        let response                 = response.bytes().await?;
        let mut firmware_version     = parse_soap(&response[..], "FirmwareVersion",  None, true, false);
        let mut serial_number        = parse_soap(&response[..], "SerialNumber",     None, true, false);
        let mut hardware_id          = parse_soap(&response[..], "HardwareId",       None, true, false);
        let mut model                = parse_soap(&response[..], "Model",            None, true, false);
        let mut manufacturer         = parse_soap(&response[..], "Manufacturer",     None, true, false);

        info!("Manufacturer: {}", manufacturer[0]);
        info!("Model: {}", model[0]);

        let mut result             = DeviceInfo::default(); 
        result.firmware_version    = Some(firmware_version.remove(0));
        result.serial_num          = Some(serial_number.remove(0));
        result.hardware_id         = Some(hardware_id.remove(0));
        result.model               = Some(model.remove(0));
        result.manufacturer        = Some(manufacturer.remove(0));

        Ok(result)
    }

    #[rustfmt::skip]
    async fn set_profiles(onvif_url: url::Url) -> Result<Profiles> {
        let response              = client::send(onvif_url, Messages::Profiles).await?;
        let response              = response.bytes().await?;
        let width                 = parse_soap(&response[..], "Width",          None,                                 true, false);
        let height                = parse_soap(&response[..], "Height",         None,                                 true, false);
        let mut video_codec       = parse_soap(&response[..], "Encoding",       Some("VideoEncoderConfiguration"),    true, false);
        let mut audio_codec       = parse_soap(&response[..], "Encoding",       Some("AudioEncoderConfiguration"),    true, false);
        let mut h264_profile      = parse_soap(&response[..], "H264Profile",    None,                                 true, false);

        info!("Video Codec: {}", video_codec[0]);
        info!("Audio Codec: {}", audio_codec[0]);
        info!("H264 Profile: {}", h264_profile[0]);
        info!(
            "Video dimensions: {} x {}",
            width[0],
            height[0]
        );

        let mut result         = Profiles::default(); 
        result.video_dim       = Some((width[0].parse().unwrap(), height[0].parse().unwrap()));
        result.audio_codec     = Some(audio_codec.remove(0));
        result.h264_profile    = Some(h264_profile.remove(0));
        result.video_codec     = Some(video_codec.remove(0));

        Ok(result)
    }

    #[rustfmt::skip]
    async fn set_stream_uri(onvif_url: url::Url) -> Result<StreamUri> {
        let response                      = client::send(onvif_url, Messages::GetStreamURI).await?;
        let response                      = response.bytes().await?;
        let mut invalid_after_connect     = parse_soap(&response[..], "InvalidAfterConnect", None, true, false);
        let mut timeout                   = parse_soap(&response[..], "Timeout",             None, true, false);
        let mut url_string                = parse_soap(&response[..], "Uri",                 None, true, false);

        info!("RTSP URL: {}", url_string[0]);
        
        let mut result                 = StreamUri::default(); 
        result.invalid_connect         = Some(invalid_after_connect.remove(0));
        result.uri                     = Some(url_string           .remove(0));
        result.timeout                 = Some(timeout              .remove(0));

        Ok(result)
    }

    #[rustfmt::skip]
    async fn set_services(onvif_url: url::Url) -> Result<Services> {
        let response         = client::send(onvif_url, Messages::GetServices).await?;
        let response         = response.bytes().await?;
        let services         = parse_soap(&response[..], "XAddr", None, false, false);
        let mut result       = Services::default(); 

        for service in services {
            info!("Service: {}", service);
            
            // Match Service URL Address by keywords
            match &service {
                s if s.contains("device_service")    =>(),
                s if s.contains("analytics")         => result.analytics    = Some(service.clone()),
                s if s.contains("event")             => result.event        = Some(service.clone()),
                s if s.contains("deviceIO")          => result.io           = Some(service.clone()),
                s if s.contains("imaging")           => result.imaging      = Some(service.clone()),
                s if s.contains("media_service")     => result.media        = Some(service.clone()),
                s if s.contains("media2")            => result.media2       = Some(service.clone()),
                s if s.contains("ptz")               => result.ptz          = Some(service.clone()),
                _ => error!("Encountered unknown Service"),
            }
        }

        Ok(result)
    }

    async fn set_service_capabilities<T>(onvif_url: url::Url) -> Result<T>
    where
        T: ServiceCapabilities + Default
    {
        debug!("Event Service URL: {onvif_url}");
        let response         = client::send(onvif_url, Messages::GetServiceCapabilities).await?;
        let resp1            = response.text().await?;
        let resp2            = resp1.as_bytes();
        let capabilities     = parse_soap(&resp2[..], "Capabilities", None, true, true);
        let mut result       = T::default();

        // debug!("Get capabilities: \n{resp1}");

        capabilities[0]
            .split(" ")
            .map(|s| s.split_once('=').unwrap())
            .collect::<Vec<(&str, &str)>>()
            .iter()
            .for_each(|v| result.set_prop_with_pair(*v));

        Ok(result)
    }
    
    #[rustfmt::skip]
    async fn set_analytics_configurations(onvif_url: url::Url) -> Result<AnalyticsConfigList> {
        let response         = client::send(onvif_url, Messages::GetAnalyticsConfigurations).await?;
        let resp1            = response.text().await?;
        // let resp2            = resp1.as_bytes();
        // let capabilities     = parse_soap(&resp2[..], "Capabilities", None, true, true);
        let mut result       = AnalyticsConfigList::default(); 

        debug!("Get analytics configs: \n{resp1}");

        Ok(result)
    }

    #[rustfmt::skip]
    async fn set_event_properties(onvif_url: url::Url) -> Result<()> {
        let response         = client::send(onvif_url, Messages::GetEventProperties).await?;
        let resp1            = response.text().await?;
        // let resp2            = resp1.as_bytes();
        // let capabilities     = parse_soap(&resp2[..], "Capabilities", None, true, true);

        debug!("Get event properties: \n{resp1}");

        Ok(())
    }

    #[rustfmt::skip]
    async fn set_event_brokers(onvif_url: url::Url) -> Result<()> {
        let response         = client::send(onvif_url, Messages::GetEventBrokers).await?;
        // let response                      = response.bytes().await?;
        let response                      = response.text().await?;

        debug!("Get Event Brokers: \n{response}");

        Ok(())
    }

    #[rustfmt::skip]
    async fn pull_messages(onvif_url: url::Url) -> Result<()> {
        let response         = client::send(onvif_url, Messages::PullMessages).await?; // let response                      = response.bytes().await?;
        let response                      = response.text().await?;

        debug!("Pull Event Messages: \n{response}");

        Ok(())
    }
    
    #[rustfmt::skip]
    async fn set_service_profiles(onvif_url: url::Url) -> Result<()> {
        let response                      = client::send(onvif_url, Messages::GetProfiles).await?;
        // let response                      = response.bytes().await?;
        let response                      = response.text().await?;

        debug!("Get Profiles: \n{response}");

        Ok(())
    }
    
    #[rustfmt::skip]
    async fn set_dns(onvif_url: url::Url) -> Result<()> {
        let response                      = client::send(onvif_url, Messages::GetDNS).await?;
        // let response                      = response.bytes().await?;
        let response                      = response.text().await?;

        debug!("Get DNS: \n{response}");

        Ok(())
    }

    async fn set_dot11_status(onvif_url: url::Url) -> Result<()> {
        let response                      = client::send(onvif_url, Messages::GetDot11Status).await?;
        // let response                      = response.bytes().await?;
        let response                      = response.text().await?;

        trace!("Get Dot11 Status\n {response}");

        Ok(())
    }
    
    async fn set_geo_location(onvif_url: url::Url) -> Result<()> {
        let response                      = client::send(onvif_url, Messages::GetGeoLocation).await?;
        // let response                      = response.bytes().await?;
        let response                      = response.text().await?;

        trace!("Get Geo Location\n {response}");
        
        Ok(())
    }
    
    async fn set_pull_point_sub(onvif_url: url::Url) -> Result<()> {
        debug!("Event Service URL: {onvif_url}");
        let response                      = client::send(onvif_url, Messages::CreatePullPointSubscriptionRequest).await?;
        // let response                      = response.bytes().await?;
        let response                      = response.text().await?;

        debug!("Get Pull Point Subscription\n {response}");

        Ok(())
    }
    
    async fn build_all(&mut self) -> Result<()>;
}
