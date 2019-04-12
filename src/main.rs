extern crate serde;
extern crate ron;

use ron::de::from_reader;
use std::{fs::File};
use serde::{Deserialize};

#[derive(Deserialize, Debug)]
pub enum Divisions {
    None,
    Uniform(f32),
    MajorMinor(f32, f32),
}

#[derive(Deserialize, Debug)]
pub enum Format {
    Numeric(u32, u32),
    Custom(String)
}

#[derive(Deserialize, Debug)]
pub struct Range(f32, f32);

#[derive(Deserialize, Debug)]
pub enum GaugeStyle {
    IndicatorOnly,
    Outline,
    Filled,
    Dashed
}

#[derive(Deserialize, Debug)]
pub enum Lamp {
    Round,
    Rect,
    RoundedRect,
    Image(String)
}

#[derive(Deserialize, Debug)]
pub enum Unit {
    None,
    Named(String),
}

#[derive(Deserialize, Debug)]
pub enum Test {
    Always,
    Never,
    LessThan(f32),
    GreaterThan(f32),
    Equal(f32),
    Between(f32, f32)
}

#[derive(Deserialize, Debug)]
pub enum Color {
    Clear,
    Black,
    White,
    Grey,
    Red,
    Orange,
    Yellow,
    Green,
    Blue,
    Indigo,
    Violet,
    RGBA(f32, f32, f32, f32)
}

#[derive(Deserialize, Debug)]
pub enum Pattern {
    Inherit,
    Solid(Color),
    SlowBlink(Color),
    FastBlink(Color)
}

#[derive(Deserialize, Debug)]
pub struct StyleDef {
    name: String,
    background: Pattern,
    foreground: Pattern,
    indicator: Pattern
}

#[derive(Deserialize, Debug)]
pub enum Style {
    Default(Color, Color, Color),
    Define(StyleDef)
}

#[derive(Deserialize, Debug)]
pub enum State {
    Default,
    Alarm(String)
}

#[derive(Deserialize, Debug)]
pub enum GaugeType {
    Dial(Range, Divisions, GaugeStyle),
    VerticalBar(Range, Divisions, GaugeStyle),
    HorizontalBar(Range, Divisions, GaugeStyle),
    VerticalWedge(Range, Divisions, GaugeStyle),
    HorizontalWedge(Range, Divisions, GaugeStyle),
    IdiotLight(Lamp),
    Text(Format, GaugeStyle),
}

#[derive(Deserialize, Debug)]
pub struct When(String, Test, State);

#[derive(Deserialize, Debug)]
enum Source {
    Static(f32),
    Oscillating(f32, f32),
    Random(f32, f32),
    Channel(String),
}

#[derive(Deserialize, Debug)]
pub enum Conversion {
    Identity,
    Scale(f32),
    Linear(f32, f32),
    Polynomial(Vec<f32>)
}

#[derive(Deserialize, Debug)]
pub enum Length {
    Pixel(f32),
    Percent(f32)
}

#[derive(Deserialize, Debug)]
pub enum Size {
    Radius(Length),
    Box(Length, Length)
}

#[derive(Deserialize, Debug)]
pub struct Point {
    x: Length,
    y: Length
}

#[derive(Deserialize, Debug)]
pub struct GridSize(u32, u32);

#[derive(Deserialize, Debug)]
pub struct GridPosition(u32, u32);

#[derive(Deserialize, Debug)]
pub enum Layout {
    FromCenter(Point, Size),
    FromTopLeft(Point, Point),
    Grid(GridSize, GridPosition),
}

#[derive(Deserialize, Debug)]
pub struct Gauge {
    name: String,
    kind: GaugeType,
    channel: String,
    layout: Layout,
    styles: Vec<(State, String)>
}

#[derive(Deserialize, Debug)]
pub struct Channel {
    name: String,
    source: Source,
    units: Unit,
    transform: Conversion
}

#[derive(Deserialize, Debug)]
pub struct Page(Vec<String>);

#[derive(Deserialize, Debug)]
pub struct V1 {
    width: u32,
    height: u32,
    channels: Vec<Channel>,
    conditions: Vec<When>,
    gauges: Vec<Gauge>,
    pages: Vec<Page>,
    styles: Vec<Style>
}

#[derive(Debug)]
pub enum V1Error {
    ReadError(String),
    ParseError(String),
    NoSuchChannel(String),
    NoSuchState(State),
    NoSuchGauge(Gauge)
}

impl V1 {
    pub fn new_from_file(path: String) -> Result<V1, V1Error> {
        let reader = File::open(path).expect("Couldn't open config");
        let config: V1 = from_reader(reader).unwrap();
        config.validate()
    }

    fn validate(self) -> Result<V1, V1Error> {
        // check all channels, states, and gauges have unique names
        // check that all condition tests are based on defined values
        // check that condition graph is acyclic
        // check that all gauges use a defined channel
        // check that all states within a gauge are mutually exclusive
        // warn about unused channels
        // warn about unused states
        // warn about about overlapping gauges
        Ok(self)
    }
}

fn main() {
    println!("Config: {:?}", V1::new_from_file("config.ron".to_string()));
}
