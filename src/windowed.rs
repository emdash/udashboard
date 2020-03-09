use crate::render::CairoRenderer;
use crate::data::{State, DataSource};
use crate::clock::Clock;
use crate::config::Screen;

use gtk::prelude::*;
use gtk::*;
use cairo::*;
use std::process;
use std::rc::Rc;
use std::time::Instant;


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


pub fn run<DS>(
    screen: Screen,
    renderer: CairoRenderer,
    data: DS
) where DS:DataSource {
    if gtk::init().is_err() {
        eprintln!("Failed to initialize GTK!");
        process::exit(1);
    }

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

    da.connect_draw(move |_, c| {
        draw(c, clock.seconds());
        Inhibit(true)
    });

    gtk::idle_add(move || {
        da.queue_draw();
        Continue(true)
    });

    gtk::main();
}
