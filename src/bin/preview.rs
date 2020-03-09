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
};

use udashboard::config::Screen;
use udashboard::vm;
use udashboard::windowed;
use udashboard::render::{CairoRenderer, parse};

const prog: &str = "1.0 0.0 0.0 rgb 0 0 #1 load get #1 load get rect fill";

fn main() {
    let screen = Screen { width: 1024.0, height: 600.0 };
    let renderer = CairoRenderer::new(
        screen,
        parse(prog)
    );
    windowed::run(screen, renderer);
}
