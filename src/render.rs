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

// Cairo rendering implementation
use std::fs::File;
use std::f64::consts::PI;

use cairo;
use cairo::{Context, Format, ImageSurface, LineCap};

use crate::config;
use crate::config::{
    // Bounds,
    Color,
    // Config,
    Divisions,
    Gauge,
    GaugeStyle,
    GaugeType,
    Label,
    // Lamp,
    Pattern,
    Scale,
    Screen,
    Style,
    // StyleSet,
    // Unit
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
        self.set_pattern(cr, &self.default_style.background);
        cr.paint();

        for gauge in &self.pages[self.page] {
            cr.save();
            self.render_gauge(cr, &gauge, state);
            cr.restore();
        }
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

    fn center_text(&self, cr: &Context, text: &str) {
        let extents = cr.text_extents(text);
        cr.rel_move_to(-extents.width / 2.0, 0.0);
        cr.show_text(text)
    }

    fn center_label(&self, cr: &Context, label: &Label) {
        cr.save();
        match label {
            Label::None => (),
            Label::Plain(text) => self.center_text(cr, text),
            Label::Sized(text, sz) => {
                cr.set_font_size(*sz as f64);
                self.center_text(cr, text);
            },
            Label::Styled(text, sz, color) => {
                cr.set_font_size(*sz as f64);
                self.set_color(cr, color);
                self.center_text(cr, text)
            }
        }
        cr.restore();
    }

    fn render_gauge(
        &self,
        cr: &Context,
        gauge: &Gauge,
        state: &State
    ) {
        match &gauge.kind  {
            GaugeType::Dial(opts)            =>
                self.dial(cr, gauge, opts, state),
            GaugeType::VerticalBar(opts)     => {println!("{:?}", opts)},
            GaugeType::HorizontalBar(opts)   => {println!("{:?}", opts)},
            GaugeType::VerticalWedge(opts)   => {println!("{:?}", opts)},
            GaugeType::HorizontalWedge(opts) => {println!("{:?}", opts)},
            GaugeType::IdiotLight(l)         => {println!("{:?}", l)},
            GaugeType::Text(f, s)            => {println!("{:?}", s)}
        }
    }

    fn dial(
        &self,
        cr: &Context,
        gauge: &Gauge,
        scale: &Scale,
        state: &State
    ) {
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
        cr.move_to(0.0, -radius * 0.15);

        self.center_label(cr, &gauge.label);

        let (major, minor) = match &scale.2 {
            Divisions::None => (None, None),
            Divisions::Uniform(min) => (None, Some(min)),
            Divisions::MajorMinor(maj, min) => (Some(maj), Some(min))
        };

        cr.set_line_cap(LineCap::Round);

        if let Some(ticks) = minor {
            cr.save();
            cr.set_line_width(5.0);
            for tick in ticks {
                cr.save();
                cr.rotate(scale.to_angle(*tick).into());
                cr.move_to(0.0, -radius * 0.95);
                cr.line_to(0.0, -radius * 0.75);
                cr.restore();
            }
            cr.stroke();
            cr.restore();
        }

        if let Some(ticks) = major {
            cr.save();
            cr.set_line_width(10.0);
            for (_, value) in ticks {
                cr.save();
                cr.rotate(scale.to_angle(*value).into());
                cr.move_to(0.0, -radius * 0.95);
                cr.line_to(0.0, -radius * 0.70);
                cr.restore();
            }

            cr.stroke();

            cr.set_font_size(24.0);
            for (label, value) in ticks {
                cr.save();
                cr.rotate(scale.to_angle(*value).into());
                cr.line_to(0.0, -radius * 0.50);
                self.center_label(cr, label);
                cr.restore();
            }
            cr.restore();
        }

        if self.set_indicator(cr, gauge, state) {
            if let Some(value) = state.get(&gauge.channel) {
                cr.rotate(scale.to_angle(value).into());
                cr.move_to(-10.0, 0.0);
                cr.rel_line_to(0.0, -radius);
                cr.line_to(10.0, 0.0);
                cr.close_path();
                cr.fill();
            }

            cr.arc(0.0, 0.0, radius / 10.0, 0.0, 2.0 * PI);
            cr.fill();
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

        surface.write_to_png(&mut file).unwrap();
    }
}
