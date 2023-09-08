use crate::device::*;
use anyhow::Result;

pub mod camera;

pub trait Builder {
    fn set_capabilities(onvif_url: url::Url) -> Result<Capabilities>;
    fn set_device_info(onvif_url: url::Url) -> Result<DeviceInfo>;
    fn set_profiles(onvif_url: url::Url) -> Result<Profiles>;
    fn set_stream_uri(onvif_url: url::Url) -> Result<StreamUri>;
    fn set_services(onvif_url: url::Url);
    fn set_service_capabilities(onvif_url: url::Url);
    fn set_dns(onvif_url: url::Url);
    fn build_all(&mut self);
}
