// Cairo rendering implementation
use std::fs::File;
use std::f64::consts::PI;

use cairo;
use cairo::{Context, Format, ImageSurface};

use crate::config;
use crate::config::{
    Bounds,
    Color,
    Config,
    Divisions,
    Gauge,
    GaugeStyle,
    GaugeType,
    Lamp,
    Pattern,
    Scale,
    Screen,
    Style,
    StyleSet,
    Unit
};

use crate::data::State;

pub struct CairoRenderer {
    screen: Screen,
    pages: Vec<Vec<Gauge>>,
    default_style: Style,
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
            default_style: default_style,
            page: 0
        }
    }

    pub fn render(
        &self,
        cr: &Context,
        state: &State
    ) {
        cr.save();

        self.set_pattern(cr, self.default_style.background);
        cr.paint();

        for gauge in &self.pages[self.page] {
            self.render_gauge(cr, &gauge, state);
        }

        cr.restore();
    }

    fn set_color(&self, cr: &Context, color: Color) {
        cr.set_source_rgba(
            color.0.into(),
            color.1.into(),
            color.2.into(),
            color.3.into()
        );
    }

    fn set_pattern(&self, cr: &Context, pat: Pattern) {
        match pat {
            Pattern::Hidden => cr.set_source_rgba(0.0, 0.0, 0.0, 0.0),
            Pattern::Solid(c) => self.set_color(cr, c),
            Pattern::SlowBlink(c) => self.set_color(cr, c),
            Pattern::FastBlink(c) => self.set_color(cr, c)
        }
    }

    fn render_gauge(
        &self,
        cr: &Context,
        gauge: &Gauge,
        state: &State
    ) {
        let kind = &gauge.kind;

        match kind {
            GaugeType::Dial(opts) => self.dial(cr, gauge, opts, state),
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
        gauge: &Gauge,
        scale: &Scale,
        state: &State
    ) {
        cr.save();

        let bounds = gauge.bounds;
        let radius = (bounds.width.min(bounds.height) / 2.0) as f64;
        let cx = (bounds.x + bounds.width / 2.0) as f64;
        let cy = (bounds.y + bounds.height / 2.0) as f64;

        self.set_pattern(cr, self.default_style.foreground);
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
