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
}
