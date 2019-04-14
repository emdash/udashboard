// Internal Representation used by all rendering backends

use std::collections::HashMap;
use serde::{Deserialize};

#[derive(Deserialize, Debug, Copy, Clone)]
pub struct Screen {
    pub width: f32,
    pub height: f32
}

#[derive(Deserialize, Debug, Copy, Clone)]
pub enum Divisions {
    None,
    Uniform(f32),
    MajorMinor(f32, f32),
}

#[derive(Deserialize, Debug, Clone)]
pub enum Format {
    Numeric(u32, u32),
    Custom(String)
}

#[derive(Deserialize, Debug, Copy, Clone)]
pub enum GaugeStyle {
    IndicatorOnly,
    Outline,
    Filled,
    Dashed
}

#[derive(Deserialize, Debug, Copy, Clone)]
pub struct Scale(pub f32, pub f32, pub Divisions, pub GaugeStyle);

#[derive(Deserialize, Debug, Clone)]
pub enum Lamp {
    Round,
    Rect,
    RoundedRect,
    Image(String)
}

#[derive(Deserialize, Debug, Clone)]
pub enum GaugeType {
    Dial(Scale),
    VerticalBar(Scale),
    HorizontalBar(Scale),
    VerticalWedge(Scale),
    HorizontalWedge(Scale),
    IdiotLight(Lamp),
    Text(Format, GaugeStyle),
}

#[derive(Deserialize, Debug, Clone)]
pub enum Unit {
    None,
    Named(String),
}

#[derive(Deserialize, Debug, Hash, Clone, PartialEq, Eq)]
pub enum State {
    Default,
    Alarm(String)
}

#[derive(Deserialize, Debug, Copy, Clone)]
pub enum Test {
    Always,
    Never,
    LessThan(f32),
    GreaterThan(f32),
    Equal(f32),
    Between(f32, f32)
}

#[derive(Deserialize, Debug, Clone)]
pub struct When(String, Test, State);

pub type Logic = Vec<When>;

#[derive(Deserialize, Debug, Copy, Clone)]
pub struct Point {
    pub x: f32,
    pub y: f32
}

#[derive(Deserialize, Debug, Copy, Clone)]
pub struct Bounds {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32
}

#[derive(Deserialize, Debug, Copy, Clone)]
pub struct Color(pub f32, pub f32, pub f32, pub f32);

#[derive(Deserialize, Debug, Copy, Clone)]
pub enum Pattern {
    Hidden,
    Solid(Color),
    SlowBlink(Color),
    FastBlink(Color),
}

#[derive(Deserialize, Debug, Copy, Clone)]
pub struct Style {
    pub background: Pattern,
    pub foreground: Pattern,
    pub indicator: Pattern
}

impl Style {
    // define a crazy style for debugging.
    pub fn default() -> Style {
        Style {
            background: Pattern::SlowBlink(Color(1.0, 0.0, 0.0, 1.0)),
            foreground: Pattern::Solid(Color(1.0, 0.0, 0.0, 1.0)),
            indicator: Pattern::FastBlink(Color(1.0, 0.0, 1.0, 1.0))
        }
    }
}

pub type StyleSet = HashMap<State, Style>;

#[derive(Deserialize, Debug, Clone)]
pub struct Gauge {
    pub label: String,
    pub kind: GaugeType,
    pub channel: String,
    pub bounds: Bounds,
    pub styles: StyleSet
}

#[derive(Deserialize, Debug, Clone)]
pub enum Source {
    Static(f32),
    Oscillating(f32, f32),
    Random(f32, f32),
    Channel(String),
}

#[derive(Deserialize, Debug, Clone)]
pub enum Function {
    Identity,
    Scale(f32),
    Linear(f32, f32),
    Polynomial(Vec<f32>)
}

#[derive(Deserialize, Debug, Clone)]
pub struct Channel {
    pub name: String,
    pub source: Source,
    pub units: Unit,
    pub transform: Function
}

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub screen: Screen,
    pub channels: Vec<Channel>,
    pub pages: Vec<Vec<Gauge>>,
    pub logic: Logic,
}
