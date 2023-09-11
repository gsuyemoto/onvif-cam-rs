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
    pub media2:        Option<String>,
    pub ptz:           Option<String>,
}

#[derive(Default)]
#[rustfmt::skip]
pub struct AnalyticsConfig {
    pub token:        Option<String>,
    pub name:         Option<String>,
    pub use_count:    Option<u8>,
}

#[derive(Default)]
pub struct AnalyticsConfigList {
    pub configs: Vec<AnalyticsConfig>,
}

pub trait ServiceCapabilities {
    fn set_prop_with_pair(&mut self, pair: (&str, &str));
}

#[derive(Default)]
#[rustfmt::skip]
pub struct EventCapabilities {
    pub pause_support:            Option<bool>,
    pub pull_point_supoort:       Option<bool>,
    pub sub_policy_support:       Option<bool>,
    pub max_notif_produce:        Option<u8>,
    pub max_pull_points:          Option<u8>,
    pub persist_notif_store:      Option<bool>,
}

#[rustfmt::skip]
impl ServiceCapabilities for EventCapabilities {
    fn set_prop_with_pair(&mut self, pair: (&str, &str)) {
        match pair.0 {
            key if key.contains("PausableSubscription")
                => self.pause_support = pair.1.parse().ok(),
            
            key if key.contains("PullPointSupport")
                => self.pull_point_supoort = pair.1.parse().ok(),
            
            key if key.contains("PolicySupport")
                => self.sub_policy_support = pair.1.parse().ok(),
            
            key if key.contains("MaxNotification")
                => self.max_notif_produce = pair.1.parse().ok(),
            
            key if key.contains("MaxNullPoints")
                => self.max_pull_points = pair.1.parse().ok(),
            
            key if key.contains("NotificationStorage")
                => self.persist_notif_store = pair.1.parse().ok(),

            _   => eprintln!("Unknown key pair for capabilities"),
        }
    }
}

#[derive(Default)]
#[rustfmt::skip]
pub struct AnalyticsCapabilities {
    pub rule_support:                 Option<bool>,
    pub analytics_module:             Option<bool>,
    pub cell_based_scene:             Option<bool>,
    pub rule_options:                 Option<bool>,
    pub analytics_module_options:     Option<bool>,
    pub supported_metadata:           Option<bool>,
    pub image_sending_type:           Option<String>,
}

#[rustfmt::skip]
impl ServiceCapabilities for AnalyticsCapabilities {
    fn set_prop_with_pair(&mut self, pair: (&str, &str)) {
        match pair.0 {
            key if key.contains("RuleSupport")
                => self.rule_support = pair.1.parse().ok(),
            
            key if key.contains("AnalyticsModuleSupport")
                => self.analytics_module = pair.1.parse().ok(),
            
            key if key.contains("CellBasedSceneDescriptionSupported")
                => self.cell_based_scene = pair.1.parse().ok(),
            
            key if key.contains("RuleOptionsSupported")
                => self.rule_options = pair.1.parse().ok(),
            
            key if key.contains("AnalyticsModuleOptionsSupported")
                => self.analytics_module_options = pair.1.parse().ok(),
            
            key if key.contains("SupportedMetadata")
                => self.supported_metadata = pair.1.parse().ok(),

            key if key.contains("ImageSendingType")
                => self.image_sending_type = pair.1.parse().ok(),

            _   => eprintln!("Unknown key pair for capabilities"),
        }
    }
}

pub fn parse_device_type(dev_type: String) -> DeviceTypes {
    match dev_type {
        a if a.contains("NetworkVideoTransmitter") => DeviceTypes::Camera,
        a if a.contains("Doorbell") => DeviceTypes::Doorbell,
        _ => DeviceTypes::Unknown,
    }
}
