


#[cfg(test)]
mod tests {
    use crate::grammar;
    use crate::ast::*;
    use BinOp::*;

    fn assert_parses_to(text: &'static str, ast: Expr) {
        assert_eq!(
            grammar::ExprParser::new().parse(text).unwrap(),
            ast
        );
    }

    #[test]
    fn test_terms() {
        assert_parses_to("42", Expr::Int(42));
        assert_parses_to("42.0", Expr::Float(42.0));
        assert_parses_to("(42)", Expr::Int(42));
        assert_parses_to("foo", Expr::Id(String::from("foo")));
    }

    #[test]
    fn test_relational() {
        assert_parses_to("3 + 4 < 3 * 4", bin(
            Lt,
            bin(Add, Expr::Int(3), Expr::Int(4)),
            bin(Mul, Expr::Int(3), Expr::Int(4))
        ));

        assert_parses_to("3 + 4 > 3 * 4", bin(
            Gt,
            bin(Add, Expr::Int(3), Expr::Int(4)),
            bin(Mul, Expr::Int(3), Expr::Int(4))
        ));

        assert_parses_to("3 + 4 <= 3 * 4", bin(
            Lte,
            bin(Add, Expr::Int(3), Expr::Int(4)),
            bin(Mul, Expr::Int(3), Expr::Int(4))
        ));

        assert_parses_to("3 + 4 >= 3 * 4", bin(
            Gte,
            bin(Add, Expr::Int(3), Expr::Int(4)),
            bin(Mul, Expr::Int(3), Expr::Int(4))
        ));

        assert_parses_to("3 + 4 == 3 * 4", bin(
            Eq,
            bin(Add, Expr::Int(3), Expr::Int(4)),
            bin(Mul, Expr::Int(3), Expr::Int(4))
        ));
    }
}

