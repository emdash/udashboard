use std::{
    io::{Error, Result},
    fs::File,
    os::unix::io::{
        IntoRawFd,
        RawFd
    }
};

use drm::{
    drm_mode,
    drm_mode::{Crtc, Encoder, Connector}
};

const device: &str = "/dev/dri/card0";

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
        let crtcs = Resources::map(
            resources.get_crtcs(),
            |id| drm_mode::get_crtc(fd, id)
        );
        let encoders = Resources::map(
            resources.get_encoders(),
            |id| drm_mode::get_encoder(fd, id)
        );
        let connectors = Resources::map(
            resources.get_connectors(),
            |id| drm_mode::get_connector(fd, id)
        );
        Some(Resources {count_fbs, crtcs, encoders, connectors})
    }

    // I am really, really tired of getting nasty compiler errors when
    // I try to do things like iter().map()....collect(). It shouldn't
    // be this fucking hard, but I always get weird type mismatch
    // errors because the compiler is too stupid to figure out what
    // 99% of us actually want.
    //
    // Seriously, why does *this* work, when all that iterator
    // nonsense never seems to compile>?
    //
    // And why can't some simple api for transforming vectors just be
    // part of std?
    fn map<A, B, F>(collection: Vec<A>, func: F) -> Vec<B>
        where F: Fn(A) -> Option<B>
    {
        let mut ret = Vec::new();

        for a in collection {
            if let Some(b) = func(a) {
                ret.push(b);
            }
        }

        ret
    }
}

struct OutputDevice {
    fd: RawFd,
}

impl OutputDevice {
    pub fn open_drm(path: &str) -> Result<OutputDevice> {
        let fd = File::open(device)?.into_raw_fd();
        Ok(OutputDevice { fd })
    }

    pub fn get_resources(&self) -> Option<Resources> {
        Resources::new(self.fd)
    }
}

pub fn drm_magic() {
    let output = OutputDevice::open_drm(device).expect("Couldn't open device");
    if let Some(resources) = output.get_resources() {
        println!("{:?}", resources);
    }
}
