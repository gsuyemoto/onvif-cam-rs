use crate::device::{Capabilities, DeviceInfo, Profiles, StreamUri};
use crate::utils::parse_soap;
use crate::client::{self, Messages};

use log::info;
use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait CameraBuilder {
    #[rustfmt::skip]
    async fn set_capabilities(onvif_url: url::Url) -> Result<Capabilities> {
        let response              = client::send(onvif_url, Messages::Capabilities).await?;
        let response              = response.bytes().await?;
        let mut media_service     = parse_soap(&response[..], "XAddr", Some("Media"),       true);
        let mut event_service     = parse_soap(&response[..], "XAddr", Some("Events"),      true);
        let mut analytics_service = parse_soap(&response[..], "XAddr", Some("Analytics"),   true);
        let mut ptz_service       = parse_soap(&response[..], "XAddr", Some("PTZ"),         true);
        let mut image_service     = parse_soap(&response[..], "XAddr", Some("Imaging"),     true);

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
        let mut firmware_version     = parse_soap(&response[..], "FirmwareVersion",  None, true);
        let mut serial_number        = parse_soap(&response[..], "SerialNumber",     None, true);
        let mut hardware_id          = parse_soap(&response[..], "HardwareId",       None, true);
        let mut model                = parse_soap(&response[..], "Model",            None, true);
        let mut manufacturer         = parse_soap(&response[..], "Manufacturer",     None, true);

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
        let width                 = parse_soap(&response[..], "Width",          None,                                 true);
        let height                = parse_soap(&response[..], "Height",         None,                                 true);
        let mut video_codec       = parse_soap(&response[..], "Encoding",       Some("VideoEncoderConfiguration"),    true);
        let mut audio_codec       = parse_soap(&response[..], "Encoding",       Some("AudioEncoderConfiguration"),    true);
        let mut h264_profile      = parse_soap(&response[..], "H264Profile",    None,                                 true);

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
        let mut invalid_after_connect     = parse_soap(&response[..], "InvalidAfterConnect", None, true);
        let mut timeout                   = parse_soap(&response[..], "Timeout",             None, true);
        let mut url_string                = parse_soap(&response[..], "Uri",                 None, true);

        info!("RTSP URL: {}", url_string[0]);
        
        let mut result                 = StreamUri::default(); 
        result.uri                     = Some(url_string.remove(0));
        result.invalid_connect         = Some(invalid_after_connect.remove(0));
        result.timeout                 = Some(timeout.remove(0));

        Ok(result)
    }

    #[rustfmt::skip]
    fn set_services(onvif_url: url::Url) {
        info!("[CameraBuilder] set services");
    }

    #[rustfmt::skip]
    fn set_service_capabilities(onvif_url: url::Url) {
        info!("[CameraBuilder] set service capabilities");
    }

    #[rustfmt::skip]
    fn set_dns(onvif_url: url::Url) {
        info!("[CameraBuilder] set dns");
    }

    async fn build_all(&mut self) -> Result<()>;
}
