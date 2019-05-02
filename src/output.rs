// uDashBoard: featherweight dashboard application.
//
// Copyright (C) 2019  Brandon Lewis
//
// This program is free software: you can redistribute it and/or
// modify it under the terms of the GNU Lesser General Public License
// as published by the Free Software Foundation, either version 3 of
// the License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU
// Lesser General Public License for more details.
//
// You should have received a copy of the GNU Lesser General Public
// License along with this program.  If not, see
// <https://www.gnu.org/licenses/>.

use crate::clock::Clock;
use crate::render::CairoRenderer;
use crate::data::State;

use std::{
    cell::RefCell,
    collections::HashMap,
    format,
    fs::{OpenOptions, File},
    os::unix::io::{
        RawFd,
        AsRawFd
    }
};

use cairo::{Context, Format, ImageSurface};

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

const PFFLAGS: [crtc::PageFlipFlags; 1] = [crtc::PageFlipFlags::PageFlipEvent];


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
        let nfds = select(None, Some(&mut fds), None, None, None)
            .expect("select failed");
        if nfds > 0 {
            // if we get here, it's safe to extract events
            // from the fd.
            let events = crtc::receive_events(card)
                .expect("couldn't receive events.");

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


struct Page {
    pub fb: FrameBufferHandle,
    pub db: RefCell<DumbBuffer>
}

impl Page {
    pub fn new(card: &Card, mode: &Mode) -> Page {
        // This is the only format that seems to work...
        let fmt = PixelFormat::RGB565;
        let sz = mode.size();
        let mut db = RefCell::new(
            DumbBuffer::create_from_device(
                card,
                widen(sz),
                fmt
            ).expect("!")
        );

        let fb = createfb(card, db.get_mut()).expect("!").handle();
        Page {fb, db}
    }

    fn get_image_surface(&self) -> ImageSurface {
        let db = self.db.borrow();
        let (width, height) = db.size();
        let (width, height) = (width as i32, height as i32);
        let stride = db.pitch() as i32;
        let format = Format::Rgb16_565;

        if format.stride_for_width(width as u32) == Ok(stride) {
            ImageSurface::create(
                Format::Rgb16_565,
                width as i32,
                height as i32
            )
        } else {
            let size = (height as usize) * (stride as usize);
            let buffer = vec![0; size];
            ImageSurface::create_for_data(
                buffer,
                format,
                width,
                height,
                stride
            )
        }.expect("couldn't create surface")
    }

    fn render_priv(
        &self,
        surface: &ImageSurface,
        state: &State,
        renderer: &CairoRenderer
    ) {
        let cr = Context::new(&surface);
        renderer.render(&cr, &state);
    }

    pub fn render(
        &self,
        card: &Card,
        renderer: &CairoRenderer,
        crtc: crtc::Handle,
        state: &State
    ) {
        // I tried so hard to optimize this code to re-use the
        // dumbbuffer, mapping, and cairo context. It worked fine on
        // my laptop. But when I got the BBB, it brought the whole
        // system down repeatedly. I strongly suspect that cairo's
        // rasterizer commits access violations, and while this is
        // normally harmless, it's death when you're dealing with
        // shared memory and frame buffers. I will revisit this if /
        // when framerate becomes an issue.

        let mut s = self.get_image_surface();
        let mut db = self.db.borrow_mut();

        // XXX: if we can't avoid the memcpy anyway, is it possible /
        // better to *write* to the framebuffer?
        let mut dm = db.map(card).expect("couldn't map buffer");
        self.render_priv(&s, state, renderer);

        dm.as_mut().copy_from_slice(
            s.get_data().expect("couldn't borrow image data").as_mut()
        );

        crtc::page_flip(card, crtc, self.fb, &PFFLAGS)
            .expect("Could not set CRTC");

        // XXX: This blocks until the page flip occurs, which could be
        // a relatively long time. Revisit this if / when framerate
        // becomes an issue.
        await_vblank(&card);
    }
}


// Loop forever rendering things al the things.
fn render_loop(
    card: Card,
    crtc: crtc::Handle,
    renderer: CairoRenderer,
    pages: [Page; 2]
) {
    let clock = Clock::new();

    let mut state = State {
        values: HashMap::new(),
        states: HashMap::new(),
        time: 0
    };

    state.values.insert("RPM".to_string(), 1500.0);

    let start = clock.seconds();
    for page in pages.iter().cycle() {
        let time = clock.seconds() - start;
        let val = 0.5 * time.sin() + 0.5;
        state.values.insert("RPM".to_string(), 6500.0 * val);
        state.values.insert("OIL_PRESSURE".to_string(), 60.0 * val);
        state.values.insert("ECT".to_string(), 230.0 * val);
        state.values.insert("SESSION_TIME".to_string(), time);
        state.values.insert("GEAR".to_string(), 1.0 + 5.0 * val);
        page.render(&card, &renderer, crtc, &state);
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
    let pages = [Page::new(&card, &mode), Page::new(&card, &mode)];
    let con_hdl = [connector.handle()];
    let orig = (0, 0);

    // Set initial mode on the crtc.  Set this to the back buffer,
    // because we will start rendering into the front buffer.
    crtc::set(&card, crtc.handle(), pages[0].fb, &con_hdl, orig, Some(mode))
        .expect("Could not set CRTC");

    render_loop(card, crtc.handle(), renderer, pages);
}


// Entry point for rendering.
pub fn run(renderer: CairoRenderer, device: String) -> () {
    render(Card::open(&device), renderer);
}
