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

use std::{
    collections::HashMap,
    env::args,
    io::stdin
};

use udashboard::v1;
use udashboard::{
    drm,
    render::{CairoRenderer, PNGRenderer},
    data::{State, ReadSource},
    vm
};


fn main() {
    let config = v1::load(args().nth(1).unwrap())
        .expect("couldn't load config");

    let renderer = CairoRenderer::new(
        config.screen,
        vm::load(args().nth(1).expect("no program file given.")).unwrap()
    );

    if let Some(path) = args().nth(2) {
        drm::run(path, renderer, ReadSource::new(stdin()));
    } else {
        println!("No device path given, rendering to png.");

        let mut state = State {
            values: HashMap::new(),
            states: HashMap::new(),
            time: 0.0
        };

        state.values.insert("RPM".to_string(), 1500.0);
        state.values.insert("OIL_PRESSURE".to_string(), 45.0);
        state.values.insert("ECT".to_string(), 205.0);
        state.values.insert("SESSION_TIME".to_string(), 105.0);
        state.values.insert("GEAR".to_string(), 5.0);
        state.values.insert("RPM".to_string(), 1500.0);

        PNGRenderer::new(
            "screenshot.png".to_string(),
            renderer
        ).render(&state);
    }
}
