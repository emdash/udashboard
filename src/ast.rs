use std::collections::HashMap;
use std::rc::Rc;


// Abstract over various memory management strategies.
pub type Node<T> = Rc<T>;
pub type Seq<T> = Vec<Node<T>>;
pub type AList<T> = Vec<(String, Node<T>)>;
pub type Map<T> = HashMap<String, Node<T>>;


// Enum for cairo-specific operations
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CairoOp {
    SetSourceRgb,
    SetSourceRgba,
    Rect,
    Fill,
    Stroke,
    Paint
        // TODO: the rest of the api
}


// Arithmetic and logic operations
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    And,
    Or,
    Xor,
    Lt,
    Gt,
    Lte,
    Gte,
    Eq,
    Shl,
    Shr,
    Min,
    Max
}


#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum UnOp {
    Not,
    Neg,
    Abs,
}


// ADT for types
#[derive(Clone, Debug, PartialEq, Eq)]
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
    Cond(Seq<(Expr, Expr)>),
    Block(Seq<Statement>, Node<Expr>),
    BinOp(BinOp, Node<Expr>, Node<Expr>),
    UnOp(UnOp, Node<Expr>),
    Call(Node<Expr>, Seq<Expr>),
    Lambda(AList<TypeTag>, Node<Expr>)
}


pub fn bin(op: BinOp, lhs: Expr, rhs: Expr) -> Expr {
    Expr::BinOp(op, Node::new(lhs), Node::new(rhs))
}


pub fn un(op: UnOp, operand: Expr) -> Expr {
    Expr::UnOp(op, Node::new(operand))
}


pub fn id(name: &'static str) -> Expr {
    Expr::Id(String::from(name))
}


pub fn call(func: Expr, args: Vec<Expr>) -> Expr {
    Expr::Call(
        Node::new(func),
        args.into_iter().map(|e| Node::new(e)).collect()
    )
}


pub fn dot(obj: Expr, id: &str) -> Expr {
    Expr::Dot(Node::new(obj), String::from(id))
}


pub fn index(obj: Expr, e: Expr) -> Expr {
    Expr::Index(Node::new(obj), Node::new(e))
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
    pub description: String,
    pub params: HashMap<String, (TypeTag, String)>,
    pub code: Seq<Statement>
}
