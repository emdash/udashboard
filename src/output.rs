use std::{
    collections::HashMap,
    ffi::c_void,
    fs::File,
    io::{Error, Result},
    intrinsics::transmute,
    os::unix::io::{
        IntoRawFd,
        RawFd
    }
};

use drm::{
    drm::Capability,
    drm_mode,
    drm_mode::{
        Crtc,
        CrtcId,
        Encoder,
        EncoderId,
        ModeInfo,
        Connection,
        Connector
    },
    ffi
};


// A display is a particular combination of connector, encoder, crt,
// operating in a given mode.
#[derive(Debug)]
struct Display {
    pub geometry: (u32, u32),
    pub mode: ModeInfo,
    pub encoder: Encoder,
    pub crtc: Crtc
}


// our drm-rs crate provides no abstraction over buffers.
struct Buffer {}


struct OutputDevice {
    fd: RawFd
}


impl OutputDevice {
    pub fn open_drm(path: &str) -> Result<OutputDevice> {
        let fd = File::open(path)?.into_raw_fd();
        Ok(OutputDevice { fd })
    }

    pub fn has(&self, cap: Capability) -> Option<bool> {
        match drm::drm::get_cap(self.fd, cap) {
            Ok(has_cap) => Some(has_cap == 1),
            _ => None
        }
    }

    pub fn get_available_displays(&self) -> Vec<Display> {
        let resources = drm_mode::get_resources(self.fd)
            .expect("Couldn't get resources");

        resources.get_connectors()
            .into_iter()
            .filter_map(|id| drm_mode::get_connector(self.fd, id))
            .filter(|c| c.get_count_modes() > 0)
            .filter(|c| c.get_connection() == Connection::Connected)
            .map(|c| self.create_display_for_connector(c))
            .collect()
    }

    fn create_display_for_connector(&self, connector: Connector) -> Display {
        let mode = connector.get_modes()[0].clone();
        let encoder = drm_mode::get_encoder(self.fd, connector.get_encoder_id())
            .expect("Connector does not have an encoder.");
        let crtc = drm_mode::get_crtc(self.fd, encoder.get_crtc_id())
            .expect("Encoder does not have a crtc.");

        Display {
            geometry: (mode.get_hdisplay().into(), mode.get_vdisplay().into()),
            mode: mode,
            encoder: encoder,
            crtc: crtc
        }
    }
}


pub fn drm_magic() {
    let device = "/dev/dri/card0";
    let output = OutputDevice::open_drm(device).expect("Couldn't open device");
    let has_dumb_buffer = output
        .has(Capability::DumbBuffer)
        .expect("Get Capability Failed");

    if has_dumb_buffer {
        println!("{:#?}", output.get_available_displays());
    } else {
        println!("Device doesn't have dumb buffer support.");
    }
}
