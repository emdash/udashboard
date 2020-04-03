


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

    fn assert_statement(text: &'static str, ast: Statement) {
        assert_eq!(
            grammar::StatementParser::new().parse(text).unwrap(),
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

    #[test]
    fn test_simple_statement() {
        assert_statement("fill <- ;", emit("fill", vec!{}));
        assert_statement(
            "moveto <- x, y;",
            emit("moveto", vec!{id("x"), id("y")})
        );

        assert_statement(
            "let y = x * 3 + 4;",
            def("y", bin(Add, bin(Mul, id("x"), Int(3)), Int(4)))
        );
    }

    #[test]
    fn test_block() {
        assert_statement("{}", statement_block(vec!{}));
        assert_statement(
            "{let x = 1; let y = 2; moveto <- x, y;}",
            statement_block(vec!{
                def("x", Int(1)),
                def("y", Int(2)),
                emit("moveto", vec!{ id("x"), id("y")})
            })
        );

        assert_statement(
            "{{} {{let x = 1; let y = 2;}}}",
            statement_block(vec!{
                statement_block(vec!{}),
                statement_block(vec!{def("x", Int(1)), def("y", Int(2))})
            })
        );

        assert_statement(
            "{{{{{let y = 1;}}}}}",
            def("y", Int(1))
        );
    }

    #[test]
    fn test_list_iter() {
        let test = r#"{
              let x = [1, 2, 3];
              for i in x {
                  moveto <- i, i;
                  circle <- 50.0;
              }

        }"#;

        assert_statement(test, statement_block(vec!{
            def("x", list(vec!{Int(1), Int(2), Int(3)})),
            list_iter("i", id("x"), statement_block(vec!{
                emit("moveto", vec!{id("i"), id("i")}),
                emit("circle", vec!{Float(50.0)})
            }))
        }));
    }


    #[test]
    fn test_map_iter() {
        let test = r#"{
              let x = {"a": 1, "b": 2, "c": 3};
              for (k, v) in x {
                  moveto <- v, v;
                  text <- k;
              }

        }"#;

        assert_statement(test, statement_block(vec!{
            def("x", map(vec!{
                (s("a"), Int(1)),
                (s("b"), Int(2)),
                (s("c"), Int(3))
            })),
            map_iter("k", "v", id("x"), statement_block(vec!{
                emit("moveto", vec!{id("v"), id("v")}),
                emit("text", vec!{id("k")})
            }))
        }));
    }

    #[test]
    fn test_guard() {
        assert_statement(
            "if a { text <- b; }",
            guard(
                vec!{(id("a"), emit("text", vec!{id("b")}))},
                None
            )
        );

        assert_statement(
            r#"if a { text <- b; } else { text <- "error"; }"#,
            guard(
                vec!{(id("a"), emit("text", vec!{id("b")}))},
                Some(emit("text", vec!{string("error")}))
            )
        );

        assert_statement(
            r#"if a {
               text <- "a";
            } elif b {
               text <- "b";
            } else {
               text <- "error";
            }"#,
            guard(
                vec!{
                    (id("a"), emit("text", vec!{string("a")})),
                    (id("b"), emit("text", vec!{string("b")})),
                },
                Some(emit("text", vec!{string("error")}))
            )
        );

        assert_statement(
            r#"if a {
               text <- "a";
            } elif b {
               text <- "b";
            } elif c {
               text <- "c";
            } else {
               text <- "error";
            }"#,
            guard(
                vec!{
                    (id("a"), emit("text", vec!{string("a")})),
                    (id("b"), emit("text", vec!{string("b")})),
                    (id("c"), emit("text", vec!{string("c")})),
                },
                Some(emit("text", vec!{string("error")}))
            )
        );

        assert_statement(
            r#"if a {
               text <- "a";
            } elif b {
               text <- "b";
            }"#,
            guard(
                vec!{
                    (id("a"), emit("text", vec!{string("a")})),
                    (id("b"), emit("text", vec!{string("b")})),
                },
                None
            )
        );

        assert_statement(
            r#"if a {
               text <- "a";
            } elif b {
               text <- "b";
            } elif c {
               text <- "c";
            }"#,
            guard(
                vec!{
                    (id("a"), emit("text", vec!{string("a")})),
                    (id("b"), emit("text", vec!{string("b")})),
                    (id("c"), emit("text", vec!{string("c")})),
                },
                None
            )
        );
    }
}
