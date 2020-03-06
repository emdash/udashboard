use std::borrow::Borrow;
use std::collections::HashMap;
use std::rc::Rc;


// Abstract over various memory management strategies.
type Node<T> = Rc<T>;
type Seq<T> = Node<Vec<T>>;
type Map<T> = Node<HashMap<String, T>>;


// Datastructure to manage lexical scoping.
struct Env<T> {
    stack: Vec<HashMap<String, T>>
}


impl<T> Env<T> where T: Clone {
    pub fn new() -> Env<T> {
        let mut ret = Env {stack: vec! {}};
        ret.push();
        ret
    }

    // Look up an identifier from anywhere in our scope chain.
    pub fn get(&mut self, key: &String) -> Option<T> {
        for i in (self.stack.len() -1)..0 {
            let env = self.stack[i].clone();
            if let Some(value) = env.get(key) {
                return Some(value.clone())
            }
        }
        None
    }

    // Insert a value in the current scope.
    pub fn insert(&mut self, key: &String, value: &T) {
        let env = self.stack.last_mut().unwrap();
        let key = key.clone();
        let value = value.clone();
        env.insert(key, value);
    }

    // Import the map of values into the current scope.
    pub fn import(&mut self, scope: &Seq<(String, T)>) {
        self.push();
        for (k, v) in scope.iter() {
            self.insert(&k, &v)
        }
    }

    // Begin a new scope.
    pub fn push(&mut self) {
        self.stack.push(HashMap::new())
    }

    // End the current scope.
    pub fn pop(&mut self) {
        self.stack.pop();
    }
}


#[derive(Clone, Debug, PartialEq)]
pub enum TypeTag {
    Unit,
    Bool,
    Int,
    Float,
    Str,
    Point,
    List(Node<TypeTag>),
    Map(Map<TypeTag>),
    Lambda(Map<TypeTag>, Node<TypeTag>),
    Union(Seq<TypeTag>),
}


#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Unit,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    Point(f64, f64),
    List(Seq<Expr>),
    Map(Seq<(String, Node<Expr>)>),
    Id(Node<String>),
    Dot(Node<Expr>, String),
    Index(Node<Expr>, Node<Expr>),
    Cond(Seq<(Node<Expr>, Node<Expr>)>),
    Block(Seq<Statement>, Node<Expr>),
    Op(String, Seq<Expr>),
    Lambda(Seq<(String, TypeTag)>, Node<Expr>)
}


#[derive(Clone, Debug, PartialEq)]
pub enum Statement {
    Emit(String, Seq<Expr>),
    Def(String, Seq<Expr>),
    For(Node<Expr>, Node<Expr>),
    While(Node<Expr>, Node<Expr>)
}


#[derive(Clone, Debug, PartialEq)]
pub struct Program {
    description: String,
    params: HashMap<String, (TypeTag, String)>,
    code: Seq<Statement>
}


#[derive(Clone, Debug, PartialEq)]
pub enum TypeErr {
    NotAList(Node<TypeTag>),
    NotAMap(Node<TypeTag>),
    Undefined(String),
    InvalidField(Node<TypeTag>, String),
    NotOneOf(Seq<TypeTag>),
    NotIterable(Node<TypeTag>),
    NotImplemented
}


pub type TypeCheck = core::result::Result<TypeTag, TypeErr>;


pub struct TypeChecker {
    types: Env<TypeTag>,
}

use Expr::*;
use TypeErr::*;

impl TypeChecker {
    pub fn new() -> TypeChecker {
        TypeChecker { types: Env::new() }
    }

    pub fn eval_expr(&self, expr: &Expr) -> TypeCheck {
        match expr {
            Unit          => Ok(TypeTag::Unit),
            Bool(_)       => Ok(TypeTag::Bool),
            Int(_)        => Ok(TypeTag::Int),
            Float(_)      => Ok(TypeTag::Float),
            Str(_)        => Ok(TypeTag::Str),
            Point(_, _)   => Ok(TypeTag::Point),
            List(items)   => self.eval_list(items),
            Map(items)    => self.eval_map(items),
            Id(name)      => self.eval_id(name),
            Dot(obj, key) => self.eval_dot(obj, key),
            Index(lst, i) => self.eval_index(lst, i),
            Cond(ts)      => self.eval_cond(ts),
            Block(_, ret) => self.eval_expr(ret),
            Op(op, args)  => self.eval_op(op, args),
            Lambda(args, body) => self.eval_lambda(args, body)
        }
    }

    pub fn eval_list(&self, items: &Seq<Expr>) -> TypeCheck {
        Err(NotImplemented)
    }

    pub fn eval_map(&self, items: &Seq<(String, Node<Expr>)>) -> TypeCheck {
        Err(NotImplemented)
    }

    pub fn eval_id(&self, name: &String) -> TypeCheck {
        Err(NotImplemented)
    }

    pub fn eval_dot(&self, obj: &Node<Expr>, string: &String) -> TypeCheck {
        Err(NotImplemented)
    }

    pub fn eval_index(&self, lst: &Node<Expr>, index: &Node<Expr>) -> TypeCheck {
        Err(NotImplemented)
    }

    pub fn eval_cond(&self, item: &Seq<(Node<Expr>, Node<Expr>)>) -> TypeCheck {
        Err(NotImplemented)
    }

    pub fn eval_op(&self, name: &String, args: &Seq<Expr>) -> TypeCheck {
        Err(NotImplemented)
    }

    pub fn eval_lambda(&self, args: &Seq<(String, TypeTag)>, body: &Node<Expr>) -> TypeCheck {
        Err(NotImplemented)
    }
}
