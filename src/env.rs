use crate::ast::{Node, AList, Map};
use std::fmt::Debug;


// Datastructure to manage lexical scoping.
pub struct Env<T> {
    stack: Vec<Map<T>>
}


impl<T> Env<T> where T: Clone + Debug {
    pub fn new() -> Env<T> {
        let mut ret = Env {stack: Vec::new()};
        ret.begin();
        ret
    }

    // Look up an identifier from anywhere in our scope chain.
    pub fn get(&self, key: &String) -> Option<Node<T>> {
        println!("{:?}", self.stack);
        let len = self.stack.len();
        for i in 0..len {
            let idx = len - i - 1;
            println!("{:?}", idx);
            let env = &self.stack[idx];
            if let Some(value) = env.get(key) {
                return Some(value.clone())
            }
        }
        None
    }

    // Insert a value in the current scope.
    pub fn define(&mut self, key: &String, value: &Node<T>) {
        let env = self.stack.last_mut().unwrap();
        let key = key.clone();
        let value = value.clone();
        env.insert(key, value);
    }

    // Import the map of values into the current scope.
    pub fn import(&mut self, scope: &AList<T>) {
        for (k, v) in scope.iter() {
            self.define(k, v)
        }
    }

    // Begin a new scope.
    pub fn begin(&mut self) {
        self.stack.push(Map::new())
    }

    // End the current scope.
    pub fn end(&mut self) {
        self.stack.pop();
    }
}
