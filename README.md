A quick repo to help in development of software for IP cameras. This is a very bare bones implementation of the ONVIF protocol. Basically, I have one message implemented which is GetStreamURI. The rest of the lib is dedicated to device discovery. This whole repo is really in support of an RTSP/RTP/H264 lib client I wrote at https://github.com/gsuyemoto/rtsp-client-rs.

The example discovers an IP camera (only tested on a single Topodome) and then uses OpenCV to stream and detect faces via Haar cascades.
