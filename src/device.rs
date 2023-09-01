use url::Url;

pub struct Device {
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
    // Device Info
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

impl Device {
    pub fn new() -> Self {
        let url_onvif = Url::parse("http://127.0.0.1").unwrap();

        Device {
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
}
