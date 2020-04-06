use crate::ast::{Node, AList, Map};
use std::cell::RefCell;
use std::fmt::Debug;


// Datastructure to manage lexical scoping.
pub struct Env<T> {
    scope: RefCell<Map<T>>,
    parent: Option<Node<Env<T>>>
}


impl<T> Env<T> where T: Clone + Debug {
    fn new(parent: Option<Node<Env<T>>>) -> Env<T> {
        let scope = RefCell::new(Map::new());
        Env {scope, parent}
    }

    pub fn root() -> Env<T> {
        Self::new(None)
    }

    pub fn chain(parent: &Node<Env<T>>) -> Env<T> {
        let ret = Self::new(Some(parent.clone()));
        ret
    }

    // Look up an identifier from anywhere in our scope chain.
    pub fn get(&self, key: &String) -> Option<Node<T>> {
        if let Some(value) = self.scope.borrow().get(key) {
            Some(value.clone())
        } else if let Some(env) = &self.parent {
            env.get(key)
        } else {
            None
        }
    }

    // Insert a value in the current scope.
    pub fn define(&self, key: &String, value: &Node<T>) {
        // TODO: handle redefinition.
        self.scope.borrow_mut().insert(key.clone(), value.clone());
    }

    // Import the map of values into the current scope.
    pub fn import(&self, scope: &AList<T>) {
        for (k, v) in scope.iter() {
            self.define(k, v)
        }
    }
}
