use crate::render::CairoRenderer;
use crate::data::{ReadSource, DataSource, State};
use crate::clock::Clock;
use crate::config::Screen;
#[macro_use]
use crate::util;

use gtk::prelude::*;
use gtk::*;
use cairo::*;
use std::io::stdin;
use std::process;

// Render the entire UI.
fn draw(cr: &Context, time: f64) {
    let s = 5.0 + 5.0 * time.sin();
    cr.set_source_rgb(1.0, 1.0, 1.0);
    cr.paint();

    cr.set_source_rgb(1.0, 0.0, 0.0);

    cr.save();
    cr.scale(s, s);
    cr.rectangle(100.0, 100.0, 100.0, 100.0);
    cr.restore();
    cr.stroke();
}


pub fn run(screen: Screen, renderer: CairoRenderer) {
    if gtk::init().is_err() {
        eprintln!("Failed to initialize GTK!");
        process::exit(1);
    }

    let data = ReadSource::new(stdin());
    let clock = Clock::new();
    let window = Window::new(WindowType::Toplevel);
    let da = DrawingArea::new();

    window.add(&da);
    window.set_title("Hello, world!");
    window.show_all();
    // XXX: pixel densities vary, we should be using DPI information
    window.set_size_request(screen.width as i32, screen.height as i32);

    window.connect_delete_event(move |_, _| {
        main_quit();
        Inhibit(false)
    });

    da.connect_draw(move |_, cr| {
        renderer.render(cr, &data.get_state());
        Inhibit(true)
    });

    gtk::timeout_add(50, move || {
        da.queue_draw();
        Continue(true)
    });

    gtk::main();
}
