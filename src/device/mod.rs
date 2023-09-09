pub mod camera;

pub enum DeviceTypes {
    Camera,
    Doorbell,
    Unknown,
}

#[rustfmt::skip]
pub struct Device {
    pub url_onvif:     url::Url,
    pub device_type:   DeviceTypes,
    pub scopes:        Vec<String>,
}

#[derive(Default)]
#[rustfmt::skip]
pub struct Capabilities {
    pub url_media:       Option<url::Url>,
    pub url_events:      Option<url::Url>,
    pub url_analytics:   Option<url::Url>,
    pub url_ptz:         Option<url::Url>,
    pub url_imaging:     Option<url::Url>,
}

#[derive(Default)]
#[rustfmt::skip]
pub struct DeviceInfo {
    pub firmware_version:   Option<String>,
    pub serial_num:         Option<String>,
    pub hardware_id:        Option<String>,
    pub model:              Option<String>,
    pub manufacturer:       Option<String>,
}

#[derive(Default)]
#[rustfmt::skip]
pub struct Profiles {
    pub name:          Option<String>,
    pub video_dim:     Option<(u32, u32)>,
    pub video_codec:   Option<String>,
    pub audio_codec:   Option<String>,
    pub h264_profile:  Option<String>,
}

#[derive(Default)]
#[rustfmt::skip]
pub struct StreamUri {
    pub uri:               Option<String>,
    pub timeout:           Option<String>,
    pub invalid_connect:   Option<String>,
}

#[derive(Default)]
#[rustfmt::skip]
pub struct Services {
    pub analytics:     Option<String>,
    pub event:         Option<String>,
    pub io:            Option<String>,
    pub imaging:       Option<String>,
    pub media:         Option<String>,
    pub ptz:           Option<String>,
}

pub fn parse_device_type(dev_type: String) -> DeviceTypes {
    match dev_type {
        a if a.contains("NetworkVideoTransmitter") => DeviceTypes::Camera,
        a if a.contains("Doorbell") => DeviceTypes::Doorbell,
        _ => DeviceTypes::Unknown,
    }
}
