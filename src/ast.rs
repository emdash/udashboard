use std::collections::HashMap;
use std::rc::Rc;
use std::ops::Deref;


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
#[derive(Clone, Debug, PartialEq)]
pub enum TypeTag {
    Unit,
    Bool,
    Int,
    Float,
    Str,
    Point,
    Tuple(Seq<TypeTag>),
    List(Node<TypeTag>),
    Map(Map<TypeTag>),
    Record(AList<Member>),
    Lambda(Seq<TypeTag>, Node<TypeTag>),
    Union(Seq<TypeTag>),
}


// ADT For record fields
#[derive(Clone, Debug, PartialEq)]
pub enum Member {
    Field(Node<TypeTag>),
    Method(AList<TypeTag>, Node<TypeTag>, Node<Expr>),
    StaticValue(Node<Expr>),
    StaticMethod(AList<TypeTag>, Node<TypeTag>, Node<Expr>)
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
    Cond(Seq<(Expr, Expr)>, Node<Expr>),
    Block(Seq<Statement>, Node<Expr>),
    BinOp(BinOp, Node<Expr>, Node<Expr>),
    UnOp(UnOp, Node<Expr>),
    Call(Node<Expr>, Seq<Expr>),
    Lambda(AList<TypeTag>, Node<TypeTag>, Node<Expr>)
}


pub fn to_seq<T>(items: Vec<T>) -> Seq<T> {
    items.into_iter().map(|e| Node::new(e)).collect()
}


pub fn to_alist<T>(items: Vec<(String, T)>) -> AList<T> {
    items.into_iter().map(|i| (i.0, Node::new(i.1))).collect()
}


pub fn to_map<T>(items: Vec<(String, T)>) -> Map<T> {
    items.into_iter().map(|i| (i.0, Node::new(i.1))).collect()
}


pub fn map_to_seq<T>(items: &Map<T>) -> Seq<T> {
    items.iter().map(|i| i.1.clone()).collect()
}


pub fn s(s: &str) -> String {
    String::from(s)
}


pub fn string(st: &str) -> Expr {
    Expr::Str(s(st))
}


pub fn list(items: Vec<Expr>) -> Expr {
    Expr::List(to_seq(items))
}


pub fn map(items: Vec<(String, Expr)>) -> Expr {
    Expr::Map(to_map(items))
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
    Expr::Call(Node::new(func), to_seq(args))
}


pub fn dot(obj: Expr, id: &str) -> Expr {
    Expr::Dot(Node::new(obj), String::from(id))
}


pub fn index(obj: Expr, e: Expr) -> Expr {
    Expr::Index(Node::new(obj), Node::new(e))
}


pub fn cond(cases: Vec<(Expr, Expr)>, default: Expr) -> Expr {
    Expr::Cond(to_seq(cases), Node::new(default))
}


pub fn expr_block(stmts: Vec<Statement>, ret: Expr) -> Expr {
    Expr::Block(to_seq(stmts), Node::new(ret))
}


pub fn lambda(
    args: Vec<(String, TypeTag)>,
    ret: TypeTag,
    body: Expr
) -> Expr {
    Expr::Lambda(to_alist(args), Node::new(ret), Node::new(body))
}


// ADT for effects and structure
#[derive(Clone, Debug, PartialEq)]
pub enum Statement {
    ExprForEffect(Node<Expr>),
    Emit(String, Seq<Expr>),
    Def(String, Node<Expr>),
    TypeDef(String, Node<TypeTag>),
    ListIter(String, Node<Expr>, Node<Statement>),
    MapIter(String, String, Node<Expr>, Node<Statement>),
    While(Node<Expr>, Node<Statement>),
}


pub fn expr_for_effect(expr: Expr) -> Statement {
    match &expr {
        Expr::Block(stmts, node) => match node.deref() {
            Expr::Unit => if stmts.len() == 1 {
                stmts[0].deref().clone()
            } else {
                Statement::ExprForEffect(Node::new(expr))
            },
            _ => Statement::ExprForEffect(Node::new(expr))
        },
        _ => Statement::ExprForEffect(Node::new(expr))
    }
}


pub fn statement_block(statements: Vec<Statement>) -> Statement {
    Statement::ExprForEffect(Rc::new(expr_block(statements, Expr::Unit)))
}


pub fn emit(name: &str, exprs: Vec<Expr>) -> Statement {
    Statement::Emit(String::from(name), to_seq(exprs))
}


pub fn def(name: &str, expr: Expr) -> Statement {
    Statement::Def(String::from(name), Node::new(expr))
}


pub fn typedef(name: &str, t: TypeTag) -> Statement {
    Statement::TypeDef(String::from(name), Node::new(t))
}


pub fn list_iter(name: &str, list: Expr, body: Statement) -> Statement {
    Statement::ListIter(
        String::from(name),
        Node::new(list),
        Node::new(body)
    )
}


pub fn map_iter(
    key: &str,
    value: &str,
    map: Expr,
    body: Statement
) -> Statement {
    Statement::MapIter(
        String::from(key),
        String::from(value),
        Node::new(map),
        Node::new(body)
    )
}


pub fn while_(cond: Expr, body: Statement) -> Statement {
    Statement::While(Node::new(cond), Node::new(body))
}


pub fn guard(
    clauses: Vec<(Expr, Statement)>,
    default: Option<Statement>
) -> Statement {
    let clauses = clauses
        .into_iter()
        .map(|x| (x.0, expr_block(vec!{x.1}, Expr::Unit)))
        .collect();

    let default = if let Some(default) = default {
        expr_block(vec!{default}, Expr::Unit)
    } else {
        Expr::Unit
    };

    expr_for_effect(cond(clauses, default))
}


// ADT for programs
#[derive(Clone, Debug, PartialEq)]
pub struct Program {
    pub description: String,
    pub params: HashMap<String, (TypeTag, String)>,
    pub code: Seq<Statement>
}
