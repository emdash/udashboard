extern crate serde;
extern crate ron;

use ron::de::from_reader;
use std::{fs::File};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Divisions {
    major: u32,
    minor: u32
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Precision {
    integer: u32,
    decimal: u32
}

#[derive(Serialize, Deserialize, Debug)]
pub enum TickStyle {
    NoTicks,
    Numbered(Divisions),
    Unumbered(Divisions)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Unit(String);

#[derive(Serialize, Deserialize, Debug)]
pub enum Interval {
    Open,
    LowerBound(f32),
    UpperBound(f32),
    Range(f32, f32)
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Color {
    Clear,
    Black,
    White,
    Red,
    Orange,
    Yellow,
    Green,
    Blue,
    Indigo,
    Violet,
    RGBA(f32, f32, f32, f32),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Style {
    background: Color,
    foreground: Color,
    indicator: Color,
    warning: Color,
    danger: Color
}

#[derive(Serialize, Deserialize, Debug)]
pub enum GaugeType {
    Numeric(Precision),
    Dial(TickStyle),
    SemiDial(TickStyle),
    Bar(TickStyle),
    Triangle(TickStyle)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ChannelName(String);

#[derive(Serialize, Deserialize, Debug)]
pub struct Polynomial {
    coeffs: Vec<f32>
}

impl Polynomial {
    fn eval(&self, x: f32) -> f32 {
        let mut accum: f32 = 0.0;
        let mut value: f32 = 1.0;

        for c in self.coeffs.iter() {
            accum += c * value;
            value *= x;
        }

        accum
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Length {
    Pixel(f32),
    Percent(f32)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AbsolutePosition {
    x: Length,
    y: Length
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GridSize(u32, u32);

#[derive(Serialize, Deserialize, Debug)]
pub struct GridPosition(u32, u32);

#[derive(Serialize, Deserialize, Debug)]
pub enum AbsoluteSize {
    FromCenter(Length),
    Square(Length),
    Rect(Length, Length),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Layout {
    Absolute(AbsolutePosition, AbsoluteSize),
    Grid(GridSize, GridPosition),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GaugeConfig {
    name: String,
    kind: GaugeType,
    source: ChannelName,
    layout: Layout,
    style: Option<Style>
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ChannelConfig {
    index: u32,
    name: String,
    scale: Interval,
    warning: Option<Interval>,
    danger: Option<Interval>,
    units: Option<Unit>,
    convert: Option<Polynomial>,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum DataSource {
    Mock,
    RaceCapturePro(String)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    width: u32,
    height: u32,
    gauges: Vec<GaugeConfig>,
    channels: Vec<ChannelConfig>,
    datasource: DataSource,
    style: Style
}



fn main() {
    let file = File::open("config.ron").expect("couldn't open config");
    let config: Config = from_reader(file).unwrap();
    println!("Config: {:?}", config);
}
