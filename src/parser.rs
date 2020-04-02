#[cfg(test)]
mod tests {
    use crate::grammar;
    use crate::ast;
    use crate::ast::*;
    use Expr::*;
    use ast::BinOp::*;

    fn assert_parses_to(text: &'static str, ast: Expr) {
        assert_eq!(
            grammar::ExprParser::new().parse(text).unwrap(),
            ast
        );
    }

    #[test]
    fn test_basic() {
        assert_parses_to("3 + 4 * 5", bin(
            Add,
            Int(3),
            bin(Mul, Int(4), Int(5))
        ));
        assert_parses_to("a + 3", bin(Add, Id(String::from("a")), Int(3)));
        assert_parses_to("3 * a", bin(Mul, Int(3), id("a")));
    }

    #[test]
    fn test_terms() {
        assert_parses_to("42", Int(42));
        assert_parses_to("42.0", Float(42.0));
        assert_parses_to("(42)", Int(42));
        assert_parses_to("foo", id("foo"));
    }

    #[test]
    fn test_relational() {
        assert_parses_to("3 + 4 < 3 * 4", bin(
            Lt,
            bin(Add, Int(3), Int(4)),
            bin(Mul, Int(3), Int(4))
        ));

        assert_parses_to("3 + 4 > 3 * 4", bin(
            Gt,
            bin(Add, Int(3), Int(4)),
            bin(Mul, Int(3), Int(4))
        ));

        assert_parses_to("3 + 4 <= 3 * 4", bin(
            Lte,
            bin(Add, Int(3), Int(4)),
            bin(Mul, Int(3), Int(4))
        ));

        assert_parses_to("3 + 4 >= 3 * 4", bin(
            Gte,
            bin(Add, Int(3), Int(4)),
            bin(Mul, Int(3), Int(4))
        ));

        assert_parses_to("3 + 4 == 3 * 4", bin(
            Eq,
            bin(Add, Int(3), Int(4)),
            bin(Mul, Int(3), Int(4))
        ));
    }

    #[test]
    fn test_logic() {
        assert_parses_to(
            "x >= lower and x <= upper",
            bin(And,
                bin(Gte, id("x"), id("lower")),
                bin(Lte, id("x"), id("upper"))));

        assert_parses_to(
            "x >= 3 or x > 4 and x > 5",
            bin(And,
                bin(Or,
                    bin(Gte, id("x"), Int(3)),
                    bin(Gt, id("x"), Int(4))),
                bin(Gt, id("x"), Int(5))));

        assert_parses_to(
            "x >= 3 or (x > 4 and x > 5)",
            bin(Or,
                bin(Gte, id("x"), Int(3)),
                bin(And,
                    bin(Gt, id("x"), Int(4)),
                    bin(Gt, id("x"), Int(5)))));

        assert_parses_to(
            "(x >= 3) or (x > 4 and x > 5)",
            bin(Or,
                bin(Gte, id("x"), Int(3)),
                bin(And,
                    bin(Gt, id("x"), Int(4)),
                    bin(Gt, id("x"), Int(5)))));
    }
}

