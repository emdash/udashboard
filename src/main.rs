use std;

pub struct Divisions {
    major: u32,
    minor: u32
}

pub struct Precision {
    integer: u32,
    decimal: u32
}

pub enum TickStyle {
    None,
    Numbered(Divisions),
    Unumbered(Divisions)
}

pub struct Unit(String);

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

pub enum GaugeStyle {
    Numeric(Precision),
    Dial(TickStyle),
    SemiDial(TickStyle),
    Bar(TickStyle),
    Triangle(TickStyle)
}

pub struct ChannelName(String);

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

pub struct Bounds {
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32
}

pub struct GaugeConfig {
    style: GaugeStyle,
    source: ChannelName,
    bounds: Bounds
}

pub struct ChannelConfig {
    name: String,
    min: f32,
    max: f32,
    units: Option<Unit>,
    convert: Option<Polynomial>,
    index: u32
}

pub enum DataSource {
    Mock,
    RaceCapturePro
}

pub struct Config {
    width: u32,
    height: u32,
    gauges: Vec<GaugeConfig>,
    channels: Vec<ChannelConfig>,
    datasource: DataSource
}



fn main() {
    println!("Hello, world!");
}
