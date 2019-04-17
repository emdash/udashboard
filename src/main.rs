use std::collections::HashMap;

use udashboard::v1;
use udashboard::config::{Style, Pattern, Color};
use udashboard::render::PNGRenderer;
use udashboard::data::State;
use udashboard::output::drm_magic;

fn main() {
    let config = v1::load("config.ron".to_string()).unwrap();
    let renderer = PNGRenderer::new(
        "dashbard.png".to_string(),
        config.screen,
        config.pages,
        Style {
            background: Pattern::Solid(Color(0.0, 0.0, 0.0, 1.0)),
            foreground: Pattern::Solid(Color(1.0, 1.0, 1.0, 1.0)),
            indicator: Pattern::Solid(Color(1.0, 0.0, 0.0, 1.0)),
        }
    );

    let mut values = HashMap::new();
    values.insert("RPM".to_string(), 1500.0 as f32);

    let state = State {
        values: values,
        states: HashMap::new(),
        time: 0
    };

    renderer.render(&state);
    // start update loop.

    drm_magic();
}
