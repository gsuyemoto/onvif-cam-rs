pub enum DeviceTypes {
    Camera,
    Printer,
}

pub enum Capabilities {
    UrlMedia(url::Url),
    UrlEvents(url::Url),
    UrlAnalytics(url::Url),
    UrlPTZ(url::Url),
    UrlImaging(url::Url),
}

pub enum DeviceInfo {
    FirmwareVersion(String),
    SerialNumber(String),
    HardwareId(String),
    Model(String),
    Manufacturer(String),
}

pub struct DeviceBase {
    url_onvif: url::Url,
}
