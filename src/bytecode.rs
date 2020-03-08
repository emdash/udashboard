use std::collections::HashMap;
use std::ops::Deref;
use std::rc::Rc;


// The missing hash literal
macro_rules! map(
    { $($key:expr => $value:expr),+ } => {
        vec! { $( (String::from($key), $value), )+ }
    }
);


// The missing string constructor
fn s(lit: &'static str) -> String {
    String::from(lit)
}


// Abstract over various memory management strategies.
type Node<T> = Rc<T>;
type Seq<T> = Vec<T>;
type Map<T> = Seq<(String, T)>;


// Datastructure to manage lexical scoping.
pub struct Env<T> {
    stack: Vec<HashMap<String, T>>
}


impl<T> Env<T> where T: Clone {
    pub fn new() -> Env<T> {
        let mut ret = Env {stack: vec! {}};
        ret.begin();
        ret
    }

    // Look up an identifier from anywhere in our scope chain.
    pub fn get(&self, key: &String) -> Option<T> {
        for i in (self.stack.len() -1)..0 {
            let env = self.stack[i].clone();
            if let Some(value) = env.get(key) {
                return Some(value.clone())
            }
        }
        None
    }

    // Insert a value in the current scope.
    pub fn define(&mut self, key: &String, value: &T) {
        let env = self.stack.last_mut().unwrap();
        let key = key.clone();
        let value = value.clone();
        env.insert(key, value);
    }

    // Import the map of values into the current scope.
    pub fn import(&mut self, scope: &Seq<(String, T)>) {
        for (k, v) in scope.iter() {
            self.define(&k, &v)
        }
    }

    // Begin a new scope.
    pub fn begin(&mut self) {
        self.stack.push(HashMap::new())
    }

    // End the current scope.
    pub fn end(&mut self) {
        self.stack.pop();
    }
}


// ADT for types
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum TypeTag {
    Unit,
    Bool,
    Int,
    Float,
    Str,
    Point,
    List(Node<TypeTag>),
    Map(Map<TypeTag>),
    Lambda(Seq<TypeTag>, Node<TypeTag>),
    Union(Seq<TypeTag>),
}


// ADT for values
#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Unit,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    Point(f64, f64),
    List(Seq<Expr>),
    Map(Map<Expr>),
    Id(String),
    Dot(Node<Expr>, String),
    Index(Node<Expr>, Node<Expr>),
    Cond(Seq<(Expr, Expr)>, Option<Node<Expr>>),
    Block(Seq<Statement>, Node<Expr>),
    Op(String, Seq<Expr>),
    Lambda(Map<TypeTag>, Node<Expr>)
}


// ADT for effects and structure
#[derive(Clone, Debug, PartialEq)]
pub enum Statement {
    Emit(String, Seq<Expr>),
    Def(String, Node<Expr>),
    For(Node<Expr>, Node<Expr>),
    While(Node<Expr>, Node<Expr>),
}


// ADT for programs
#[derive(Clone, Debug, PartialEq)]
pub struct Program {
    description: String,
    params: HashMap<String, (TypeTag, String)>,
    code: Seq<Statement>
}


#[derive(Clone, Debug, PartialEq)]
pub enum TypeError {
    Mismatch(Node<TypeTag>, Node<TypeTag>),
    NotAList(Node<TypeTag>),
    NotAMap(Node<TypeTag>),
    Undefined(String),
    ListIndexMustBeInt(Node<TypeTag>),
    KeyError(Map<TypeTag>, String),
    NotOneOf(Seq<TypeTag>),
    NotIterable(Node<TypeTag>),
    NotImplemented
}


pub type TypeCheck = core::result::Result<Node<TypeTag>, TypeError>;


pub struct TypeChecker {
    types: Env<TypeTag>,
}

use Expr::*;
use TypeError::*;


impl TypeChecker {
    pub fn new() -> TypeChecker {
        TypeChecker { types: Env::new() }
    }

    // Return the narrowest representation of the given set of types.
    //
    // If the sequence is empty, reduces to unit.
    // If the sequence contains exactly one type, returns that type.
    // If the sequence contains multiple types, returns a Union with de-duped type.
    pub fn narrow(mut types: Seq<Node<TypeTag>>) -> Node<TypeTag> {
        types.dedup();
        match types.len() {
            0 => Rc::new(TypeTag::Unit),
            1 => types.pop().unwrap().clone(),
            _ => Rc::new(TypeTag::Union(
                types.iter().map(|x| x.deref().clone()).collect())
            )
        }
    }

    // Return the type of the given field in a map.
    pub fn lookup(fields: &Map<TypeTag>, name: &String) -> TypeCheck {
        if let Some((_, type_)) = fields.iter().find(|item| &item.0 == name) {
            Ok(Node::new(type_.clone()))
        } else {
            Err(TypeError::KeyError(fields.clone(), name.clone()))
        }
    }

    pub fn eval_expr(&self, expr: &Expr) -> TypeCheck {
        match expr {
            Unit               => Ok(Node::new(TypeTag::Unit)),
            Bool(_)            => Ok(Node::new(TypeTag::Bool)),
            Int(_)             => Ok(Node::new(TypeTag::Int)),
            Float(_)           => Ok(Node::new(TypeTag::Float)),
            Str(_)             => Ok(Node::new(TypeTag::Str)),
            Point(_, _)        => Ok(Node::new(TypeTag::Point)),
            List(items)        => self.eval_list(items),
            Map(items)         => self.eval_map(items),
            Id(name)           => self.eval_id(name),
            Dot(obj, key)      => self.eval_dot(obj, key),
            Index(lst, i)      => self.eval_index(lst, i),
            Cond(cases, def)   => self.eval_cond(cases, def),
            Block(_, ret)      => self.eval_expr(ret),
            Op(op, args)       => self.eval_op(op, args),
            Lambda(args, body) => self.eval_lambda(args, body)
        }
    }

    pub fn eval_list(&self, items: &Seq<Expr>) -> TypeCheck {
        let items: Result<Seq<Node<TypeTag>>, TypeError> = items
            .iter()
            .map(|v| self.eval_expr(v))
            .collect();
        Ok(Node::new(TypeTag::List(Self::narrow(items?))))
    }

    pub fn eval_map(&self, fields: &Seq<(String, Expr)>) -> TypeCheck {
        let fields: Result<Map<TypeTag>, TypeError> = fields
            .iter()
            .map(|(k, v)|  Ok((k.clone(), self.eval_expr(v)?.deref().clone())))
            .collect();
        Ok(Node::new(TypeTag::Map(fields?)))
    }

    pub fn eval_id(&self, name: &String) -> TypeCheck {
        let value = self.types.get(name);
        if let Some(type_) = value {
            Ok(Node::new(type_.clone()))
        } else {
            Err(TypeError::Undefined(name.clone()))
        }
    }

    pub fn eval_dot(&self, obj: &Node<Expr>, field: &String) -> TypeCheck {
        let obj = self.eval_expr(obj)?;
        match obj.deref() {
            TypeTag::Map(items) => Self::lookup(&items, field),
            _ => Err(TypeError::NotAMap(obj.clone()))
        }
    }

    pub fn eval_index(&self, lst: &Node<Expr>, index: &Node<Expr>) -> TypeCheck {
        let lst = self.eval_expr(lst)?;
        let index = self.eval_expr(index)?;

        if index.deref() == &TypeTag::Int {
            match lst.deref() {
                TypeTag::List(item) => Ok(item.clone()),
                x => Err(TypeError::NotAList(lst.clone()))
            }
        } else {
            Err(TypeError::ListIndexMustBeInt(index))
        }
    }

    pub fn eval_cond(
        &self,
        cases: &Seq<(Expr, Expr)>,
        def: &Option<Node<Expr>>
    ) -> TypeCheck {
        let conds: Result<Seq<Node<TypeTag>>, TypeError> = cases
            .iter()
            .map(|case| Ok(self.eval_expr(&case.0)?.clone()))
            .collect();

        let conds = conds?
            .iter()
            .cloned()
            .find(|type_| type_.deref() != &TypeTag::Bool);

        let exprs: Result<Seq<Node<TypeTag>>, TypeError> = cases
            .iter()
            .map(|case| Ok(self.eval_expr(&case.1)?.clone()))
            .collect();


        match conds {
            None => Ok(Self::narrow(exprs?)),
            Some(wrong_type) => Err(
                TypeError::Mismatch(wrong_type, Node::new(TypeTag::Bool))
            )
        }
    }

    pub fn eval_op(&self, name: &String, args: &Seq<Expr>) -> TypeCheck {
        Err(NotImplemented)
    }

    pub fn eval_lambda(&self, args: &Map<TypeTag>, body: &Node<Expr>) -> TypeCheck {
        Err(NotImplemented)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use Expr::*;

    #[test]
    fn test_simple() {
        let tc = TypeChecker::new();
        let tt = Expr::Map(map! {
            "foo" => Int(42),
            "bar" => Str(s("baz")),
            "quux" => List(vec! { Int(1), Int(2), Int(3) })
        });
        println!("{:?}", tt);
        println!("{:?}", tc.eval_expr(&tt));
        assert!(false)
    }
}
