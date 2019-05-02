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
    Bounds,
    Color,
    // Config,
    Divisions,
    Float,
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
                cr.set_font_size(*sz);
                self.center_text(cr, text);
            },
            Label::Styled(text, sz, color) => {
                cr.set_font_size(*sz);
                self.set_color(cr, color);
                self.center_text(cr, text)
            }
        }
        cr.restore();
    }

    // XXX: better name for this
    fn show_outline(
        &self,
        cr: &Context,
        gauge: &Gauge,
        style: GaugeStyle,
        state: &State
    ) {
        if self.set_foreground(cr, gauge, state) {
            match style {
                GaugeStyle::IndicatorOnly => (),
                GaugeStyle::Outline => cr.stroke(),
                GaugeStyle::Filled => cr.fill(),
                GaugeStyle::Dashed => cr.fill(), // xxx not implemented
            }
        }
    }

    fn rounded_rect(cr: &Context, bounds: &Bounds, radius: Float) {
        let centers = bounds.inset(radius);
        let y1 = bounds.y;
        let y2 = bounds.y + bounds.height;
        let x2 = bounds.x + bounds.width;
        let c1 = centers.top_left();
        let c2 = centers.top_right();
        let c3 = centers.bottom_right();
        let c4 = centers.bottom_left();
        cr.new_path();
        cr.arc(c1.0, c1.1, radius, PI, PI * 1.5);
        cr.line_to(c2.0, y1);
        cr.arc(c2.0, c2.1, radius, PI * 1.5, 0.0);
        cr.line_to(x2, c3.1);
        cr.arc(c3.0, c3.1, radius, 0.0, PI * 0.5);
        cr.line_to(c4.0, y2);
        cr.arc(c4.0, c4.1, radius, PI * 0.5, PI);
        cr.close_path();
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
            GaugeType::VerticalBar(opts)     =>
                self.vertical_bar(cr, gauge, opts, state),
            GaugeType::HorizontalBar(opts)   =>
                self.horizontal_bar(cr, gauge, opts, state),
            GaugeType::VerticalWedge(opts)   => {println!("{:?}", opts)},
            GaugeType::HorizontalWedge(opts) => {println!("{:?}", opts)},
            GaugeType::IdiotLight(l)         => {println!("{:?}", l)},
            GaugeType::Text(f, s)            => {println!("{:?} {:?}", f, s)}
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
        let (cx, cy) = bounds.center();
        let radius = bounds.radius();

        // Render main gauge background
        cr.translate(cx, cy);
        cr.arc(0.0, 0.0, radius, 0.0, 2.0 * PI);
        self.show_outline(cr, gauge, scale.3, state);

        // Render label, offset from center
        cr.move_to(0.0, -radius * 0.15);
        self.center_label(cr, &gauge.label);

        // Render divisions
        let (major, minor) = match &scale.2 {
            Divisions::None => (None, None),
            Divisions::Uniform(min) => (None, Some(min)),
            Divisions::MajorMinor(maj, min) => (Some(maj), Some(min))
        };

        cr.set_line_cap(LineCap::Round);
        self.set_background(cr, gauge, state);

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
                cr.rotate(scale.to_angle(*value));
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
                cr.rotate(-scale.to_angle(*value));
                self.center_label(cr, label);
                cr.restore();
            }
            cr.restore();
        }

        // Render the indicator.
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

    fn vertical_bar(
        &self,
        cr: &Context,
        gauge: &Gauge,
        scale: &Scale,
        state: &State
    ) {
        let bounds = gauge.bounds;
        let corner_radius = 5.0;

        Self::rounded_rect(cr, &bounds, corner_radius);
        self.show_outline(cr, gauge, scale.3, state);

        if self.set_indicator(cr, gauge, state) {
            if let Some(value) = state.get(&gauge.channel) {
                let bounds = bounds.inset(1.0);
                cr.save();
                let fill = bounds.height * (1.0 - scale.to_percent(value));
                cr.rectangle(
                    bounds.x,
                    bounds.y + fill,
                    bounds.width,
                    bounds.height - fill
                );
                cr.clip();

                Self::rounded_rect(cr, &bounds, corner_radius);
                cr.fill();
                cr.restore();
            }
        }

        self.set_background(cr, gauge, state);
        let (cx, cy) = bounds.center();
        cr.move_to(cx, cy);
        self.center_label(cr, &gauge.label);
    }

    fn horizontal_bar(
        &self,
        cr: &Context,
        gauge: &Gauge,
        scale: &Scale,
        state: &State
    ) {
        let bounds = gauge.bounds;
        let corner_radius = 5.0;

        Self::rounded_rect(cr, &bounds, corner_radius);
        self.show_outline(cr, gauge, scale.3, state);

        if self.set_indicator(cr, gauge, state) {
            if let Some(value) = state.get(&gauge.channel) {
                let bounds = bounds.inset(1.0);
                cr.save();
                cr.rectangle(
                    bounds.x,
                    bounds.y,
                    bounds.width * scale.to_percent(value),
                    bounds.height
                );
                cr.clip();

                Self::rounded_rect(cr, &bounds, corner_radius);
                cr.fill();
                cr.restore();
            }
        }

        self.set_background(cr, gauge, state);
        let (cx, cy) = bounds.center();
        cr.move_to(cx, cy);
        self.center_label(cr, &gauge.label);
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
