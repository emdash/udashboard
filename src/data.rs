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
    cell::RefCell,
    collections::HashMap,
    io::{
        BufReader,
        BufRead,
        Read
    },
    sync::{Arc,Mutex, Condvar},
    thread::{spawn}
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
    fn get_state(&self) -> State;
}


pub struct ReadSource {
    line: Arc<Mutex<String>>,
    cv: Arc<Condvar>,
    state: RefCell<State>
}

impl ReadSource {
    pub fn new<R>(src: R) -> ReadSource where R: Read + Send + 'static {
        let line = Arc::new(Mutex::new(String::new()));
        let cv = Arc::new(Condvar::new());
        let state = RefCell::new(State::new());
        let thread_cv = cv.clone();
        let thread_line = line.clone();

        let _ = spawn(move || {
            let mut reader = BufReader::new(src);
            loop {
                let mut lg = thread_line
                    .lock()
                    .unwrap();

                lg.clear();
                reader.read_line(&mut lg).unwrap();

                println!("update");

                thread_cv.notify_all();
            }
        });

        ReadSource {line, cv, state}
    }
}

impl DataSource for ReadSource {
    fn get_state(&self) -> State {
        println!("get state");
        let line = {
            let lg = self.line.lock().unwrap();
            self.cv.wait(lg).unwrap().clone()
        };

        let sample = if let Ok(values) = serde_json::from_str(&line) {
            Sample {values, time: 0.0}
        } else {
            Sample {values: HashMap::new(), time: 0.0}
        };

        self.state.borrow_mut().update(sample);
        self.state.borrow().clone()
    }
}
