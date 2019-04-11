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
    None,
    Numbered(Divisions),
    Unumbered(Divisions)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Unit(String);

#[derive(Serialize, Deserialize, Debug)]
pub enum Color {
    Clear,
    Black,
    White,
    Red,
    Organge,
    Yellow,
    Green,
    Blue,
    Indigo,
    Violet,
    RGBA(f32, f32, f32, f32),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum GaugeStyle {
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
pub struct Bounds {
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GaugeConfig {
    style: GaugeStyle,
    source: ChannelName,
    bounds: Bounds
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ChannelConfig {
    name: String,
    min: f32,
    max: f32,
    units: Option<Unit>,
    convert: Option<Polynomial>,
    index: u32
}

#[derive(Serialize, Deserialize, Debug)]
pub enum DataSource {
    Mock,
    RaceCapturePro
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    width: u32,
    height: u32,
    gauges: Vec<GaugeConfig>,
    channels: Vec<ChannelConfig>,
    datasource: DataSource
}



fn main() {
    let file = File::open("config.ron").expect("couldn't open config");
    let config: Config = from_reader(file).unwrap();
    println!("Config: {:?}", config);
}
