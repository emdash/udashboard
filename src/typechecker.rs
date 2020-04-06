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


pub type TypeExpr = core::result::Result<Node<TypeTag>, TypeError>;
pub type TypeCheck = core::result::Result<(), TypeError>;


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
    pub fn lookup(fields: &Map<TypeTag>, name: &String) -> TypeExpr {
        if let Some(type_) = fields.get(name) {
            Ok(type_.clone())
        } else {
            Err(KeyError(fields.clone(), name.clone()))
        }
    }

    pub fn eval_expr(&self, expr: &Expr) -> TypeExpr {
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
            Expr::Lambda(args, ret, body) => self.eval_lambda(args, ret, body)
        }
    }

    pub fn eval_list(&self, items: &Seq<Expr>) -> TypeExpr {
        let items: Result<Seq<TypeTag>, TypeError> = items
            .iter()
            .map(|v| self.eval_expr(v))
            .collect();
        Ok(Node::new(TypeTag::List(Self::narrow(items?))))
    }

    pub fn eval_map(&self, fields: &Map<Expr>) -> TypeExpr {
        let fields: Result<Map<TypeTag>, TypeError> = fields
            .iter()
            .map(|(k, v)|  Ok((k.clone(), self.eval_expr(v)?)))
            .collect();
        Ok(Node::new(TypeTag::Map(fields?)))
    }

    pub fn eval_id(&self, name: &String) -> TypeExpr {
        let value = self.types.get(name);
        if let Some(type_) = value {
            Ok(type_.clone())
        } else {
            Err(Undefined(name.clone()))
        }
    }

    pub fn eval_dot(&self, obj: &Node<Expr>, field: &String) -> TypeExpr {
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
    ) -> TypeExpr {
        let env = Env::chain(&self.types);
        let sub = TypeChecker::new(env);
        for stmt in stmts {
            sub.check_statement(stmt)?
        }
        sub.eval_expr(ret)
    }

    pub fn eval_index(&self, lst: &Node<Expr>, index: &Node<Expr>) -> TypeExpr {
        let lst = self.eval_expr(lst)?;
        let index = self.eval_expr(index)?;

        if index.deref() == &TypeTag::Int {
            match lst.deref() {
                TypeTag::List(item) => Ok(item.clone()),
                _ => Err(NotAList(lst.clone()))
            }
        } else {
            Err(ListIndexMustBeInt(index))
        }
    }

    pub fn eval_cond(&self, cases: &Seq<(Expr, Expr)>) -> TypeExpr {
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
    ) -> TypeExpr {
        use TypeTag as TT;
        let l = self.eval_expr(l)?;
        let r = self.eval_expr(r)?;
        match (op, l.deref(), r.deref()) {
            (BinOp::Eq, a, b) if a == b => Ok(Node::new(a.clone())),
            (_, TT::Bool, TT::Bool)   => Ok(Node::new(TT::Bool)),
            (_, TT::Int, TT::Int)     => Ok(Node::new(TT::Int)),
            (_, TT::Float, TT::Float) => Ok(Node::new(TT::Float)),
            (_, TT::Str, TT::Str)     => Ok(Node::new(TT::Float)),
            _                         => Err(Mismatch(l, r))
        }
    }

    pub fn eval_unop(&self, op: UnOp, operand: &Node<Expr>) -> TypeExpr {
        use TypeTag as TT;
        let type_ = self.eval_expr(operand)?;
        let numeric = Node::new(TT::Union(vec! {
            Node::new(TT::Int),
            Node::new(TT::Float)
        }));
        match (op, type_.deref()) {
            (UnOp::Not, TT::Bool)  => Ok(Node::new(TT::Bool)),
            (UnOp::Not, _)         => Err(Mismatch(type_, Node::new(TT::Bool))),
            (UnOp::Neg, TT::Int)   => Ok(Node::new(TT::Int)),
            (UnOp::Neg, TT::Float) => Ok(Node::new(TT::Float)),
            (UnOp::Neg, _)         => Err(Mismatch(type_, numeric)),
            (UnOp::Abs, TT::Int)   => Ok(Node::new(TT::Int)),
            (UnOp::Abs, TT::Float) => Ok(Node::new(TT::Float)),
            (UnOp::Abs, _)         => Err(Mismatch(type_, numeric))
        }
    }

    fn eval_call(&self, func: &Node<Expr>, args: &Seq<Expr>) -> TypeExpr {
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

    pub fn eval_lambda(
        &self,
        args: &AList<TypeTag>,
        ret: &Node<TypeTag>,
        body: &Node<Expr>
    ) -> TypeExpr {
        let env = Env::chain(&self.types);
        env.import(args);
        let sub = TypeChecker::new(env);
        let body_type = sub.eval_expr(body)?;
        if body_type.deref() == ret.deref() {
            Ok(Node::new(TypeTag::Lambda(
                args.iter().map(|arg| arg.1.clone()).collect(),
                ret.clone()
            )))
        } else {
            Err(Mismatch(ret.clone(), body_type))
        }
    }

    // Check whether expr is a list, and return the item type.
    pub fn is_list(&self, expr: &Node<Expr>) -> TypeExpr {
        let result = self.eval_expr(expr)?;
        match result.deref() {
            TypeTag::List(item_type) => Ok(item_type.clone()),
            _ => Err(NotIterable(result))
        }
    }

    // Check whether expr is a map, return the union over all the value types.
    pub fn is_map(&self, expr: &Node<Expr>) -> TypeExpr {
        let result = self.eval_expr(expr)?;
        match result.deref() {
            TypeTag::Map(items) => Ok(Self::narrow(map_to_seq(items))),
            _ => Err(NotIterable(result))
        }
    }

    // Check whether expr is a bool.
    pub fn is_bool(&self, expr: &Node<Expr>) -> TypeCheck {
        let result = self.eval_expr(expr)?;
        match result.deref() {
            TypeTag::Bool => Ok(()),
            _ => Err(Mismatch(result, Node::new(TypeTag::Bool)))
        }
    }

    pub fn is_unit(&self, expr: &Node<Expr>) -> TypeCheck {
        let result = self.eval_expr(expr)?;
        match result.deref() {
            TypeTag::Unit => Ok(()),
            _ => Err(Mismatch(result, Node::new(TypeTag::Unit)))
        }
    }

    pub fn check_statement(
        &self,
        stmt: &Node<Statement>
    ) -> TypeCheck {
        match stmt.deref() {
            Statement::ExprForEffect(body) => {
                self.is_unit(body)?;
            },
            Statement::Emit(_op, exprs) => {
                // TODO: _op should be a recognizable cairo op.
                for expr in exprs {
                    self.eval_expr(expr)?;
                }
            },
            Statement::Def(name, val) => {
                self.types.define(name, &self.eval_expr(val)?);
            }
            Statement::ListIter(iter, lst, body) => {
                let item = self.is_list(lst)?;
                let env = Env::chain(&self.types);
                let sub = TypeChecker::new(env);
                sub.types.define(iter, &item);
                sub.check_statement(body)?;
            },
            Statement::MapIter(k, v, map, body) => {
                // TODO: raise proper error, rather than crashing.
                assert!(k != v, "cannot be the same");
                let item = self.is_map(map)?;
                let env = Env::chain(&self.types);
                let sub = TypeChecker::new(env);
                sub.types.define(k, &Node::new(TypeTag::Str));
                sub.types.define(v, &item);
                sub.check_statement(body)?;
            },
            Statement::While(cond, body) => {
                self.is_bool(cond)?;
                self.check_statement(body)?;
            },
            Statement::Guard(clauses, default) => {
                for clause in clauses {
                    let (pred, body) = clause.deref();
                    self.is_bool(&Node::new(pred.clone()))?;
                    self.check_statement(&Node::new(body.clone()))?;
                }
                if let Some(stmnt) = default {
                    self.check_statement(&stmnt)?;
                }
            }
        };
        Ok(())
    }

    pub fn check_program(&self, prog: Program) -> TypeCheck {
        for stmt in prog.code {
            self.check_statement(&stmt)?;
        }
        Ok(())
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    // XXX: we  can get rid of these now.
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
            #[allow(unused_imports)]
            let expr = {
                use Expr::*;
                $e
            };
            #[allow(unused_imports)]
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
            #[allow(unused_imports)]
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
            let env = Env::root();
            #[allow(unused_imports)]
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
            Err(Undefined(string! {"bar"}))
        );
    }

    #[test]
    fn test_dot() {
        assert_types_to!(
            env! {"x" => Map(map! {"foo" => Str})},
            Dot(node! {Id(string! {"x"})}, string! {"foo"}),
            Ok(Str)
        );

        assert_types_to!(
            env! {"x" => Map(map! {"foo" => Str})},
            Dot(node! {Id(string! {"x"})}, string! {"bar"}),
            Err(KeyError(map! {"foo" => Str}, string! {"bar"}))
        );

        assert_types_to!(
            Env::root(),
            Dot(node! {Int(42)}, string! {"bar"}),
            Err(NotAMap(node! {Int}))
        );

        assert_types_to!(
            env! {"x" => Map(map! {"foo" => Map(map! {"bar" => Int})})},
            Dot(
                node! {
                    Dot(
                        node! {
                            Id(string! {"x"})
                        },
                        string! {"foo"}
                    )
                },
                string! {"bar"}
            ),
            Ok(Int)
        );

        assert_types_to!(
            env! {"x" => Map(map! {"foo" => Map(map! {"bar" => Int})})},
            Dot(
                node! {
                    Dot(
                        node! {
                            Id(string! {"x"})
                        },
                        string! {"foo"}
                    )
                },
                string! {"baz"}
            ),
            Err(KeyError(map! {"bar" => Int}, string! {"baz"}))
        );
    }


    #[test]
    fn test_list_iter() {
        let tc = TypeChecker::new(
            env!{"x" => TypeTag::List(node!{TypeTag::Str})}
        );

        let statement = node! {list_iter(
            "i",
            id("x"),
            emit("show_text", vec!{id("i")})
        )};

        assert_eq!(tc.check_statement(&statement), Ok(()));

        let statement = node! {list_iter(
            "i",
            id("x"),
            Statement::ExprForEffect(Node::new(expr_block(vec!{}, id("i"))))
        )};

        assert_eq!(
            tc.check_statement(&statement),
            Err(Mismatch(Node::new(TypeTag::Str), Node::new(TypeTag::Unit)))
        );
    }


    #[test]
    fn test_map_iter() {
        let tc = TypeChecker::new(
            env!{"x" => TypeTag::Map(map!{"x" => Str})}
        );

        let statement = node! {map_iter(
            "k",
            "v",
            id("x"),
            emit("show_text", vec!{id("v")})
        )};

        assert_eq!(tc.check_statement(&statement), Ok(()));

        let statement = node! {map_iter(
            "k",
            "v",
            id("x"),
            Statement::ExprForEffect(Node::new(expr_block(vec!{}, id("v"))))
        )};

        assert_eq!(
            tc.check_statement(&statement),
            Err(Mismatch(Node::new(TypeTag::Str), Node::new(TypeTag::Unit)))
        );
    }


    #[test]
    fn test_lambda() {
        use crate::ast::BinOp::*;
        assert_types_to!(
            env!{},
            lambda(
                vec!{(s("x"), TypeTag::Int)},
                TypeTag::Int,
                bin(Add, id("x"), Expr::Int(4))
            ),
            Ok(TypeTag::Lambda(
                to_seq(vec!{TypeTag::Int}),
                node!{TypeTag::Int}
            ))
        );
    }
}
