use crate::builder::camera::CameraBuilder;
use crate::device::*;

use anyhow::{anyhow, Result};
use async_trait::async_trait;

pub struct Camera {
    base: Device,
    capabilities: Capabilities,
    profiles: Profiles,
    device_info: DeviceInfo,
    stream: StreamUri,
}

#[async_trait]
impl CameraBuilder for Camera {
    #[rustfmt::skip]
    async fn build_all(&mut self) -> Result<()> {
        self.capabilities =     Camera::set_capabilities(    self.base.url_onvif.clone()).await?;
        self.profiles =         Camera::set_profiles(        self.base.url_onvif.clone()).await?;
        self.device_info =      Camera::set_device_info(     self.base.url_onvif.clone()).await?;
        self.stream =           Camera::set_stream_uri(      self.base.url_onvif.clone()).await?;

        Ok(())
    }
}

impl Camera {
    pub fn new(base: Device) -> Self {
        Camera {
            base,
            capabilities: Capabilities::default(),
            profiles: Profiles::default(),
            device_info: DeviceInfo::default(),
            stream: StreamUri::default(),
        }
    }

    pub fn get_stream_uri(&self) -> Result<&url::Url, &str> {
        self.stream.uri.as_ref().ok_or("No uri found").clone()
    }
}
