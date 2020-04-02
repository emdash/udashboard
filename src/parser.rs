#[cfg(test)]
mod tests {
    use crate::grammar;
    use crate::ast;
    use crate::ast::*;
    use Expr::*;
    use ast::BinOp::*;
    use ast::UnOp::*;

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
        assert_parses_to("a + 3", bin(Add, Id(s("a")), Int(3)));
        assert_parses_to("3 * a", bin(Mul, Int(3), id("a")));
        assert_parses_to("\"foo\"", string("foo"));

    }

    #[test]
    fn test_list() {
        assert_parses_to("[]", list(vec!{}));
        assert_parses_to("[3]", list(vec!{Int(3)}));
        assert_parses_to("[3, 4, 5]", list(vec!{Int(3), Int(4), Int(5)}));
        assert_parses_to(
            "[3 + 4, 5]",
            list(vec!{bin(Add, Int(3), Int(4)), Int(5)})
        );
    }

    #[test]
    fn test_map() {
        assert_parses_to("{}", map(vec!{}));
        assert_parses_to(
            r#"{"foo": 1}"#,
            map(vec!{(s("foo"), Int(1))})
        );

        assert_parses_to(
            r#"{"foo": 1, "bar": 2}"#,
            map(vec!{
                (s("foo"), Int(1)),
                (s("bar"), Int(2))
            })
        );
    }

    #[test]
    fn test_terms() {
        assert_parses_to("42", Int(42));
        assert_parses_to("42.0", Float(42.0));
        assert_parses_to("(42)", Int(42));
        assert_parses_to("foo", id("foo"));
        assert_parses_to("-42", Int(-42));
        assert_parses_to("-42.0", Float(-42.0));
        assert_parses_to("-x", un(Neg, id("x")));
        assert_parses_to("- 42", un(Neg, Int(42)));
        assert_parses_to("- 42.0", un(Neg, Float(42.0)));
        assert_parses_to("-(42)", un(Neg, Int(42)));
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

    #[test]
    fn test_call() {
        assert_parses_to(
            "foo(a + 3, a and b)",
            call(
                id("foo"),
                vec! { bin(Add, id("a"), Int(3)), bin(And, id("a"), id("b")) }
            )
        );

        assert_parses_to(
            "x(a or b, y <= 3, y(-g(a * 7)))",
            call(
                id("x"),
                vec! {
                    bin(Or, id("a"), id("b")),
                    bin(Lte, id("y"), Int(3)),
                    call(
                        id("y"),
                        vec!{
                            un(
                                Neg,
                                call(
                                    id("g"),
                                    vec!{ bin(Mul, id("a"), Int(7)) }
                                )
                            )
                        }
                    )
                }
            )
        );
    }

    #[test]
    fn test_dot() {
        assert_parses_to(
            "foo.bar",
            dot(id("foo"), "bar")
        );

        assert_parses_to(
            "foo.bar.baz",
            dot(dot(id("foo"), "bar"), "baz")
        );

        assert_parses_to(
            "foo.bar()",
            call(dot(id("foo"), "bar"), vec! {})
        );
    }

    #[test]
    fn test_index() {
        assert_parses_to(
            "foo[0]",
            index(id("foo"), Int(0))
        );

        assert_parses_to(
            "foo[bar]",
            index(id("foo"), id("bar"))
        );

        assert_parses_to(
            "foo[3 + 10 * 5]",
            index(id("foo"), bin(Add, Int(3), bin(Mul, Int(10), Int(5))))
        );

        assert_parses_to(
            "foo[3][4]",
            index(index(id("foo"), Int(3)), Int(4))
        );

        assert_parses_to(
            "foo[3].bar.baz[5][6]",
            index(
                index(
                    dot(
                        dot(
                            index(id("foo"), Int(3)),
                            "bar"
                        ),
                        "baz"
                    ),
                    Int(5)
                ),
                Int(6)
            )
        );

        // XXX: allow parsing foo()[3] identically. For now, language
        // won't support returning function values, only passing them,
        // so this is okay.
        assert_parses_to(
            "(foo())[3]",
            index(call(id("foo"), vec!{}), Int(3))
        );

        // XXX: see above.
        assert_parses_to(
            "(foo()).bar",
            dot(call(id("foo"), vec!{}), "bar")
        );
    }
}
