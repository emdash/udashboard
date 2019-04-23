use crate::clock::Clock;

use std::{
    format,
    fs::{OpenOptions, File},
    os::unix::io::{
        RawFd,
        AsRawFd
    },
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
        dumbbuffer,
        framebuffer
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


// Since the primary use case is for a single display, we just move
// the card into the display. In the kernel it's the other way around:
// the card owns all its resources. But in userland, all the relevant
// calls are implemented as ioctls on the card. If we need multiple
// displays, probably just wrapping this in Rc<Card> or Arc<Card> would
// be enough.
struct Display {
    card: Card
}

impl Display {
    pub fn new(card: Card) -> Display {Display{card}}

    pub fn render_loop(self) {
        // Load the information.
        let res = self.card
            .resource_handles()
            .expect("Could not load normal resource ids.");

        let connectors: Vec<connector::Info> =
            load_information(&self.card, res.connectors());

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
            .expect("No modes found on connector.");

        let crtcs: Vec<crtc::Info> = load_information(&self.card, res.crtcs());
        let crtc = crtcs
            .iter()
            .next()
            .expect("Couldn't get crtc");

        let index = [0, 1];
        let mut front = self.create_fb(mode);
        let mut back = self.create_fb(mode);
        let fbs = [front.0.handle(), back.0.handle()];
        let mut bufs = [
            front.1.map(&self.card).expect("!"),
            back.1.map(&self.card).expect("!")
        ];
        let clock = Clock::new();

        // Set initial mode on the crtc.
        crtc::set(
            &self.card,
            crtc.handle(),
            fbs[0],
            &[connector.handle()],
            (0, 0),
            Some(mode)
        ).expect("Could not set CRTC");

        for &index in index.iter().cycle() {
            let fb = fbs[index];
            let buf = &mut bufs[index];

            // Fill the buffers with values.
            render(
                ((clock.seconds().sin() * 127.0) + 128.0) as u8,
                buf.as_mut()
            );

            // Request a page flip. The actual page flip will happen
            // some time later. We cannot call this again until we
            // have received the page flip event, but the page flip is
            // handled for us.
            crtc::page_flip(
                &self.card,
                crtc.handle(),
                fb,
                &[crtc::PageFlipFlags::PageFlipEvent]
            ).expect("Could not set CRTC");

            self.wait_for_vsync();
        }
    }

    fn wait_for_vsync(&self) {
        let mut fds = FdSet::new();
        fds.insert(self.card.as_raw_fd());

        loop {
            if let Ok(nfds) = select(None, Some(&mut fds), None, None, None) {
                if nfds > 0 {
                    // if we get here, it's safe to extract events
                    // from the fd.
                    if let Ok(events) = crtc::receive_events(&self.card) {
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

    fn create_fb(&self, mode: Mode) -> (
        framebuffer::Info,
        dumbbuffer::DumbBuffer
    ) {
        // Select the pixel format
        let fmt = PixelFormat::XRGB8888;
        //let fmt = PixelFormat::RGBA8888;
        //let fmt = PixelFormat::ARGB4444;
        let resolution = mode.size();

        // Create a DB
        let mut db = dumbbuffer::DumbBuffer::create_from_device(
            &self.card,
            (resolution.0 as u32, resolution.1 as u32),
            fmt
        ).expect("Could not create dumb buffer");

        let fb = framebuffer::create(&self.card, &db)
            .expect("Could not create FB");

        (fb, db)
    }
}


pub fn render(value: u8, buffer: &mut [u8]) {
    for b in buffer {
        *b = value;
    }
}


pub fn drm_magic() -> () {
    let card = Card::open("/dev/dri/card0");
    let display = Display::new(card);

    display.render_loop();
}
