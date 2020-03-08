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
    NotImplemented
}


use TypeError::*;


pub type TypeCheck = core::result::Result<Node<TypeTag>, TypeError>;


pub struct TypeChecker {
    types: Env<TypeTag>,
}


impl TypeChecker {
    pub fn new(env: Env<TypeTag>) -> TypeChecker {
        TypeChecker { types: env }
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
            Expr::Block(_, ret)      => self.eval_expr(ret),
            Expr::Op(op, args)       => self.eval_op(op, args),
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

    pub fn eval_op(&self, name: &String, args: &Seq<Expr>) -> TypeCheck {
        Err(NotImplemented)
    }

    pub fn eval_lambda(&self, args: &AList<TypeTag>, body: &Node<Expr>) -> TypeCheck {
        Err(NotImplemented)
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
            let mut env = Env::new();
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
            Env::new(),
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
            Env::new(),
            List(list! {Int(42), Int(3), Int(4)}),
            Ok(List(node! {Int}))
        );
        assert_types_to!(
            Env::new(),
            List(list! {Float(42.0), Float(3.0), Float(4.0) }),
            Ok(List(node! {Float}))
        );
        assert_types_to!(
            env! {},
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
            Env::new(),
            Dot(node! {Int(42)}, string! {"bar"}),
            Err(NotAMap(node! {Int}))
        );
    }
}
