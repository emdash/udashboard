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

// Data handling

use std::collections::HashMap;
use crate::config::Logic;


pub struct State {
    pub values: HashMap<String, f32>,
    pub states: HashMap<String, bool>,
    pub time: u64
}


pub struct Sample {
    pub values: HashMap<String, f32>,
    pub time: u64
}


impl State {
    pub fn new() -> State {
        State {
            values: HashMap::new(),
            states: HashMap::new(),
            time: 0
        }
    }

    pub fn update(
        mut self,
        sample: Sample,
        _logic: &Logic,
    ) -> State {
        self.values.extend(sample.values);

        State {
            values: self.values,
            states: self.states,
            time: sample.time
        }
    }

    pub fn get(&self, key: &String) -> Option<f32> {
        if let Some(value) = self.values.get(key) {
            Some(*value)
        } else {
            None
        }
    }
}
