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

use std::{
    collections::HashMap,
    fs::File,
    io::BufReader,
    io::BufRead,
    io::Read,
    io::Stdin
};

use serde_json;

use crate::config::{Float};
use crate::clock::Clock;

#[derive(Debug, Clone)]
pub struct State {
    pub values: HashMap<String, Float>,
    pub states: HashMap<String, bool>,
    pub time: Float
}

pub struct Sample {
    pub values: HashMap<String, Float>,
    pub time: Float
}

impl State {
    pub fn new() -> State {
        State {
            values: HashMap::new(),
            states: HashMap::new(),
            time: 0.0
        }
    }

    pub fn update(
        &mut self,
        sample: Sample
    ) {
        self.values.extend(sample.values);
        self.time = sample.time;
    }

    pub fn get(&self, key: &String) -> Option<Float> {
        if let Some(value) = self.values.get(key) {
            Some(*value)
        } else {
            None
        }
    }
}

pub trait DataSource {
    fn get_state(&mut self) -> State;
}

pub struct ReaderSource<R: Read> {
    reader: BufReader<R>,
    state: State,
    clock: Clock
}

impl<R> ReaderSource<R> where R: Read {
    pub fn new(read: R) -> ReaderSource<R> {
        let reader = BufReader::new(read);
        let state = State::new();
        let clock = Clock::new();
        ReaderSource {reader, state, clock}
    }
}

impl<R> DataSource for ReaderSource<R> where R: Read {
    fn get_state(&mut self) -> State {
        let mut line: String = String::new();
        self.reader.read_line(&mut line);

        let time = self.clock.seconds();

        let sample = if let Ok(values) = serde_json::from_str(&line) {
            Sample {values, time}
        } else {
            Sample {values: HashMap::new(), time}
        };

        self.state.update(sample);
        self.state.clone()
    }
}

pub type FileSource = ReaderSource<File>;
pub type StdinSource = ReaderSource<Stdin>;
