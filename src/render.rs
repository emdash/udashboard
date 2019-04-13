// Cairo rendering implementation

use crate::config::{
    Bounds,
    Divisions,
    Gauge,
    GaugeStyle,
    GaugeType,
    Lamp,
    Pattern,
    Range,
    Color,
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

    pub fn render(&self, _state: State) {
    }
}
