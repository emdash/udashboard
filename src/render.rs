// Cairo rendering implementation
use std::fs::File;
use std::f64::consts::PI;

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
    Scale,
    Color,
    Screen,
    Style,
    StyleSet,
    Unit
};

use crate::data::State;

fn pattern_from_color(color: Color) -> cairo::Pattern {
    let ret = cairo::SolidPattern::from_rgba(
        color.0.into(),
        color.1.into(),
        color.2.into(),
        color.3.into()
    );

    cairo::Pattern::SolidPattern(ret)
}

#[derive(Debug)]
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
            config::Pattern::Solid(x) => {
                Pattern::Solid(pattern_from_color(x))
            },
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
            Pattern::Solid(x) => {
                cr.set_source(&x)
            },
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
        let label = &gauge.label;
        let kind = &gauge.kind;
        let value = state.values.get(&gauge.channel);
        let bounds = &gauge.bounds;
        // XXX: get style

        println!("{:?}", bounds);

        match kind {
            GaugeType::Dial(opts) => self.dial(cr, bounds, opts, label, value),
            GaugeType::VerticalBar(opts) => {println!("{:?}", opts)}
            GaugeType::HorizontalBar(opts) => {println!("{:?}", opts)}
            GaugeType::VerticalWedge(opts) => {println!("{:?}", opts)}
            GaugeType::HorizontalWedge(opts) => {println!("{:?}", opts)}
            GaugeType::IdiotLight(l) => {println!("{:?}", l)},
            GaugeType::Text(f, s) => {println!("{:?}", s)}
        }
    }

    fn dial(
        &self,
        cr: &Context,
        bounds: &Bounds,
        scale: &Scale,
        label: &String,
        value: Option<&f32>
    ) {
        cr.save();

        let radius = (bounds.width.min(bounds.height) / 2.0) as f64;
        let cx = (bounds.x + bounds.width / 2.0) as f64;
        let cy = (bounds.y + bounds.height / 2.0) as f64;

        cr.set_source_rgb(1.0, 0.0, 0.0);
        cr.arc(cx, cy, radius, 0.0, 2.0 * PI);

        match scale.3 {
            GaugeStyle::IndicatorOnly => (),
            GaugeStyle::Outline => cr.stroke(),
            GaugeStyle::Filled => cr.fill(),
            GaugeStyle::Dashed => cr.fill(), // xxx not implemented
        }

        cr.restore();
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

        surface.write_to_png(&mut file).unwrap();
    }
}
