// Cairo rendering implementation
use std::fs::File;

use cairo;
use cairo::{Context, Format, ImageSurface};

use crate::config;
use crate::config::{
    Bounds,
    Config,
    Divisions,
    Gauge,
    GaugeStyle,
    GaugeType,
    Lamp,
    Range,
    Color,
    Screen,
    Style,
    StyleSet,
    Unit
};

use crate::data::State;

fn pattern_from_color(color: Color) -> cairo::Pattern {
    cairo::Pattern::SolidPattern(cairo::SolidPattern::from_rgba(
        color.0.into(),
        color.1.into(),
        color.2.into(),
        color.3.into()
    ))
}

enum Pattern {
    Hidden,
    Solid(cairo::Pattern),
    SlowBlink(cairo::Pattern),
    FastBlink(cairo::Pattern)
}

impl Pattern {
    pub fn new(config: config::Pattern) -> Pattern {
        match config {
            config::Pattern::Hidden => Pattern::Hidden,
            config::Pattern::Solid(x) =>
                Pattern::Solid(pattern_from_color(x)),
            config::Pattern::SlowBlink(x) =>
                Pattern::SlowBlink(pattern_from_color(x)),
            config::Pattern::FastBlink(x) =>
                Pattern::FastBlink(pattern_from_color(x))
        }
    }

    pub fn set_source(
        &self,
        cr: &Context,
        null: &cairo::Pattern,
        time: u64
    ) {
        let slow_blink = (time / 1000) % 2 == 1;
        let fast_blink = (time / 250) % 2 == 1;
        match self {
            Pattern::Hidden => cr.set_source(null.into()),
            Pattern::Solid(x) => cr.set_source(&x),
            Pattern::SlowBlink(x) => if slow_blink {
                cr.set_source(&x);
            } else {
                cr.set_source(null);
            },
            Pattern::FastBlink(x) => if fast_blink {
                cr.set_source(&x);
            } else {
                cr.set_source(null);
            }
        }
    }
}

pub struct CairoRenderer {
    screen: Screen,
    pages: Vec<Vec<Gauge>>,
    background: Pattern,
    foreground: Pattern,
    indicator: Pattern,
    null: cairo::Pattern,
    page: usize
}

impl CairoRenderer {
    pub fn new(
        screen: Screen,
        pages: Vec<Vec<Gauge>>,
        default_style: Style,
    ) -> CairoRenderer {
        CairoRenderer {
            screen: screen,
            pages: pages,
            background: Pattern::new(default_style.background),
            foreground: Pattern::new(default_style.foreground),
            indicator: Pattern::new(default_style.indicator),
            null: cairo::Pattern::SolidPattern(
                cairo::SolidPattern::from_rgba(0.0, 0.0, 0.0, 0.0)
            ),
            page: 0
        }
    }

    pub fn render(
        &self,
        cr: &Context,
        state: &State
    ) {
        self.background.set_source(cr, &self.null, state.time);
        cr.paint();

        for gauge in &self.pages[self.page] {
            self.render_gauge(cr, &gauge, state);
        }
    }

    fn render_gauge(
        &self,
        cr: &Context,
        gauge: &Gauge,
        state: &State
    ) {
        match &gauge.kind {
            GaugeType::Dial(r, d, s) => {println!("{:?}", (r, d, s))}
            GaugeType::VerticalBar(r, d, s) => {println!("{:?}", (r, d, s))}
            GaugeType::HorizontalBar(r, d, s) => {println!("{:?}", (r, d, s))}
            GaugeType::VerticalWedge(r, d, s) => {println!("{:?}", (r, d, s))}
            GaugeType::HorizontalWedge(r, d, s) => {println!("{:?}", (r, d, s))}
            GaugeType::IdiotLight(l) => {println!("{:?}", l)},
            GaugeType::Text(f, s) => {println!("{:?}", s)}
        }
    }
}


pub struct PNGRenderer {
    renderer: CairoRenderer,
    path: String,
    screen: Screen
}

impl PNGRenderer {
    pub fn new(
        path: String,
        screen: Screen,
        pages: Vec<Vec<Gauge>>,
        default_style: Style
    ) -> PNGRenderer {
        PNGRenderer {
            renderer: CairoRenderer::new(
                screen,
                pages,
                default_style,
            ),
            path: path,
            screen: screen
        }
    }

    pub fn render(&self, state: &State) {
        let surface = ImageSurface::create(
            Format::ARgb32,
            self.screen.width as i32,
            self.screen.height as i32
        ).expect("Couldn't create surface.");
        let cr = Context::new(&surface);
        self.renderer.render(&cr, state);
        let mut file = File::create(self.path.clone())
            .expect("couldn't create file");
        surface.write_to_png(&mut file);
    }
}
