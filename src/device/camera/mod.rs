use crate::client::parse_soap;
use crate::builder::Device;

use anyhow::{anyhow, Result};
use log::info;
use url::Url;

const URL_ONVIF_DEFAULT: Url = Url::parse("http://127.0.0.1").unwrap();

pub struct Camera {
    pub url_onvif: Url, // http://ip.address/onvif/device_service
    // Get stream
    pub url_rtsp: Option<Url>,
    pub invalid_after_connect: Option<String>,
    pub timeout: Option<String>,
    // Profiles
    pub video_codec: Option<String>,
    pub audio_codec: Option<String>,
    pub video_dim: Option<(u16, u16)>,
    pub h264_profile: Option<String>,
    // Camera Info
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub firmware_version: Option<String>,
    pub serial_number: Option<String>,
    pub hardware_id: Option<String>,
    // Capabilities
    pub service_media: Option<String>,
    pub service_event: Option<String>,
    pub service_analytics: Option<String>,
    pub service_ptz: Option<String>,
    pub service_image: Option<String>,
}

impl Device for Camera {
    type DeviceType = Camera;
    
    fn build(self) -> Camera {

        Camera {
            url_onvif,
            url_rtsp: None,
            invalid_after_connect: None,
            timeout: None,
            video_codec: None,
            audio_codec: None,
            video_dim: None,
            h264_profile: None,
            manufacturer: None,
            model: None,
            firmware_version: None,
            serial_number: None,
            hardware_id: None,
            service_media: None,
            service_event: None,
            service_analytics: None,
            service_ptz: None,
            service_image: None,
        }
    }

    fn set_onvif_url(&mut self, url: url::Url) {
        self.url_onvif = url;
    }

    fn get_onvif_url(&self) -> url::Url {
        self.url_onvif
    }

    #[rustfmt::skip]
    fn set_capabilities(&mut self, response: bytes::Bytes) {
        let response              = response.slice(..);
        let media_service         = parse_soap(&response[..], "XAddr", Some("Media"),       true);
        let event_service         = parse_soap(&response[..], "XAddr", Some("Events"),      true);
        let analytics_service     = parse_soap(&response[..], "XAddr", Some("Analytics"),   true);
        let ptz_service           = parse_soap(&response[..], "XAddr", Some("PTZ"),         true);
        let image_service         = parse_soap(&response[..], "XAddr", Some("Imaging"),     true);

        info!("Imaging service: {}", image_service[0]);

        self.service_media       = Some(media_service[0].clone());
        self.service_event       = Some(event_service[0].clone());
        self.service_analytics   = Some(analytics_service[0].clone());
        self.service_ptz         = Some(ptz_service[0].clone());
        self.service_image       = Some(image_service[0].clone());
    }

    #[rustfmt::skip]
    fn set_device_info(&mut self, response: bytes::Bytes) {
        let response             = response.slice(..);
        let firmware_version     = parse_soap(&response[..], "FirmwareVersion",  None, true);
        let serial_number        = parse_soap(&response[..], "SerialNumber",     None, true);
        let hardware_id          = parse_soap(&response[..], "HardwareId",       None, true);
        let model                = parse_soap(&response[..], "Model",            None, true);
        let manufacturer         = parse_soap(&response[..], "Manufacturer",     None, true);

        info!("Manufacturer: {}", manufacturer[0]);
        info!("Model: {}", model[0]);

        self.firmware_version    = Some(firmware_version[0].clone());
        self.serial_number       = Some(serial_number[0].clone());
        self.hardware_id         = Some(hardware_id[0].clone());
        self.model               = Some(model[0].clone());
        self.manufacturer        = Some(manufacturer[0].clone());
    }

    #[rustfmt::skip]
    fn set_profiles(&mut self, response: bytes::Bytes) {
        let response             = response.slice(..);
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

        self.video_dim       = Some((width[0].parse().unwrap(), height[0].parse().unwrap()));
        self.audio_codec     = Some(audio_codec.remove(0));
        self.h264_profile    = Some(h264_profile.remove(0));
        self.video_codec     = Some(video_codec.remove(0));
    }

    #[rustfmt::skip]
    fn set_stream_uri(&mut self, response: bytes::Bytes) {
        let response                  = response.slice(..);
        let invalid_after_connect     = parse_soap(&response[..], "InvalidAfterConnect", None, true);
        let timeout                   = parse_soap(&response[..], "Timeout",             None, true);
        let url_string                = parse_soap(&response[..], "Uri",                 None, true);
        let url                       = url_string[0].parse()?;

        info!("RTSP URI: {}", url_string[0]);
        
        self.url_rtsp                = Some(url);
        self.invalid_after_connect   = Some(invalid_after_connect[0].clone());
        self.timeout                 = Some(timeout[0].clone());

        // let _ = io::file_save(&self.devices)?;
    }

    fn set_services(&mut self, response: bytes::Bytes) {
        info!("[Camera][set_services]");
    }

    fn set_service_capabilities(&mut self, response: bytes::Bytes) {
        info!("[Camera][set_servie_capabilities]");
    }

    fn set_dns(&mut self, response: bytes::Bytes) {
        info!("[Camera][set_dns]");
    }
}

impl Camera {
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
    pub fn get_stream_uri(&self) -> Result<String> {
        match self.url_rtsp {
            Some(url_rtsp) => Ok(url_rtsp),
            None => Err(anyhow!(
                "[Camera][get_stream_uri] No RTSP URL for this device"
            )),
        }
    }
}
