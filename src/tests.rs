//
// Parser.
//

mod parser {
    use crate::grammar::ListsParser;

    #[test]
    fn nil() {
        let parser = ListsParser::new();
        let result = parser.parse("()").unwrap();
        assert!(result.len() == 1);
        assert!(result[0].is_nil());
    }

    #[test]
    fn list() {
        let parser = ListsParser::new();
        let result = parser.parse("(+ a b)").unwrap();
        assert!(result.len() == 1);
        assert!(result[0].is_pair());
    }

    #[test]
    fn sequence_of_lists() {
        let parser = ListsParser::new();
        let result = parser.parse("(+ a b) (- a b)").unwrap();
        assert!(result.len() == 2);
        assert!(result[0].is_pair());
        assert!(result[1].is_pair());
    }
}

//
// Intermediate representation.
//

mod ir {
    use crate::{grammar::ListsParser, ir::FunctionDefinition};
    use map_macro::btree_set;

    #[test]
    fn def_single_statement() {
        let parser = ListsParser::new();
        let atoms = parser.parse("(def ADD (A B) (+ A B))").unwrap();
        let defuns: Vec<_> = atoms
            .into_iter()
            .map(FunctionDefinition::try_from)
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(btree_set! {}, defuns[0].closure());
        let stmt = defuns[0].statements().iter().next().unwrap();
        assert_eq!(btree_set! {"A".into(), "B".into()}, stmt.closure());
    }

    #[test]
    fn def_with_lambda_with_external_bindings() {
        let parser = ListsParser::new();
        let atoms = parser.parse("(def test(a) ((\\ (b) (+ a b)) 1))").unwrap();
        let defuns: Vec<_> = atoms
            .into_iter()
            .map(FunctionDefinition::try_from)
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(btree_set! {}, defuns[0].closure());
        let stmt = defuns[0].statements().iter().next().unwrap();
        assert_eq!(btree_set! {"a".into()}, stmt.closure());
        let stmt = stmt.statements().next().unwrap();
        assert_eq!(btree_set! {"a".into()}, stmt.closure());
    }

    #[test]
    fn def_with_nested_lambda_with_external_bindings() {
        let parser = ListsParser::new();
        let atoms = parser
            .parse(
                r#"
                (def test(a)
                    ((\ (b)
                        ((\ (c) (+ a (+ b c ))) 1)) 2))
                "#,
            )
            .unwrap();
        let defuns: Vec<_> = atoms
            .into_iter()
            .map(FunctionDefinition::try_from)
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(btree_set! {}, defuns[0].closure());
        let stmt = defuns[0].statements().iter().next().unwrap();
        assert_eq!(btree_set! {"a".into()}, stmt.closure());
        let stmt = stmt.statements().next().unwrap();
        assert_eq!(btree_set! {"a".into()}, stmt.closure());
        let stmt = stmt.statements().next().unwrap();
        assert_eq!(btree_set! {"a".into(), "b".into()}, stmt.closure());
    }

    #[test]
    fn def_with_let_and_lambda_with_external_bindings() {
        let parser = ListsParser::new();
        let atoms = parser
            .parse("(def test(a) (let ((inc . (\\ (b) (+ a b)))) (- 2 (inc 1))))")
            .unwrap();
        let defuns: Vec<_> = atoms
            .into_iter()
            .map(FunctionDefinition::try_from)
            .collect::<Result<_, _>>()
            .unwrap();
        println!("{:?}", defuns);
        println!("{:?}", defuns[0].closure());
        let stmt = defuns[0].statements().iter().next().unwrap();
        println!("{:?}", stmt.closure());
        let stmt = stmt.statements().next().unwrap();
        println!("{:?}", stmt.closure());
    }
}

//
// Compiler.
//

mod compiler {
    use crate::{
        compiler::{Compiler, Context, LabelOrOpCode},
        grammar::ListsParser,
        ir::Statement,
        opcodes::{Immediate, OpCode},
    };

    #[test]
    fn single_builtin() {
        let parser = ListsParser::new();
        let atom = parser.parse("(+ 1 2)").unwrap().remove(0);
        let stmt: Statement = atom.try_into().unwrap();
        let mut context = Context::default();
        let mut compiler = Compiler::default();
        compiler.compile_statement(&mut context, &stmt).unwrap();
        assert_eq!(
            context.stream(),
            &[
                OpCode::Psh(Immediate::Number(2)).into(),
                OpCode::Psh(Immediate::Number(1)).into(),
                OpCode::Add.into(),
            ]
        );
    }

    #[test]
    fn nested_builtin() {
        let parser = ListsParser::new();
        let atom = parser.parse("(+ (- 3 4) 2)").unwrap().remove(0);
        let stmt: Statement = atom.try_into().unwrap();
        let mut context = Context::default();
        let mut compiler = Compiler::default();
        compiler.compile_statement(&mut context, &stmt).unwrap();
        assert_eq!(
            context.stream(),
            &[
                OpCode::Psh(Immediate::Number(2)).into(),
                OpCode::Psh(Immediate::Number(4)).into(),
                OpCode::Psh(Immediate::Number(3)).into(),
                OpCode::Sub.into(),
                OpCode::Add.into(),
            ]
        );
    }

    #[test]
    fn if_then() {
        let parser = ListsParser::new();
        let atom = parser.parse("(if (< 3 0) (+ 1 2))").unwrap().remove(0);
        let stmt: Statement = atom.try_into().unwrap();
        let mut context = Context::default();
        let mut compiler = Compiler::default();
        compiler.compile_statement(&mut context, &stmt).unwrap();
        assert_eq!(
            context.stream(),
            &[
                OpCode::Psh(Immediate::Number(0)).into(),
                OpCode::Psh(Immediate::Number(3)).into(),
                OpCode::Lt.into(),
                LabelOrOpCode::BranchIfNot("BEGIN_ELSE_0000".to_owned().into_boxed_str()),
                OpCode::Psh(Immediate::Number(2)).into(),
                OpCode::Psh(Immediate::Number(1)).into(),
                OpCode::Add.into(),
                LabelOrOpCode::Branch("END_ELSE_0001".to_owned().into_boxed_str()),
                OpCode::Psh(Immediate::Nil).into(),
            ]
        );
    }

    #[test]
    fn if_then_else() {
        let parser = ListsParser::new();
        let atom = parser
            .parse("(if (< 3 0) (+ 1 2) (- 4 2))")
            .unwrap()
            .remove(0);
        let stmt: Statement = atom.try_into().unwrap();
        let mut context = Context::default();
        let mut compiler = Compiler::default();
        compiler.compile_statement(&mut context, &stmt).unwrap();
        assert_eq!(
            context.stream(),
            &[
                OpCode::Psh(Immediate::Number(0)).into(),
                OpCode::Psh(Immediate::Number(3)).into(),
                OpCode::Lt.into(),
                LabelOrOpCode::BranchIfNot("BEGIN_ELSE_0000".to_owned().into_boxed_str()),
                OpCode::Psh(Immediate::Number(2)).into(),
                OpCode::Psh(Immediate::Number(1)).into(),
                OpCode::Add.into(),
                LabelOrOpCode::Branch("END_ELSE_0001".to_owned().into_boxed_str()),
                OpCode::Psh(Immediate::Number(2)).into(),
                OpCode::Psh(Immediate::Number(4)).into(),
                OpCode::Sub.into(),
            ]
        );
    }

    #[test]
    fn nested_if_then() {
        let parser = ListsParser::new();
        let atom = parser
            .parse("(if (< 3 0) (if (>= 1 4) (- 4 2)))")
            .unwrap()
            .remove(0);
        let stmt: Statement = atom.try_into().unwrap();
        let mut context = Context::default();
        let mut compiler = Compiler::default();
        compiler.compile_statement(&mut context, &stmt).unwrap();
        assert_eq!(
            context.stream(),
            &[
                OpCode::Psh(Immediate::Number(0)).into(),
                OpCode::Psh(Immediate::Number(3)).into(),
                OpCode::Lt.into(),
                LabelOrOpCode::BranchIfNot("BEGIN_ELSE_0000".to_owned().into_boxed_str()),
                OpCode::Psh(Immediate::Number(4)).into(),
                OpCode::Psh(Immediate::Number(1)).into(),
                OpCode::Ge.into(),
                LabelOrOpCode::BranchIfNot("BEGIN_ELSE_0001".to_owned().into_boxed_str()),
                OpCode::Psh(Immediate::Number(2)).into(),
                OpCode::Psh(Immediate::Number(4)).into(),
                OpCode::Sub.into(),
                LabelOrOpCode::Branch("END_ELSE_0002".to_owned().into_boxed_str()),
                OpCode::Psh(Immediate::Nil).into(),
                LabelOrOpCode::Branch("END_ELSE_0003".to_owned().into_boxed_str()),
                OpCode::Psh(Immediate::Nil).into(),
            ]
        );
    }

    #[test]
    fn let_binding_with_a_single_constant() {
        let parser = ListsParser::new();
        let atom = parser.parse("(let ((a . 1)) (+ a 1))").unwrap().remove(0);
        let stmt: Statement = atom.try_into().unwrap();
        let mut context = Context::default();
        let mut compiler = Compiler::default();
        compiler.compile_statement(&mut context, &stmt).unwrap();
        assert_eq!(
            context.stream(),
            &[
                OpCode::Psh(Immediate::Number(1)).into(),
                OpCode::Psh(Immediate::Number(1)).into(),
                OpCode::Get(2).into(),
                OpCode::Add.into(),
                OpCode::Rot(2).into(),
                OpCode::Pop(1).into(),
            ]
        )
    }

    #[test]
    fn let_binding_with_a_single_funcall() {
        let parser = ListsParser::new();
        let atom = parser
            .parse("(let ((a . (+ 1 2))) (+ a 1))")
            .unwrap()
            .remove(0);
        let stmt: Statement = atom.try_into().unwrap();
        let mut context = Context::default();
        let mut compiler = Compiler::default();
        compiler.compile_statement(&mut context, &stmt).unwrap();
        assert_eq!(
            context.stream(),
            &[
                OpCode::Psh(Immediate::Number(2)).into(),
                OpCode::Psh(Immediate::Number(1)).into(),
                OpCode::Add.into(),
                OpCode::Psh(Immediate::Number(1)).into(),
                OpCode::Get(2).into(),
                OpCode::Add.into(),
                OpCode::Rot(2).into(),
                OpCode::Pop(1).into(),
            ]
        )
    }

    #[test]
    fn let_binding_with_multiple_bindings() {
        let parser = ListsParser::new();
        let atom = parser
            .parse("(let ((a . (+ 1 2)) (b . 2)) (+ a b))")
            .unwrap()
            .remove(0);
        let stmt: Statement = atom.try_into().unwrap();
        let mut context = Context::default();
        let mut compiler = Compiler::default();
        compiler.compile_statement(&mut context, &stmt).unwrap();
        assert_eq!(
            context.stream(),
            &[
                OpCode::Psh(Immediate::Number(2)).into(),
                OpCode::Psh(Immediate::Number(1)).into(),
                OpCode::Add.into(),
                OpCode::Psh(Immediate::Number(2)).into(),
                OpCode::Get(1).into(),
                OpCode::Get(3).into(),
                OpCode::Add.into(),
                OpCode::Rot(3).into(),
                OpCode::Pop(2).into(),
            ]
        )
    }

    #[test]
    fn nested_let_bindings() {
        let parser = ListsParser::new();
        let atom = parser
            .parse("(let ((a . (+ 1 2))) (let ((b . (+ a 3))) (- a b)))")
            .unwrap()
            .remove(0);
        let stmt: Statement = atom.try_into().unwrap();
        let mut context = Context::default();
        let mut compiler = Compiler::default();
        compiler.compile_statement(&mut context, &stmt).unwrap();
        assert_eq!(
            context.stream(),
            &[
                OpCode::Psh(Immediate::Number(2)).into(),
                OpCode::Psh(Immediate::Number(1)).into(),
                OpCode::Add.into(),
                OpCode::Psh(Immediate::Number(3)).into(),
                OpCode::Get(2).into(),
                OpCode::Add.into(),
                OpCode::Get(1).into(),
                OpCode::Get(3).into(),
                OpCode::Sub.into(),
                OpCode::Rot(2).into(),
                OpCode::Pop(1).into(),
                OpCode::Rot(2).into(),
                OpCode::Pop(1).into(),
            ]
        )
    }

    #[test]
    fn lambda() {
        let parser = ListsParser::new();
        let atom = parser.parse("(\\ (a b) (+ a b))").unwrap().remove(0);
        let stmt: Statement = atom.try_into().unwrap();
        let mut context = Context::default();
        let mut compiler = Compiler::default();
        compiler.compile_statement(&mut context, &stmt).unwrap();
        assert_eq!(
            context.stream(),
            &[
                LabelOrOpCode::Funcall("LAMBDA_0000".into()).into(),
                OpCode::Pak(1).into()
            ]
        )
    }

    #[test]
    fn def_single_statement() {
        let parser = ListsParser::new();
        let atoms = parser.parse("(def ADD (A B) (+ A B))").unwrap();
        let compiler = Compiler::default();
        let (_, result) = compiler.compile(atoms).unwrap();
        assert_eq!(
            result,
            vec![
                OpCode::Rot(3),
                OpCode::Get(2),
                OpCode::Get(2),
                OpCode::Add,
                OpCode::Rot(3),
                OpCode::Pop(2),
                OpCode::Ret,
            ]
        );
    }

    #[test]
    fn def_multiple_statements() {
        let parser = ListsParser::new();
        let atoms = parser.parse("(def ADD (A B C) (+ A B) (- A C))").unwrap();
        let compiler = Compiler::default();
        let (_, result) = compiler.compile(atoms).unwrap();
        assert_eq!(
            result,
            vec![
                OpCode::Rot(4),
                OpCode::Get(2),
                OpCode::Get(2),
                OpCode::Add,
                OpCode::Pop(1),
                OpCode::Get(3),
                OpCode::Get(2),
                OpCode::Sub,
                OpCode::Rot(4),
                OpCode::Pop(3),
                OpCode::Ret,
            ]
        );
    }

    #[test]
    fn def_with_main() {
        let parser = ListsParser::new();
        let atoms = parser
            .parse("(def ADD (A B C) (+ A B) (- A C))(def main () (ADD 1 2 3))")
            .unwrap();
        let compiler = Compiler::default();
        let (_, result) = compiler.compile(atoms).unwrap();
        assert_eq!(
            result,
            vec![
                //
                // ADD.
                //
                OpCode::Rot(4),
                OpCode::Get(2),
                OpCode::Get(2),
                OpCode::Add,
                OpCode::Pop(1),
                OpCode::Get(3),
                OpCode::Get(2),
                OpCode::Sub,
                OpCode::Rot(4),
                OpCode::Pop(3),
                OpCode::Ret,
                //
                // Main.
                //
                OpCode::Psh(Immediate::Number(3)),
                OpCode::Psh(Immediate::Number(2)),
                OpCode::Psh(Immediate::Number(1)),
                OpCode::Psh(Immediate::Funcall(0)),
                OpCode::Call,
                OpCode::Ret,
            ]
        );
    }

    #[test]
    fn def_fibonacci() {
        let parser = ListsParser::new();
        let atoms = parser
            .parse("(def fib (N) (if (<= N 1) N (+ (fib (- N 1)) (fib (- N 2)))))")
            .unwrap();
        let compiler = Compiler::default();
        let (_, result) = compiler.compile(atoms).unwrap();
        assert_eq!(
            result,
            vec![
                OpCode::Rot(2),                     // [ret0, N]
                OpCode::Psh(Immediate::Number(1)),  // [ret0, N, 1]
                OpCode::Get(2),                     // [ret0, N, 1, N]
                OpCode::Le,                         // [ret0, N, T/nil]
                OpCode::Brn(3),                     // [ret0, N]
                OpCode::Get(1),                     // [ret0, N, N]
                OpCode::Br(12),                     //
                OpCode::Psh(Immediate::Number(2)),  // [ret0, N, 2]
                OpCode::Get(2),                     // [ret0, N, 1, N]
                OpCode::Sub,                        // [ret0, N, N-2]
                OpCode::Psh(Immediate::Funcall(0)), // [ret0, N, N-2, fun0]
                OpCode::Call,                       // [ret0, N, N-2, ret1]
                OpCode::Psh(Immediate::Number(1)),  // [ret0, N, R0, 1]
                OpCode::Get(3),                     // [ret0, N, R0, 1, N]
                OpCode::Sub,                        // [ret0, N, R0, N-1]
                OpCode::Psh(Immediate::Funcall(0)), // [ret0, N, R0, N-1, fun0]
                OpCode::Call,                       // [ret0, N, N-1, ret1]
                OpCode::Add,                        // [ret0, N, R0+R1]
                OpCode::Rot(2),                     // [ret0, R0+R1, N]
                OpCode::Pop(1),                     // [ret0, R0+R1]
                OpCode::Ret
            ]
        );
    }

    #[test]
    fn def_with_let_and_lambda() {
        let parser = ListsParser::new();
        let atoms = parser
            .parse("(def test(a) (let ((add . (\\ (b c) (+ b c)))) (- a (add 1 2))))")
            .unwrap();
        let compiler = Compiler::default();
        let (_, result) = compiler.compile(atoms).unwrap();
        assert_eq!(
            result,
            vec![
                //
                // Lambda.
                //
                OpCode::Rot(3),
                OpCode::Get(2),
                OpCode::Get(2),
                OpCode::Add,
                OpCode::Rot(3),
                OpCode::Pop(2),
                OpCode::Ret,
                //
                // Test.
                //
                OpCode::Rot(2),                     // [ret0, a]
                OpCode::Psh(Immediate::Funcall(0)), // [ret0, a, fun0]
                OpCode::Pak(1),                     // [ret0, a, pak0]
                OpCode::Psh(Immediate::Number(2)),  // [ret0, a, pak0, 2]
                OpCode::Psh(Immediate::Number(1)),  // [ret0, a, pak0, 2, 1]
                OpCode::Get(3),                     // [ret0, a, pak0, 2, 1, pak0]
                OpCode::Call,                       // [ret0, a, pak0, 2, 1, ret1]
                OpCode::Get(3),                     // [ret0, a, pak0, 3, a]
                OpCode::Sub,                        // [ret0, a, pak0, a-3]
                OpCode::Rot(2),                     // [ret0, a, a-3, pak0]
                OpCode::Pop(1),                     // [ret0, a, a-3]
                OpCode::Rot(2),                     // [ret0, a-3, a]
                OpCode::Pop(1),                     // [ret0, a-3]
                OpCode::Ret
            ]
        )
    }

    #[test]
    fn def_with_lambda_with_external_bindings() {
        let parser = ListsParser::new();
        let atoms = parser
            .parse(
                r#"
                (def test(a)
                    (let ((add . (\ (b)
                                    ((\ () (+ a b))))))
                        (add 1)))
                "#,
            )
            .unwrap();
        let compiler = Compiler::default();
        let (_, result) = compiler.compile(atoms).unwrap();
        assert_eq!(
            result,
            vec![
                //
                // (\ () (+ a b))
                //
                OpCode::Rot(3), // [ret0, a, b]
                OpCode::Get(1), // [ret0, a, b, b]
                OpCode::Get(3), // [ret0, a, b, b, a]
                OpCode::Add,    // [ret0, a, b, a+b]
                OpCode::Rot(3), // [ret0, a+b, a, b]
                OpCode::Pop(2), // [ret0, a+b]
                OpCode::Ret,    // [a+b]
                //
                // (\ (b) ((\ () (+ a b)))
                //
                OpCode::Rot(3),                     // [ret0, b, a]
                OpCode::Get(1),                     // [ret0, b, a, a]
                OpCode::Get(3),                     // [ret0, b, a, a, b]
                OpCode::Psh(Immediate::Funcall(0)), // [ret0, b, a, a, b, fun0]
                OpCode::Pak(3),                     // [ret0, b, a, pak0]
                OpCode::Call,                       // [ret0, b, a, a+b]
                OpCode::Rot(3),                     // [ret0, a+b, b, a]
                OpCode::Pop(2),                     // [ret0, a+b]
                OpCode::Ret,                        // [a+b]
                //
                // (def test ..)
                //
                OpCode::Rot(2),                     // [ret0, a]
                OpCode::Get(1),                     // [ret0, a, a]
                OpCode::Psh(Immediate::Funcall(7)), // [ret0, a, a, fun7]
                OpCode::Pak(2),                     // [ret0, a, pak0]
                OpCode::Psh(Immediate::Number(1)),  // [ret0, a, pak0, 1]
                OpCode::Get(2),                     // [ret0, a, pak0, 1, pak0]
                OpCode::Call,                       // [ret0, a, pak0, a+1]
                OpCode::Rot(2),                     // [ret0, a, a+1, pak0]
                OpCode::Pop(1),                     // [ret0, a, a+1]
                OpCode::Rot(2),                     // [ret0, a+1, a]
                OpCode::Pop(1),                     // [ret0, a+1]
                OpCode::Ret                         // [a+1]
            ]
        )
    }

    #[test]
    fn def_loop_with_tailcall_optimization() {
        let parser = ListsParser::new();
        let atoms = parser.parse("(def test() (test))").unwrap();
        let compiler = Compiler::default();
        let (_, result) = compiler.compile(atoms).unwrap();
        assert_eq!(result, vec![OpCode::Br(0)]);
    }

    #[test]
    fn def_loop_with_may_statements_with_tailcall_optimization() {
        let parser = ListsParser::new();
        let atoms = parser
            .parse(
                r#"
                (def test()
                    (+ 1 1)
                    (test))
                "#,
            )
            .unwrap();
        let compiler = Compiler::default();
        let (_, result) = compiler.compile(atoms).unwrap();
        assert_eq!(
            result,
            vec![
                OpCode::Psh(Immediate::Number(1)),
                OpCode::Psh(Immediate::Number(1)),
                OpCode::Add,
                OpCode::Pop(1),
                OpCode::Br(-4),
            ]
        );
    }

    #[test]
    fn def_if_then_with_tailcall_optimization() {
        let parser = ListsParser::new();
        let atoms = parser.parse("(def test(a) (if a (test (cdr a))))").unwrap();
        let compiler = Compiler::default();
        let (_, result) = compiler.compile(atoms).unwrap();
        assert_eq!(
            result,
            vec![
                OpCode::Rot(2),              // [ret0, a]
                OpCode::Get(1),              // [ret0, a, a]
                OpCode::Brn(7),              //
                OpCode::Get(1),              // [ret0, a, a]
                OpCode::Cdr,                 // [ret0, a, cdr(a)]
                OpCode::Rot(2),              // [ret0, cdr(a), a]
                OpCode::Pop(1),              // [ret0, cdr(a)]
                OpCode::Br(-6),              //
                OpCode::Psh(Immediate::Nil), // [ret0, a, nil]
                OpCode::Rot(2),              // [ret0, nil, a]
                OpCode::Pop(1),              // [ret0, nil]
                OpCode::Ret                  // [nil]
            ]
        );
    }

    #[test]
    fn def_if_then_else_with_tailcall_optimization() {
        let parser = ListsParser::new();
        let atoms = parser
            .parse(
                r#"
                (def test(a)
                    (if a
                        (test (cdr a))
                        (test (car a))
                    ))
                "#,
            )
            .unwrap();
        let compiler = Compiler::default();
        let (_, result) = compiler.compile(atoms).unwrap();
        assert_eq!(
            result,
            vec![
                OpCode::Rot(2),  // [ret0, a]
                OpCode::Get(1),  // [ret0, a, a]
                OpCode::Brn(7),  //
                OpCode::Get(1),  // [ret0, a, a]
                OpCode::Cdr,     // [ret0, a, cdr(a)]
                OpCode::Rot(2),  // [ret0, cdr(a), a]
                OpCode::Pop(1),  // [ret0, cdr(a)]
                OpCode::Br(-6),  //
                OpCode::Get(1),  // [ret0, a, a]
                OpCode::Car,     // [ret0, a, car(a)]
                OpCode::Rot(2),  // [ret0, cdr(a), a]
                OpCode::Pop(1),  // [ret0, cdr(a)]
                OpCode::Br(-11), //
            ]
        );
    }
}
