use std::{
    io::{Error, Result},
    fs::File,
    os::unix::io::{
        IntoRawFd,
        RawFd
    }
};

use drm::{
    drm::Capability,
    drm_mode,
    drm_mode::{Crtc, Encoder, ModeInfo, Connection}
};


#[derive(Debug)]
struct Connector {
    pub id: u32,
    pub encoder_id: u32,
    pub state: Connection,
    pub type_name: &'static str,
    pub modes: Vec<ModeInfo>
}

impl Connector {
    pub fn new(connector: drm_mode::Connector) -> Connector {
        Connector {
            id: connector.get_connector_id(),
            encoder_id: connector.get_encoder_id(),
            state: connector.get_connection(),
            type_name: connector.get_type_name(),
            modes: connector.get_modes()
        }
    }
}


#[derive(Debug)]
struct Resources {
    pub count_fbs: i32,
    pub crtcs: Vec<Crtc>,
    pub encoders: Vec<Encoder>,
    pub connectors: Vec<Connector>
}


impl Resources {
    pub fn new(fd: RawFd) -> Option<Resources> {
        let resources = drm_mode::get_resources(fd)?;
        let count_fbs = resources.get_count_fbs();
        let crtcs = resources
            .get_crtcs()
            .into_iter()
            .filter_map(|id| drm_mode::get_crtc(fd, id))
            .collect();

        let encoders = resources
            .get_encoders()
            .into_iter()
            .filter_map(|id| drm_mode::get_encoder(fd, id))
            .collect();

        let connectors = resources
            .get_connectors()
            .into_iter()
            .filter_map(|id| drm_mode::get_connector(fd, id))
            .map(|c| Connector::new(c))
            .collect();

        Some(Resources {count_fbs, crtcs, encoders, connectors})
    }
}

struct OutputDevice {
    fd: RawFd
}

impl OutputDevice {
    pub fn open_drm(path: &str) -> Result<OutputDevice> {
        let fd = File::open(path)?.into_raw_fd();
        Ok(OutputDevice { fd })
    }

    pub fn get_resources(&self) -> Option<Resources> {
        Resources::new(self.fd)
    }

    pub fn has(&self, cap: Capability) -> Option<bool> {
        match drm::drm::get_cap(self.fd, cap) {
            Ok(has_cap) => Some(has_cap == 1),
            _ => None
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
        if let Some(resources) = output.get_resources() {
            println!("{:#?}", resources);
        }
    } else {
        println!("Device doesn't have dumb buffer support.");
    }
}
