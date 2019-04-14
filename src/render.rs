// Cairo rendering implementation
use std::fs::File;
use std::f64::consts::PI;

use cairo;
use cairo::{Context, Format, ImageSurface, TextExtents};

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

        self.set_pattern(cr, &self.default_style.background);
        cr.paint();

        for gauge in &self.pages[self.page] {
            self.render_gauge(cr, &gauge, state);
        }

        cr.restore();
    }

    fn set_color(&self, cr: &Context, color: &Color) {
        cr.set_source_rgba(
            color.0.into(),
            color.1.into(),
            color.2.into(),
            color.3.into()
        );
    }

    fn set_pattern(&self, cr: &Context, pat: &Pattern) -> bool{
        match pat {
            Pattern::Hidden => {return false},
            Pattern::Solid(c) => self.set_color(cr, c),
            Pattern::SlowBlink(c) => self.set_color(cr, c),
            Pattern::FastBlink(c) => self.set_color(cr, c)
        }

        true
    }

    fn get_style(&self, g: &Gauge, _state: &State) -> Style {
        // XXX: do actual lookup
        let state = &config::State::Default;
        let style = g.styles.get(state);
        *(style).unwrap()
    }

    fn set_background(&self, cr: &Context, g: &Gauge, state: &State) -> bool {
        self.set_pattern(cr, &self.get_style(g, state).background)
    }

    fn set_foreground(&self, cr: &Context, g: &Gauge, state: &State) -> bool {
        self.set_pattern(cr, &self.get_style(g, state).foreground)
    }

    fn set_indicator(&self, cr: &Context, g: &Gauge, state: &State) -> bool {
        self.set_pattern(cr, &self.get_style(g, state).indicator)
    }

    fn render_gauge(
        &self,
        cr: &Context,
        gauge: &Gauge,
        state: &State
    ) {
        match &gauge.kind  {
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

        cr.translate(cx, cy);

        if self.set_foreground(cr, gauge, state) {
            cr.arc(0.0, 0.0, radius, 0.0, 2.0 * PI);
            match scale.3 {
                GaugeStyle::IndicatorOnly => (),
                GaugeStyle::Outline => cr.stroke(),
                GaugeStyle::Filled => cr.fill(),
                GaugeStyle::Dashed => cr.fill(), // xxx not implemented
            }
        }

        self.set_pattern(cr, &self.default_style.background);
        cr.set_font_size(14.0);

        let extents = cr.text_extents(&gauge.label);
        cr.move_to(-extents.width / 2.0, radius * 0.15 + extents.height);
        println!("Got here");
        cr.show_text(&gauge.label);

        if self.set_indicator(cr, gauge, state) {
            if let Some(value) = state.get(&gauge.channel) {
                let range = scale.0 - scale.1;
                let angle = 2.0 * PI * (((value - scale.0) / range) as f64);

                cr.new_path();
                cr.rotate(angle);
                cr.move_to(-5.0, 0.0);
                cr.rel_line_to(0.0, -radius);
                cr.line_to(5.0, 0.0);
                cr.close_path();
                cr.fill();
            }

            cr.arc(0.0, 0.0, radius / 10.0, 0.0, 2.0 * PI);
            cr.fill();
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
