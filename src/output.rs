use crate::clock::Clock;

use std::{
    format,
    fs::{OpenOptions, File},
    os::unix::io::{
        RawFd,
        AsRawFd
    }
};

use drm::{
    Device as BasicDevice,
    buffer::PixelFormat,
    control::{
        Device as ControlDevice,
        Mode,
        ResourceHandle,
        ResourceInfo,
        connector,
        crtc,
        dumbbuffer::{DumbBuffer, DumbMapping},
        framebuffer::{
            Info as FrameBuffer,
            Handle as FrameBufferHandle,
            create as createfb
        }
    }
};

use nix::sys::select::{FdSet, select};


// needed because the api is generic to the point of breaking type inference.
fn load_information<T, U>(card: &Card, handles: &[T]) -> Vec<U>
    where
    T: ResourceHandle,
    U: ResourceInfo<Handle = T>,
{
    handles
        .iter()
        .map(|&h| card
             .resource_info(h) // XXX: Where is this implemented??!
             .expect("Could not load resource info")
        )
        .collect()
}


// Library does not provide default implementation of Device, so we
// define our own type which is just a trivial wrapper around RawFd.
struct Card {file: File}
impl AsRawFd for Card {fn as_raw_fd(&self) -> RawFd {self.file.as_raw_fd()}}
impl BasicDevice for Card {}
impl ControlDevice for Card {}
impl Card {
    pub fn open(path: &str) -> Card {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .expect(&format!("Couldn't open {}", path));

        Card{file}
    }
}


fn await_vblank(card: &Card) {
    let mut fds = FdSet::new();
    fds.insert(card.as_raw_fd());

    loop {
        if let Ok(nfds) = select(None, Some(&mut fds), None, None, None) {
            if nfds > 0 {
                // if we get here, it's safe to extract events
                // from the fd.
                if let Ok(events) = crtc::receive_events(card) {
                    for event in events {
                        // If we receive a PageFlip, it's safe to
                        // queue the next one.
                        match event {
                            crtc::Event::PageFlip(_) => return,
                            _ => ()
                        }
                    }
                }
            }
        }
    }
}


fn dumb_buffer(card: &Card, mode: &Mode) -> DumbBuffer {
    let fmt = PixelFormat::XRGB8888;
    let sz = mode.size();
    let sz = (sz.0 as u32, sz.1 as u32);

    DumbBuffer::create_from_device(card, sz, fmt)
        .expect("Could not create dumb buffer")
}

struct Page<'a> {
    fb_priv: FrameBuffer,
    pub fb: FrameBufferHandle,
    pub dm: DumbMapping<'a>,
}

impl<'a> Page<'a> {
    pub fn new(card: &Card, db: &'a mut DumbBuffer) -> Page<'a> {
        let fb = createfb(card, db).expect("!");
        Page {
            fb_priv: fb,
            fb: fb.handle(),
            dm: db.map(card).expect("!")
        }
    }
}


// Run forever, redrawing the screen as fast as possible, using
// double-buffering.
fn render(card: Card) {
    // Set up the connection to the GPU ....
    let res = card
        .resource_handles()
        .expect("Could not load normal resource ids.");

    let connectors: Vec<connector::Info> =
        load_information(&card, res.connectors());

    let connector = connectors
        .iter()
        .filter(|c| c.connection_state() == connector::State::Connected)
        .next()
        .expect("No display is connected.");

    // Get the first (usually best) mode
    let &mode = connector
        .modes()
        .iter()
        .next()
        .expect("no mode!");

    // Get the crtc
    let crtcs: Vec<crtc::Info> = load_information(&card, res.crtcs());
    let crtc = crtcs
        .iter()
        .next()
        .expect("Couldn't get crtc");

    // .... To here

    // Cache some values
    let index = [0, 1];
    let mut front = dumb_buffer(&card, &mode);
    let mut back = dumb_buffer(&card, &mode);

    let mut pages = [Page::new(&card, &mut front), Page::new(&card, &mut back)];
    let clock = Clock::new();
    let pf_flags = [crtc::PageFlipFlags::PageFlipEvent];
    let con_hdl = [connector.handle()];
    let orig = (0, 0);

    // Set initial mode on the crtc.
    crtc::set(&card, crtc.handle(), pages[0].fb, &con_hdl, orig, Some(mode))
        .expect("Could not set CRTC");

    for &index in index.iter().cycle() {
        let page = &mut pages[index];
        let fb = page.fb;
        let buf = page.dm.as_mut();
        let val = ((clock.seconds().sin() * 127.0) + 128.0) as u8;

        // Fill the buffers with values.
        draw(val, buf.as_mut());

        // Request a page flip. The actual page flip will happen
        // some time later. We cannot call this again until we
        // have received the page flip event, but the page flip is
        // handled for us.
        crtc::page_flip(&card, crtc.handle(), fb, &pf_flags)
            .expect("Could not set CRTC");

        await_vblank(&card);
    }
}


pub fn draw(value: u8, buffer: &mut [u8]) {
    for b in buffer {
        *b = value;
    }
}


pub fn drm_magic() -> () {
    render(Card::open("/dev/dri/card0"));
}
