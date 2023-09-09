use crate::builder::camera::CameraBuilder;
use crate::device::*;

use anyhow::Result;
use async_trait::async_trait;

#[rustfmt::skip]
pub struct Camera {
    base:             Device,
    capabilities:     Capabilities,
    profiles:         Profiles,
    device_info:      DeviceInfo,
    pub stream:       StreamUri,
    services:         Services,
}

#[async_trait]
impl CameraBuilder for Camera {
    #[rustfmt::skip]
    async fn build_all(&mut self) -> Result<()> {
        self.capabilities     = Camera::set_capabilities(    self.base.url_onvif.clone()).await?;
        self.device_info      = Camera::set_device_info(     self.base.url_onvif.clone()).await?;
        self.profiles         = Camera::set_profiles(        self.base.url_onvif.clone()).await?;
        self.stream           = Camera::set_stream_uri(      self.base.url_onvif.clone()).await?;
        self.services         = Camera::set_services(        self.base.url_onvif.clone()).await?;
        // _ =           Camera::set_dot11_status(      self.base.url_onvif.clone()).await?;
        // _ =           Camera::set_geo_location(      self.base.url_onvif.clone()).await?;

        // Get the EVENT SERVICES Url to send request for EVENT pull point
        // let url               = self.services.event.as_ref().unwrap();
        // let event_url         = url::Url::parse(&url)?;
        // _                     = Camera::set_pull_point_sub(event_url).await?;

        
        // Get the EVENT SERVICES Url to send request for EVENT pull point
        let url               = self.services.event.as_ref().unwrap();
        let event_url         = url::Url::parse(&url)?;
        _                     = Camera::set_service_capabilities(event_url).await?;

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
            services: Services::default(),
        }
    }
}

#[rustfmt::skip]
impl From<&str> for Camera {
    fn from(input: &str) -> Self {
        let url_onvif = match url::Url::parse(input) {
            Ok(url) => url,
            Err(e) => panic!("[Device][Camera] Error parsing str: {e}"),
        };

        let base = Device {
            url_onvif,
            device_type:    DeviceTypes::Camera,
            scopes:         Vec::new(),
        };    

        Camera {
            base,
            capabilities: Capabilities::default(),
            profiles: Profiles::default(),
            device_info: DeviceInfo::default(),
            stream: StreamUri::default(),
            services: Services::default(),
        }
    }
}
