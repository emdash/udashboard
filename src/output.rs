use crate::clock::Clock;
use crate::render::CairoRenderer;
use crate::data::State;

use std::{
    borrow::Borrow,
    collections::HashMap,
    format,
    fs::{OpenOptions, File},
    os::unix::io::{
        RawFd,
        AsRawFd
    }
};

use cairo::{Context, Format, ImageSurface};
use cairo_sys as ffi;

use drm::{
    Device as BasicDevice,
    buffer::{Buffer, PixelFormat},
    control::{
        Device as ControlDevice,
        Mode,
        ResourceHandle,
        ResourceInfo,
        connector,
        crtc,
        dumbbuffer::{DumbBuffer},
        framebuffer::{
            Handle as FrameBufferHandle,
            create as createfb
        }
    }
};

use nix::sys::select::{FdSet, select};


// Mode reports size as (u16, u16), but every place we use it wants
// (u32, u32) which is _maddening_
fn widen<T: Into<U>, U>(a: (T, T)) -> (U, U) {
    (a.0.into(), a.1.into())
}


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


struct Page {
    pub fb: FrameBufferHandle,
    pub db: Box<DumbBuffer>
}

impl Page {
    pub fn new(card: &Card, mode: &Mode) -> Page {
        // This is the only format that seems to work...
        let fmt = PixelFormat::RGB565;
        let sz = mode.size();
        let db = Box::new(DumbBuffer::create_from_device(card, widen(sz), fmt).expect("!"));
        let fb = createfb(card, &(*db)).expect("!").handle();
        Page {fb, db}
    }
}


// Run forever, redrawing the screen as fast as possible, using
// double-buffering.
fn render(card: Card, renderer: CairoRenderer) {
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
    // Create a Page struct for reach buffer.
    let mut p1 = Page::new(&card, &mode);
    let mut p2 = Page::new(&card, &mode);
    let (w, h) = p1.db.size();
    let pitch = p1.db.pitch();

    let mut dm1 = p1.db.map(&card).expect("!");
    let mut dm2 = p2.db.map(&card).expect("!");

    // XXX: @u$)(@#@ ImageSurface::create_for_data() requires
    // 'static!!?!? So we have to use unsafe even though we can
    // statically prove that dm lives longer than the context.
    let ptr1 = dm1.as_mut().as_mut_ptr();
    let ptr2 = dm2.as_mut().as_mut_ptr();

    let c1 = unsafe {
        let surface = ImageSurface::from_raw_full(
            ffi::cairo_image_surface_create_for_data(
                ptr1,
                Format::Rgb16_565.into(),
                w as i32,
                h as i32,
                pitch as i32
            )
        ).expect("!");
        Context::new(&surface)
    };
    let c2 = unsafe {
        let surface = ImageSurface::from_raw_full(
            ffi::cairo_image_surface_create_for_data(
                ptr2,
                Format::Rgb16_565.into(),
                w as i32,
                h as i32,
                pitch as i32
            )
        ).expect("!");
        Context::new(&surface)
    };

    let cr = [(p1.fb, c1), (p2.fb, c2)];
    let clock = Clock::new();
    let pf_flags = [crtc::PageFlipFlags::PageFlipEvent];
    let con_hdl = [connector.handle()];
    let orig = (0, 0);

    let mut state = State {
        values: HashMap::new(),
        states: HashMap::new(),
        time: 0
    };

    state.values.insert("RPM".to_string(), 1500.0 as f32);

    // Set initial mode on the crtc.
    crtc::set(&card, crtc.handle(), p1.fb, &con_hdl, orig, Some(mode))
        .expect("Could not set CRTC");

    for (fb, cr) in cr.iter().cycle() {
        let val = (0.5 * ((clock.seconds() * 2.0).sin() + 1.0)) as f32;
        state.values.insert("RPM".to_string(), 1500.0 * val);

        // Fill the buffers with values.
        renderer.render(cr, &state);

        // Request a page flip. The actual page flip will happen
        // some time later. We cannot call this again until we
        // have received the page flip event, but the page flip is
        // handled for us.
        crtc::page_flip(&card, crtc.handle(), *fb, &pf_flags)
            .expect("Could not set CRTC");

        await_vblank(&card);
    }
}


pub fn run(renderer: CairoRenderer) -> () {
    render(Card::open("/dev/dri/card0"), renderer);
}
