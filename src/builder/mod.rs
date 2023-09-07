pub trait Builder {
    type DeviceType;

    fn set_capabilities(&mut self, response: bytes::Bytes);
    fn set_device_info(&mut self, response: bytes::Bytes);
    fn set_profiles(&mut self, response: bytes::Bytes);
    fn set_stream_uri(&mut self, response: bytes::Bytes);
    fn set_services(&mut self, response: bytes::Bytes);
    fn set_service_capabilities(&mut self, response: bytes::Bytes);
    fn set_dns(&mut self, response: bytes::Bytes);
    fn build(self) -> Self::DeviceType;
}
