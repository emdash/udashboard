use crate::render::CairoRenderer;
use crate::data::{ReadSource, DataSource};
use crate::clock::Clock;
use crate::config::Screen;


use gtk::prelude::*;
use gtk::*;
use std::io::stdin;
use std::process;


pub fn run(screen: Screen, renderer: CairoRenderer) {
    if gtk::init().is_err() {
        eprintln!("Failed to initialize GTK!");
        process::exit(1);
    }

    let data = ReadSource::new(stdin());
    let _clock = Clock::new();
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
