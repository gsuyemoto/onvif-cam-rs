This is a very bare bones implementation of the ONVIF protocol. The following messages are implemented:

* Discovery
* Capabilities
* DeviceInfo
* Profiles
* GetStreamURI

Implementation of those messages are bare basics and don't store or parse the entire SOAP response in many cases. This whole lib is really in support of an RTSP/RTP/H264 streaming client I wrote at https://github.com/gsuyemoto/rtsp-cam-rs, which will become a Rust crate soon.

The example discovers an IP camera (only tested on a single Topodome) and then uses OpenCV to stream via RTP and detect faces via Haar cascades.

In order to run the example, you will need Clang and OpenCV. On Debian Linux:
```bash
sudo apt-get install libclang-dev libopencv-dev
```
