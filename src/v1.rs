// Defines file format for config file syntax version 1.

use ron::de::from_reader;
use std::{
    collections::HashMap,
    fs::File
};
use serde::{Deserialize};

use crate::config;
use crate::config::{
    Bounds,
    Channel,
    Config,
    GaugeType,
    Screen,
    State,
    Logic
};

#[derive(Deserialize, Debug, Copy, Clone)]
enum Color {
    Clear,
    Black,
    White,
    Grey,
    Red,
    Orange,
    Yellow,
    Green,
    Blue,
    Violet,
    RGBA(f32, f32, f32, f32)
}

impl Color {
    pub fn to_config(self) -> config::Color {
        match self {
            Color::Clear  => config::Color(0.0, 0.0, 0.0, 0.0),
            Color::Black  => config::Color(0.0, 0.0, 0.0, 1.0),
            Color::White  => config::Color(1.0, 1.0, 1.0, 1.0),
            Color::Grey   => config::Color(0.7, 0.7, 0.7, 1.0),
            Color::Red    => config::Color(1.0, 0.0, 0.0, 1.0),
            Color::Orange => config::Color(1.0, 0.5, 0.0, 1.0),
            Color::Yellow => config::Color(0.0, 1.0, 1.0, 1.0),
            Color::Green  => config::Color(0.0, 1.0, 0.0, 1.0),
            Color::Blue   => config::Color(0.0, 0.0, 1.0, 1.0),
            Color::Violet => config::Color(1.0, 0.0, 1.0, 1.0),
            Color::RGBA(r, g, b, a) => config::Color(r, g, b, a)
        }
    }
}

#[derive(Deserialize, Debug)]
enum Pattern {
    Inherit,
    Solid(Color),
    SlowBlink(Color),
    FastBlink(Color)
}

impl Pattern {
    pub fn to_config(&self, inherited: config::Pattern) -> config::Pattern {
        match self {
            Pattern::Inherit => inherited,
            Pattern::Solid(x) => config::Pattern::Solid(x.to_config()),
            Pattern::SlowBlink(x) => config::Pattern::SlowBlink(x.to_config()),
            Pattern::FastBlink(x) => config::Pattern::FastBlink(x.to_config())
        }
    }
}

#[derive(Deserialize, Debug)]
struct Style {
    background: Pattern,
    foreground: Pattern,
    indicator: Pattern
}

impl Style {
    pub fn to_config(&self, inheritted: config::Style) -> config::Style {
        config::Style {
            background: self.background.to_config(inheritted.background),
            foreground: self.foreground.to_config(inheritted.foreground),
            indicator: self.indicator.to_config(inheritted.indicator)
        }
    }
}

type StyleSet = HashMap<State, String>;


fn to_config_styleset(styles: StyleSet, defs: &StyleDefs) -> config::StyleSet {
    let mut ret = config::StyleSet::new();
    let default = config::Style::default();
    let default = defs.get(&StyleId::Default).unwrap().to_config(default);

    ret.insert(State::Default, default);

    for (state, id) in styles {
        // xxx: unwrap is bad smell here, error handling!
        let s = defs.get(&StyleId::Define(id)).unwrap().to_config(default);
        ret.insert(state, s);
    }

    ret
}


#[derive(Deserialize, Debug, Clone, Hash, PartialEq, Eq)]
enum StyleId {
    Default,
    Define(String)
}

type StyleDefs = HashMap<StyleId, Style>;

#[derive(Deserialize, Debug, Copy, Clone)]
enum Length {
    Pixel(f32),
    Percent(f32)
}

impl Length {
    pub fn to_pixels(self, total: f32) -> f32 {
        match self {
            Length::Pixel(x) => x,
            Length::Percent(x) => total * (x / 100.0)
        }
    }
}

#[derive(Deserialize, Debug, Copy, Clone)]
enum Size {
    Radius(Length),
    Box(Length, Length)
}

impl Size {
    pub fn to_config(self, screen: Screen) -> config::Point {
        match self {
            Size::Radius(r) => {
                let min = screen.width.min(screen.height);
                let r = r.to_pixels(min) / 2.0;
                config::Point { x: r, y: r}
            }, Size::Box(w, h) => {
                config::Point {
                    x: w.to_pixels(screen.width),
                    y: h.to_pixels(screen.height)
                }
            }
        }
    }
}

#[derive(Deserialize, Debug)]
struct Point {
    pub x: Length,
    pub y: Length
}

impl Point {
    pub fn to_config(&self, screen: Screen) -> config::Point {
        config::Point {
            x: Point::to_pixels(self.x, screen.width),
            y: Point::to_pixels(self.y, screen.height)
        }
    }

    fn to_pixels(value: Length, total: f32) -> f32 {
        match value {
            Length::Pixel(x) => x,
            Length::Percent(x) => (x / 100.0) * total
        }
    }
}

#[derive(Deserialize, Debug)]
struct GridSize(pub u32, pub u32);

#[derive(Deserialize, Debug)]
struct GridPosition(pub u32, pub u32);

#[derive(Deserialize, Debug)]
enum Layout {
    FromCenter(Point, Size),
    FromTopLeft(Point, Point),
    Grid(GridSize, GridPosition),
}

impl Layout {
    pub fn to_config(&self, screen: Screen) -> Bounds {
        match self {
            Layout::FromCenter(center, dim) => {
                let center = center.to_config(screen);
                let dim = dim.to_config(screen);
                Bounds {
                    x: center.x - dim.x / 2.0,
                    y: center.y - dim.y / 2.0,
                    width: dim.x,
                    height: dim.y
                }
            }, Layout::FromTopLeft(p1, p2) => {
                let p1 = p1.to_config(screen);
                let p2 = p2.to_config(screen);
                Bounds {
                    x: p1.x,
                    y: p1.y,
                    width: p2.x - p1.x,
                    height: p2.y - p1.y
                }
            }, Layout::Grid(size, pos) => {
                let cell_width = screen.width / size.1 as f32;
                let cell_height = screen.height / size.0 as f32;
                Bounds {
                    x: pos.1 as f32 * cell_width,
                    y: pos.0 as f32 * cell_height,
                    width: cell_width,
                    height: cell_height
                }
            }
        }
    }
}

#[derive(Deserialize, Debug)]
struct Gauge {
    name: String,
    label: Option<String>,
    kind: GaugeType,
    channel: String,
    layout: Layout,
    styles: StyleSet
}

impl Gauge {
    fn to_config(
        self,
        screen: Screen,
        style_defs: &StyleDefs
    ) -> config::Gauge {
        let Gauge {
            name,
            label,
            kind,
            channel,
            layout,
            styles
        } = self;

        config::Gauge {
            name: name,
            label: label,
            kind: kind,
            channel: channel,
            bounds: layout.to_config(screen),
            styles: to_config_styleset(styles, style_defs)
        }
    }
}

#[derive(Deserialize, Debug)]
struct Page(Vec<String>);

#[derive(Deserialize, Debug)]
struct V1 {
    width: u32,
    height: u32,
    channels: Vec<Channel>,
    conditions: Logic,
    gauges: Vec<Gauge>,
    pages: Vec<Page>,
    styles: StyleDefs
}

#[derive(Debug)]
pub enum V1Error {
    ReadError(String),
    ParseError(String),
    NoSuchChannel(String),
    NoSuchState(State),
    NoSuchGauge(String)
}

impl V1 {
    fn to_config(self) -> Config {
        let screen = Screen {
            width: self.width as f32,
            height: self.height as f32
        };

        let (pages, channels, conditions) = self.build_page_list(screen);

        Config {
            screen: screen,
            channels: channels,
            pages: pages,
            logic: conditions
        }
    }

    fn build_page_list(self, screen: Screen) -> (
        Vec<Vec<config::Gauge>>,
        Vec<Channel>,
        Logic
    ) {
        let mut gauges: HashMap<String, config::Gauge> = HashMap::new();
        let mut ret = Vec::new();

        for g in self.gauges {
            let name = g.name.clone();
            let cfg = g.to_config(screen, &self.styles);
            gauges.insert(name, cfg);
        }

        for page in self.pages {
            let mut temp = Vec::new();

            for gauge in page.0 {
                let gauge = gauges.get(&gauge).unwrap();
                temp.push(gauge.clone());
            }

            ret.push(temp);
        }

        (ret, self.channels, self.conditions)
    }

    fn validate(self) -> Result<Config, V1Error> {
        // check all channels, states, and gauges have unique names
        // check that all condition tests are based on defined values
        // check that condition graph is acyclic
        // check that all gauges use a defined channel
        // check that all states within a gauge are mutually exclusive
        // warn about unused channels
        // warn about unused states
        // warn about about overlapping gauges
        Ok(self.to_config())
    }
}


pub fn load(path: String) -> Result<Config, V1Error> {
    let reader = File::open(path).expect("Couldn't open config");
    let config: V1 = from_reader(reader).unwrap();
    config.validate()
}
