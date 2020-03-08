use crate::ast::*;
use crate::env::*;
use std::ops::Deref;


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
    NotCallable(Node<TypeTag>),
    ArgError(Seq<TypeTag>, Seq<TypeTag>),
    NotImplemented
}


use TypeError::*;


pub type TypeCheck = core::result::Result<Node<TypeTag>, TypeError>;


pub struct TypeChecker {
    types: Node<Env<TypeTag>>,
}


impl TypeChecker {
    pub fn new(env: Env<TypeTag>) -> TypeChecker {
        TypeChecker { types: Node::new(env) }
    }

    // Return the narrowest representation of the given set of types.
    //
    // If the sequence is empty, reduces to unit.
    // If the sequence contains exactly one type, returns that type.
    // If the sequence contains multiple types, returns a Union with de-duped type.
    pub fn narrow(mut types: Seq<TypeTag>) -> Node<TypeTag> {
        types.dedup();
        match types.len() {
            0 => Node::new(TypeTag::Unit),
            1 => types.pop().unwrap(),
            _ => Node::new(TypeTag::Union(types))
        }
    }

    // Return the type of the given field in a map.
    pub fn lookup(fields: &Map<TypeTag>, name: &String) -> TypeCheck {
        if let Some(type_) = fields.get(name) {
            Ok(type_.clone())
        } else {
            Err(KeyError(fields.clone(), name.clone()))
        }
    }

    pub fn eval_expr(&self, expr: &Expr) -> TypeCheck {
        match expr {
            Expr::Unit               => Ok(Node::new(TypeTag::Unit)),
            Expr::Bool(_)            => Ok(Node::new(TypeTag::Bool)),
            Expr::Int(_)             => Ok(Node::new(TypeTag::Int)),
            Expr::Float(_)           => Ok(Node::new(TypeTag::Float)),
            Expr::Str(_)             => Ok(Node::new(TypeTag::Str)),
            Expr::Point(_, _)        => Ok(Node::new(TypeTag::Point)),
            Expr::List(items)        => self.eval_list(items),
            Expr::Map(items)         => self.eval_map(items),
            Expr::Id(name)           => self.eval_id(name),
            Expr::Dot(obj, key)      => self.eval_dot(obj, key),
            Expr::Index(lst, i)      => self.eval_index(lst, i),
            Expr::Cond(cases)        => self.eval_cond(cases),
            Expr::Block(stmts, ret)  => self.eval_block(stmts, ret),
            Expr::BinOp(op, l, r)    => self.eval_binop(*op, l, r),
            Expr::UnOp(op, operand)  => self.eval_unop(*op, operand),
            Expr::Call(func, args)   => self.eval_call(func, args),
            Expr::Lambda(args, body) => self.eval_lambda(args, body)
        }
    }

    pub fn eval_list(&self, items: &Seq<Expr>) -> TypeCheck {
        let items: Result<Seq<TypeTag>, TypeError> = items
            .iter()
            .map(|v| self.eval_expr(v))
            .collect();
        Ok(Node::new(TypeTag::List(Self::narrow(items?))))
    }

    pub fn eval_map(&self, fields: &Map<Expr>) -> TypeCheck {
        let fields: Result<Map<TypeTag>, TypeError> = fields
            .iter()
            .map(|(k, v)|  Ok((k.clone(), self.eval_expr(v)?)))
            .collect();
        Ok(Node::new(TypeTag::Map(fields?)))
    }

    pub fn eval_id(&self, name: &String) -> TypeCheck {
        let value = self.types.get(name);
        if let Some(type_) = value {
            Ok(type_.clone())
        } else {
            Err(Undefined(name.clone()))
        }
    }

    pub fn eval_dot(&self, obj: &Node<Expr>, field: &String) -> TypeCheck {
        let obj = self.eval_expr(obj)?;
        match obj.deref() {
            TypeTag::Map(items) => Self::lookup(&items, field),
            _ => Err(NotAMap(obj.clone()))
        }
    }

    pub fn eval_block(
        &self,
        stmts: &Seq<Statement>,
        ret: &Node<Expr>
    ) -> TypeCheck {
        let mut env = Env::chain(&self.types);
        let sub = TypeChecker::new(env);
        for stmt in stmts {
            sub.check_statement(stmt)?
        }
        sub.eval_expr(ret)
    }

    pub fn check_iterable(&self, expr: &Node<Expr>) -> Result<(), TypeError> {
        let result = self.eval_expr(expr)?;
        match result.deref() {
            TypeTag::List(_) => Ok(()),
            _ => Err(NotIterable(result))
        }
    }

    pub fn check_bool(&self, expr: &Node<Expr>) -> Result<(), TypeError> {
        let result = self.eval_expr(expr)?;
        match result.deref() {
            TypeTag::Bool => Ok(()),
            _ => Err(Mismatch(result, Node::new(TypeTag::Bool)))
        }
    }


    pub fn check_statement(
        &self,
        stmt: &Node<Statement>
    ) -> Result<(), TypeError> {
        match stmt.deref() {
            Statement::Emit(_, exprs) => {
                for expr in exprs {
                    self.eval_expr(expr)?;
                }
            },
            Statement::Def(name, val) => {
                self.types.define(name, &self.eval_expr(val)?);
            }
            Statement::For(lst, body) => {
                self.check_iterable(lst)?;
                self.eval_expr(body)?;
            },
            Statement::While(cond, body) => {
                self.check_bool(cond)?;
                self.eval_expr(body)?;
            }
        };
        Ok(())
    }

    pub fn check_program(&self, prog: Program) -> Result<(), TypeError> {
        for stmt in prog.code {
            self.check_statement(&stmt)?;
        }
        Ok(())
    }

    pub fn eval_index(&self, lst: &Node<Expr>, index: &Node<Expr>) -> TypeCheck {
        let lst = self.eval_expr(lst)?;
        let index = self.eval_expr(index)?;

        if index.deref() == &TypeTag::Int {
            match lst.deref() {
                TypeTag::List(item) => Ok(item.clone()),
                x => Err(NotAList(lst.clone()))
            }
        } else {
            Err(ListIndexMustBeInt(index))
        }
    }

    pub fn eval_cond(&self, cases: &Seq<(Expr, Expr)>) -> TypeCheck {
        let conds: Result<Seq<TypeTag>, TypeError> = cases
            .iter()
            .map(|case| Ok(self.eval_expr(&case.0)?.clone()))
            .collect();

        let conds = conds?
            .iter()
            .cloned()
            .find(|type_| type_.deref() != &TypeTag::Bool);

        let exprs: Result<Seq<TypeTag>, TypeError> = cases
            .iter()
            .map(|case| Ok(self.eval_expr(&case.1)?.clone()))
            .collect();


        match conds {
            None => Ok(Self::narrow(exprs?)),
            Some(wrong_type) => Err(
                Mismatch(wrong_type, Node::new(TypeTag::Bool))
            )
        }
    }

    pub fn eval_binop(
        &self,
        op: BinOp,
        l: &Node<Expr>,
        r: &Node<Expr>
    ) -> TypeCheck {
        use TypeTag::*;
        let l = self.eval_expr(l)?;
        let r = self.eval_expr(r)?;
        match (op, l.deref(), r.deref()) {
            (BinOp::Eq, a, b) if a == b => Ok(Node::new(a.clone())),
            (_, Bool, Bool)   => Ok(Node::new(Bool)),
            (_, Int, Int)     => Ok(Node::new(Int)),
            (_, Float, Float) => Ok(Node::new(Float)),
            (_, Str, Str)     => Ok(Node::new(Float)),
            _                 => Err(Mismatch(l, r))
        }
    }

    pub fn eval_unop(&self, op: UnOp, operand: &Node<Expr>) -> TypeCheck {
        use TypeTag::*;
        let type_ = self.eval_expr(operand)?;
        let numeric = Node::new(Union(vec! {
            Node::new(Int),
            Node::new(Float)
        }));
        match (op, type_.deref()) {
            (Not, Bool)  => Ok(Node::new(Bool)),
            (Not, _)     => Err(Mismatch(type_, Node::new(Bool))),
            (Neg, Int)   => Ok(Node::new(Int)),
            (Neg, Float) => Ok(Node::new(Float)),
            (Neg, _)     => Err(Mismatch(type_, numeric)),
            (Abs, Int)   => Ok(Node::new(Int)),
            (Abs, Float) => Ok(Node::new(Float))
        }
    }

    fn eval_call(&self, func: &Node<Expr>, args: &Seq<Expr>) -> TypeCheck {
        let func = self.eval_expr(func)?;
        let args: Result<Seq<TypeTag>, TypeError> = args
            .iter()
            .map(|arg| Ok(self.eval_expr(arg)?))
            .collect();
        let args = args?;

        if let TypeTag::Lambda(aargs, ret) = func.deref() {
            if args == args {
                Ok(ret.clone())
            } else {
                Err(ArgError(args, aargs.clone()))
            }
        } else {
            Err(NotCallable(func))
        }
    }

    pub fn eval_lambda(&self, args: &AList<TypeTag>, body: &Node<Expr>) -> TypeCheck {
        let mut env = Env::chain(&self.types);
        env.import(args);
        let sub = TypeChecker::new(env);
        Ok(Node::new(TypeTag::Lambda(
            args.iter().map(|arg| arg.1.clone()).collect(),
            sub.eval_expr(body)?
        )))
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    // The missing String literal
    macro_rules! string(
        { $s:expr } => { String::from($s) }
    );

    // Hash literal that wraps items in a Node
    macro_rules! map(
        { $($key:expr => $value:expr),* } => {
            vec! { $( (string!($key), Node::new($value))),* }
            .iter()
            .cloned()
            .collect()
        }
    );

    // Vec literal that wraps items in a Node
    macro_rules! list(
        { $($i:expr),* } => { vec! { $( Node::new($i)),* } }
    );

    macro_rules! node(
        { $i:expr } => { Node::new($i) }
    );

    macro_rules! assert_types_to(
        ( $env:expr, $e:expr, Ok($t:expr) ) => {
            let tc = TypeChecker::new($env);
            let expr = {
                use Expr::*;
                $e
            };
            let type_ = {
                use TypeTag::*;
                $t
            };
            assert_eq!(tc.eval_expr(&expr), Ok(Node::new(type_)));
        };
        ( $env:expr, $e:expr, Err($t:expr) ) => {
            let tc = TypeChecker::new($env);
            let expr = {
                use Expr::*;
                $e
            };
            let err = {
                use TypeError::*;
                use TypeTag::*;
                $t
            };
            assert_eq!(tc.eval_expr(&expr), Err(err));
        }

    );

    macro_rules! env (
        ( $( $id:expr => $v:expr),* ) => { {
            let mut env = Env::root();
            {
                use TypeTag::*;
                $( env.define(&string! {$id}, & node! {$v}); )*
            }
            env
        } }
    );

    #[test]
    fn test_simple() {
        assert_types_to!(
            Env::root(),
            Map(map! {
                "foo" => Int(42),
                "bar" => Str(string!("baz")),
                "quux" => List(list! {Int(1), Int(2), Int(3)})
            }), Ok(Map(map! {
                "foo" => Int,
                "bar" => Str,
                "quux" => List(node! {Int})
            }))
        );
    }

    #[test]
    fn test_list() {
        assert_types_to!(
            Env::root(),
            List(list! {Int(42), Int(3), Int(4)}),
            Ok(List(node! {Int}))
        );
        assert_types_to!(
            Env::root(),
            List(list! {Float(42.0), Float(3.0), Float(4.0) }),
            Ok(List(node! {Float}))
        );
        assert_types_to!(
            Env::root(),
            List(list! {Int(42), Float(2.0), Str(string!{"foo"})}),
            Ok(List(node! {Union(list! {Int, Float, Str})}))
        );
    }

    #[test]
    fn test_id() {
        assert_types_to!(
            env! {"foo" => Int},
            Id(string! {"foo"}),
            Ok(Int)
        );
        assert_types_to!(
            env! {"foo" => Int},
            Id(string! {"bar"}),
            Err(TypeError::Undefined(string! {"bar"}))
        );
    }

    #[test]
    fn test_dot() {
        assert_types_to!(
            env! {"x" => Map(map! {"foo" => TypeTag::Str})},
            Dot(node! {Id(string! {"x"})}, string! {"foo"}),
            Ok(Str)
        );

        assert_types_to!(
            env! {"x" => Map(map! {"foo" => TypeTag::Str})},
            Dot(node! {Id(string! {"x"})}, string! {"bar"}),
            Err(KeyError(map! {"foo" => TypeTag::Str}, string! {"bar"}))
        );

        assert_types_to!(
            Env::root(),
            Dot(node! {Int(42)}, string! {"bar"}),
            Err(NotAMap(node! {Int}))
        );
    }
}
