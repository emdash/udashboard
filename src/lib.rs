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

extern crate cairo;
extern crate gtk;
extern crate regex;
extern crate ron;
extern crate serde;
#[macro_use]
extern crate lazy_static;


pub mod ast;
pub mod clock;
pub mod config;
pub mod data;
pub mod env;
pub mod drm;
pub mod windowed;
pub mod render;
pub mod typechecker;
#[macro_use]
pub mod util;
pub mod v1;
pub mod vm;

