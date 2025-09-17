use std::rc::Rc;

use crate::{
    atom::Atom,
    compiler::{Artifacts, CompilerTrait},
    error::Error,
};

//
// Null compiler.
//

#[derive(Clone)]
struct NullCompiler;

impl CompilerTrait for NullCompiler {
    fn eval(self, _: Rc<Atom>, _: usize, _: bool) -> Result<Rc<Atom>, Error> {
        Err(Error::NotSupported)
    }

    fn expand(self, _: Rc<Atom>, _: usize, _: bool) -> Result<Rc<Atom>, Error> {
        Err(Error::NotSupported)
    }

    fn load(&mut self, _: Rc<Atom>) -> Result<(), Error> {
        Ok(())
    }

    fn compile(self) -> Result<Artifacts, Error> {
        Ok(Default::default())
    }
}

//
// Intermediate representation.
//

mod ir {
    use std::{collections::HashSet, rc::Rc};

    use crate::{atom::Atom, error::Error, grammar::ListsParser, ir::TopLevelStatement};
    use map_macro::btree_set;

    fn parse(stmt: &str) -> Vec<Rc<Atom>> {
        let parser = ListsParser::new();
        let mut compiler = super::NullCompiler;
        parser.parse(&mut compiler, stmt).unwrap()
    }

    #[test]
    fn def_single_statement() -> Result<(), Error> {
        let atoms = parse("(def ADD (A B) (+ A B))");
        let defuns: Vec<_> = atoms
            .into_iter()
            .map(|v| TopLevelStatement::from_atom(v, &HashSet::default()))
            .collect::<Result<_, _>>()?;
        assert_eq!(btree_set! {}, defuns[0].closure());
        let stmt = defuns[0].statements().iter().next().unwrap();
        assert_eq!(btree_set! {"A".into(), "B".into()}, stmt.closure());
        Ok(())
    }

    #[test]
    fn def_with_lambda_with_external_bindings() -> Result<(), Error> {
        let atoms = parse("(def test(a) ((\\ (b) (+ a b)) 1))");
        let defuns: Vec<_> = atoms
            .into_iter()
            .map(|v| TopLevelStatement::from_atom(v, &HashSet::default()))
            .collect::<Result<_, _>>()?;
        assert_eq!(btree_set! {}, defuns[0].closure());
        let stmt = defuns[0].statements().iter().next().unwrap();
        assert_eq!(btree_set! {"a".into()}, stmt.closure());
        let stmt = stmt.statements().next().unwrap();
        assert_eq!(btree_set! {"a".into()}, stmt.closure());
        Ok(())
    }

    #[test]
    fn def_with_nested_lambda_with_external_bindings() -> Result<(), Error> {
        let atoms = parse(
            r#"
            (def test(a)
                ((\ (b)
                    ((\ (c) (+ a (+ b c ))) 1)) 2))
            "#,
        );
        let defuns: Vec<_> = atoms
            .into_iter()
            .map(|v| TopLevelStatement::from_atom(v, &HashSet::default()))
            .collect::<Result<_, _>>()?;
        assert_eq!(btree_set! {}, defuns[0].closure());
        let stmt = defuns[0].statements().iter().next().unwrap();
        assert_eq!(btree_set! {"a".into()}, stmt.closure());
        let stmt = stmt.statements().next().unwrap();
        assert_eq!(btree_set! {"a".into()}, stmt.closure());
        let stmt = stmt.statements().next().unwrap();
        assert_eq!(btree_set! {"a".into(), "b".into()}, stmt.closure());
        Ok(())
    }

    #[test]
    fn def_with_let_and_lambda_with_external_bindings() -> Result<(), Error> {
        let atoms = parse("(def test(a) (let ((inc . (\\ (b) (+ a b)))) (- 2 (inc 1))))");
        let defuns: Vec<_> = atoms
            .into_iter()
            .map(|v| TopLevelStatement::from_atom(v, &HashSet::default()))
            .collect::<Result<_, _>>()?;
        println!("{:?}", defuns);
        println!("{:?}", defuns[0].closure());
        let stmt = defuns[0].statements().iter().next().unwrap();
        println!("{:?}", stmt.closure());
        let stmt = stmt.statements().next().unwrap();
        println!("{:?}", stmt.closure());
        Ok(())
    }
}

//
// Compiler.
//

mod compiler {
    use std::collections::HashSet;

    use crate::{
        compiler::{Artifacts, Compiler, CompilerTrait, Context, LabelOrOpCode},
        error::Error,
        grammar::ListsParser,
        ir::Statement,
        opcodes::{Arity, Immediate, OpCode},
    };

    fn parse(stmt: &str) -> Result<Context, Error> {
        //
        // Parse the statement, using a Null compiler.
        //
        let mut compiler = super::NullCompiler;
        let parser = ListsParser::new();
        let mut atoms = parser
            .parse(&mut compiler, stmt)
            .map_err(|e| Error::Parse(e.to_string()))?;
        let stmt = Statement::from_atom(atoms.remove(0), &HashSet::default())?;
        //
        // Compile the statement.
        //
        let mut compiler = Compiler::default();
        let mut context = Context::default();
        compiler.compile_statement(&mut context, &stmt)?;
        Ok(context)
    }

    fn compile(stmt: &str) -> Result<Artifacts, Error> {
        let parser = ListsParser::new();
        let mut compiler = Compiler::default();
        let _ = parser
            .parse(&mut compiler, stmt)
            .map_err(|e| Error::Parse(e.to_string()))?;
        compiler.compile()
    }

    fn compile_with_operators(stmt: &str) -> Result<Artifacts, Error> {
        let parser = ListsParser::new();
        let mut compiler = Compiler::default();
        compiler.lift_operators()?;
        let _ = parser
            .parse(&mut compiler, stmt)
            .map_err(|e| Error::Parse(e.to_string()))?;
        compiler.compile()
    }

    #[test]
    fn single_builtin() -> Result<(), Error> {
        let context = parse("(+ 1 2)")?;
        assert_eq!(
            context.stream(),
            &[
                OpCode::Psh(Immediate::Number(2)).into(),
                OpCode::Psh(Immediate::Number(1)).into(),
                OpCode::Add.into(),
            ]
        );
        Ok(())
    }

    #[test]
    fn nested_builtin() -> Result<(), Error> {
        let context = parse("(+ (- 3 4) 2)")?;
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
        Ok(())
    }

    #[test]
    fn if_then() -> Result<(), Error> {
        let context = parse("(if (< 3 0) (+ 1 2))")?;
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
        Ok(())
    }

    #[test]
    fn if_then_else() -> Result<(), Error> {
        let context = parse("(if (< 3 0) (+ 1 2) (- 4 2))")?;
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
        Ok(())
    }

    #[test]
    fn nested_if_then() -> Result<(), Error> {
        let context = parse("(if (< 3 0) (if (>= 1 4) (- 4 2)))")?;
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
        Ok(())
    }

    #[test]
    fn let_binding_with_a_single_constant() -> Result<(), Error> {
        let context = parse("(let ((a . 1)) (+ a 1))")?;
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
        );
        Ok(())
    }

    #[test]
    fn let_binding_with_a_single_funcall() -> Result<(), Error> {
        let context = parse("(let ((a . (+ 1 2))) (+ a 1))")?;
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
        );
        Ok(())
    }

    #[test]
    fn let_binding_with_multiple_bindings() -> Result<(), Error> {
        let context = parse("(let ((a . (+ 1 2)) (b . 2)) (+ a b))")?;
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
        );
        Ok(())
    }

    #[test]
    fn nested_let_bindings() -> Result<(), Error> {
        let context = parse("(let ((a . (+ 1 2))) (let ((b . (+ a 3))) (- a b)))")?;
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
        );
        Ok(())
    }

    #[test]
    fn lambda() -> Result<(), Error> {
        let context = parse("(\\ (a b) (+ a b))")?;
        assert_eq!(
            context.stream(),
            &[
                LabelOrOpCode::Funcall("LAMBDA_0000".into()).into(),
                OpCode::Pak(1).into()
            ]
        );
        Ok(())
    }

    #[test]
    fn quote() -> Result<(), Error> {
        let context = parse("(car '(1 2 3))")?;
        assert_eq!(
            context.stream(),
            &[
                OpCode::Psh(Immediate::Nil).into(),
                OpCode::Psh(Immediate::Number(3)).into(),
                OpCode::Cons.into(),
                OpCode::Psh(Immediate::Number(2)).into(),
                OpCode::Cons.into(),
                OpCode::Psh(Immediate::Number(1)).into(),
                OpCode::Cons.into(),
                OpCode::Car.into()
            ]
        );
        Ok(())
    }

    #[test]
    fn backquote_with_unquoted_list() -> Result<(), Error> {
        let context = parse("(car `(1 ,(+ 1 2) 3))")?;
        assert_eq!(
            context.stream(),
            &[
                OpCode::Psh(Immediate::Nil).into(),
                OpCode::Psh(Immediate::Number(3)).into(),
                OpCode::Cons.into(),
                OpCode::Psh(Immediate::Number(2)).into(),
                OpCode::Psh(Immediate::Number(1)).into(),
                OpCode::Add.into(),
                OpCode::Cons.into(),
                OpCode::Psh(Immediate::Number(1)).into(),
                OpCode::Cons.into(),
                OpCode::Car.into()
            ]
        );
        Ok(())
    }

    #[test]
    fn backquote_with_unquoted_symbol() -> Result<(), Error> {
        let context = parse("(car `(1 ,V 3))")?;
        assert_eq!(
            context.stream(),
            &[
                OpCode::Psh(Immediate::Nil).into(),
                OpCode::Psh(Immediate::Number(3)).into(),
                OpCode::Cons.into(),
                LabelOrOpCode::Funcall("V".into()),
                OpCode::Cons.into(),
                OpCode::Psh(Immediate::Number(1)).into(),
                OpCode::Cons.into(),
                OpCode::Car.into()
            ]
        );
        Ok(())
    }

    #[test]
    fn quote_with_double_unquote() -> Result<(), Error> {
        let res = parse("(car `(1 ,(,V) 3))");
        assert!(matches!(res, Err(Error::UnquoteOutsideBackquote(_))));
        Ok(())
    }

    #[test]
    fn nested_backquote_unquote() -> Result<(), Error> {
        let context = parse("(car `(1 ,(car `(1 ,V 2)) 3))")?;
        assert_eq!(
            context.stream(),
            &[
                OpCode::Psh(Immediate::Nil).into(),
                OpCode::Psh(Immediate::Number(3)).into(),
                OpCode::Cons.into(),
                OpCode::Psh(Immediate::Nil).into(),
                OpCode::Psh(Immediate::Number(2)).into(),
                OpCode::Cons.into(),
                LabelOrOpCode::Funcall("V".into()),
                OpCode::Cons.into(),
                OpCode::Psh(Immediate::Number(1)).into(),
                OpCode::Cons.into(),
                OpCode::Car.into(),
                OpCode::Cons.into(),
                OpCode::Psh(Immediate::Number(1)).into(),
                OpCode::Cons.into(),
                OpCode::Car.into()
            ]
        );
        Ok(())
    }

    #[test]
    fn def_single_statement() -> Result<(), Error> {
        let result = compile("(def ADD (A B) (+ A B))")?;
        assert_eq!(
            result.opcodes(),
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
        Ok(())
    }

    #[test]
    fn def_multiple_statements() -> Result<(), Error> {
        let result = compile("(def ADD (A B C) (+ A B) (- A C))")?;
        assert_eq!(
            result.opcodes(),
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
        Ok(())
    }

    #[test]
    fn def_with_main() -> Result<(), Error> {
        let result = compile(
            r#"
            (def ADD (A B C) (+ A B) (- A C))
            (def main () (ADD 1 2 3))
            "#,
        )?;
        assert_eq!(
            result.opcodes(),
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
                OpCode::Psh(Immediate::Funcall(0, Arity::Some(3))),
                OpCode::Call(3),
                OpCode::Ret,
            ]
        );
        Ok(())
    }

    #[test]
    fn def_fibonacci() -> Result<(), Error> {
        let result = compile("(def fib (N) (if (<= N 1) N (+ (fib (- N 1)) (fib (- N 2)))))")?;
        assert_eq!(
            result.opcodes(),
            vec![
                OpCode::Rot(2),                                     // [ret0, N]
                OpCode::Psh(Immediate::Number(1)),                  // [ret0, N, 1]
                OpCode::Get(2),                                     // [ret0, N, 1, N]
                OpCode::Le,                                         // [ret0, N, T/nil]
                OpCode::Brn(3),                                     // [ret0, N]
                OpCode::Get(1),                                     // [ret0, N, N]
                OpCode::Br(12),                                     //
                OpCode::Psh(Immediate::Number(2)),                  // [ret0, N, 2]
                OpCode::Get(2),                                     // [ret0, N, 1, N]
                OpCode::Sub,                                        // [ret0, N, N-2]
                OpCode::Psh(Immediate::Funcall(0, Arity::Some(1))), // [ret0, N, N-2, fun0]
                OpCode::Call(1),                                    // [ret0, N, N-2, ret1]
                OpCode::Psh(Immediate::Number(1)),                  // [ret0, N, R0, 1]
                OpCode::Get(3),                                     // [ret0, N, R0, 1, N]
                OpCode::Sub,                                        // [ret0, N, R0, N-1]
                OpCode::Psh(Immediate::Funcall(0, Arity::Some(1))), // [ret0, N, R0, N-1, fun0]
                OpCode::Call(1),                                    // [ret0, N, N-1, ret1]
                OpCode::Add,                                        // [ret0, N, R0+R1]
                OpCode::Rot(2),                                     // [ret0, R0+R1, N]
                OpCode::Pop(1),                                     // [ret0, R0+R1]
                OpCode::Ret
            ]
        );
        Ok(())
    }

    #[test]
    fn def_with_let_and_lambda() -> Result<(), Error> {
        let result = compile("(def test(a) (let ((add . (\\ (b c) (+ b c)))) (- a (add 1 2))))")?;
        assert_eq!(
            result.opcodes(),
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
                OpCode::Rot(2),                                     // [ret0, a]
                OpCode::Psh(Immediate::Funcall(0, Arity::Some(2))), // [ret0, a, fun0]
                OpCode::Pak(1),                                     // [ret0, a, pak0]
                OpCode::Psh(Immediate::Number(2)),                  // [ret0, a, pak0, 2]
                OpCode::Psh(Immediate::Number(1)),                  // [ret0, a, pak0, 2, 1]
                OpCode::Get(3),                                     // [ret0, a, pak0, 2, 1, pak0]
                OpCode::Call(2),                                    // [ret0, a, pak0, 2, 1, ret1]
                OpCode::Get(3),                                     // [ret0, a, pak0, 3, a]
                OpCode::Sub,                                        // [ret0, a, pak0, a-3]
                OpCode::Rot(2),                                     // [ret0, a, a-3, pak0]
                OpCode::Pop(1),                                     // [ret0, a, a-3]
                OpCode::Rot(2),                                     // [ret0, a-3, a]
                OpCode::Pop(1),                                     // [ret0, a-3]
                OpCode::Ret
            ]
        );
        Ok(())
    }

    #[test]
    fn def_with_lambda_with_external_bindings() -> Result<(), Error> {
        let result = compile(
            r#"
            (def test(a)
                (let ((add . (\ (b)
                                ((\ () (+ a b))))))
                    (add 1)))
            "#,
        )?;
        assert_eq!(
            result.opcodes(),
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
                OpCode::Rot(3),                                  // [ret0, b, a]
                OpCode::Get(1),                                  // [ret0, b, a, a]
                OpCode::Get(3),                                  // [ret0, b, a, a, b]
                OpCode::Psh(Immediate::Funcall(0, Arity::None)), // [ret0, b, a, a, b, fun0]
                OpCode::Pak(3),                                  // [ret0, b, a, pak0]
                OpCode::Call(0),                                 // [ret0, b, a, a+b]
                OpCode::Rot(3),                                  // [ret0, a+b, b, a]
                OpCode::Pop(2),                                  // [ret0, a+b]
                OpCode::Ret,                                     // [a+b]
                //
                // (def test ..)
                //
                OpCode::Rot(2),                                     // [ret0, a]
                OpCode::Get(1),                                     // [ret0, a, a]
                OpCode::Psh(Immediate::Funcall(7, Arity::Some(1))), // [ret0, a, a, fun7]
                OpCode::Pak(2),                                     // [ret0, a, pak0]
                OpCode::Psh(Immediate::Number(1)),                  // [ret0, a, pak0, 1]
                OpCode::Get(2),                                     // [ret0, a, pak0, 1, pak0]
                OpCode::Call(1),                                    // [ret0, a, pak0, a+1]
                OpCode::Rot(2),                                     // [ret0, a, a+1, pak0]
                OpCode::Pop(1),                                     // [ret0, a, a+1]
                OpCode::Rot(2),                                     // [ret0, a+1, a]
                OpCode::Pop(1),                                     // [ret0, a+1]
                OpCode::Ret                                         // [a+1]
            ]
        );
        Ok(())
    }

    #[test]
    fn def_loop_with_tailcall_optimization() -> Result<(), Error> {
        let result = compile("(def test() (test))")?;
        assert_eq!(result.opcodes(), vec![OpCode::Br(0)]);
        Ok(())
    }

    #[test]
    fn def_loop_with_many_statements_with_tailcall_optimization() -> Result<(), Error> {
        let result = compile(
            r#"
            (def test()
                (+ 1 1)
                (test))
            "#,
        )?;
        assert_eq!(
            result.opcodes(),
            vec![
                OpCode::Psh(Immediate::Number(1)),
                OpCode::Psh(Immediate::Number(1)),
                OpCode::Add,
                OpCode::Pop(1),
                OpCode::Br(-4),
            ]
        );
        Ok(())
    }

    #[test]
    fn def_if_then_with_tailcall_optimization() -> Result<(), Error> {
        let result = compile("(def test (a b) (if a (test (cdr a) (cdr b))))")?;
        assert_eq!(
            result.opcodes(),
            vec![
                OpCode::Rot(3),              // [ret0, b, a]
                OpCode::Get(1),              // [ret0, b, a, a]
                OpCode::Brn(9),              //
                OpCode::Get(2),              // [ret0, b, a, a]
                OpCode::Cdr,                 // [ret0, b, a, cdr(b)]
                OpCode::Get(2),              // [ret0, b, a, cdr(b), a]
                OpCode::Cdr,                 // [ret0, b, a, cdr(b), cdr(a)]
                OpCode::Rtm(4, 2),           // [ret0, cdr(b), cdr(a), b, a, ]
                OpCode::Pop(2),              // [ret0, cdr(b), cdr(a)]
                OpCode::Br(-8),              //
                OpCode::Psh(Immediate::Nil), // [ret0, b, a, nil]
                OpCode::Rot(3),              // [ret0, nil, b, a]
                OpCode::Pop(2),              // [ret0, nil]
                OpCode::Ret                  // [nil]
            ]
        );
        Ok(())
    }

    #[test]
    fn def_if_then_else_with_tailcall_optimization() -> Result<(), Error> {
        let result = compile(
            r#"
            (def test(a)
                (if a
                    (test (cdr a))
                    (test (car a))
                ))
            "#,
        )?;
        assert_eq!(
            result.opcodes(),
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
        Ok(())
    }

    #[test]
    fn def_capture_all_arguments_with_main() -> Result<(), Error> {
        let result = compile(
            r#"
            (def count_args_r (A)
                (if A
                    (+ 1 (count_args_r (cdr A)))
                    0))

            (def count_args A (count_args_r A))

            (def main ()
                (count_args 1 2 3 4))
            "#,
        )?;
        assert_eq!(
            result.opcodes(),
            vec![
                //
                // count_args_r
                //
                OpCode::Rot(2),
                OpCode::Get(1),
                OpCode::Brn(8),
                OpCode::Get(1),
                OpCode::Cdr,
                OpCode::Psh(Immediate::Funcall(0, Arity::Some(1))),
                OpCode::Call(1),
                OpCode::Psh(Immediate::Number(1)),
                OpCode::Add,
                OpCode::Br(2),
                OpCode::Psh(Immediate::Number(0)),
                OpCode::Rot(2),
                OpCode::Pop(1),
                OpCode::Ret,
                //
                // count_args
                //
                OpCode::Rot(2),
                OpCode::Get(1),
                OpCode::Psh(Immediate::Funcall(0, Arity::Some(1))),
                OpCode::Call(1),
                OpCode::Rot(2),
                OpCode::Pop(1),
                OpCode::Ret,
                //
                // main
                //
                OpCode::Psh(Immediate::Number(4)),
                OpCode::Psh(Immediate::Number(3)),
                OpCode::Psh(Immediate::Number(2)),
                OpCode::Psh(Immediate::Number(1)),
                OpCode::Psh(Immediate::Funcall(14, Arity::All)),
                OpCode::Call(4),
                OpCode::Ret
            ]
        );
        Ok(())
    }

    #[test]
    fn def_empty_capture_with_main() -> Result<(), Error> {
        let result = compile(
            r#"
            (def count_args A (if A 1 0))

            (def main ()
                (count_args))
            "#,
        )?;
        assert_eq!(
            result.opcodes(),
            vec![
                //
                // count_args
                //
                OpCode::Rot(2),
                OpCode::Get(1),
                OpCode::Brn(3),
                OpCode::Psh(Immediate::Number(1)),
                OpCode::Br(2),
                OpCode::Psh(Immediate::Number(0)),
                OpCode::Rot(2),
                OpCode::Pop(1),
                OpCode::Ret,
                //
                // main
                //
                OpCode::Psh(Immediate::Funcall(0, Arity::All)),
                OpCode::Call(0),
                OpCode::Ret
            ]
        );
        Ok(())
    }

    #[test]
    fn def_remainder_capture_with_main() -> Result<(), Error> {
        let result = compile(
            r#"
            (def select (A B . C) (if C A B))

            (def main ()
                (select 1 2 3 4))
            "#,
        )?;
        assert_eq!(
            result.opcodes(),
            vec![
                //
                // select().
                //
                OpCode::Rot(4), // [ret0, c, b, a]
                OpCode::Get(3), // [ret0, c, b, a, c]
                OpCode::Brn(3), //
                OpCode::Get(1), // [ret0, c, b, a, a]
                OpCode::Br(2),  //
                OpCode::Get(2), // [ret0, c, b, a, b]
                OpCode::Rot(4), // [ret0, res0, c, b, a]
                OpCode::Pop(3), // [ret0, res0]
                OpCode::Ret,    // [res0]
                //
                // main().
                //
                OpCode::Psh(Immediate::Number(4)),
                OpCode::Psh(Immediate::Number(3)),
                OpCode::Psh(Immediate::Number(2)),
                OpCode::Psh(Immediate::Number(1)),
                OpCode::Psh(Immediate::Funcall(0, Arity::SomeWithRem(2))),
                OpCode::Call(4),
                OpCode::Ret
            ]
        );
        Ok(())
    }

    #[test]
    fn def_operator_currying_with_main() -> Result<(), Error> {
        let result = compile_with_operators(
            r#"
            (def incr (A)
                (let ((+1 . (+ 1)))
                    (+1 A)))

            (def main ()
                (incr 1))
            "#,
        )?;
        assert_eq!(
            result.opcodes(),
            vec![
                //
                // +().
                //
                OpCode::Rot(3),
                OpCode::Get(2),
                OpCode::Get(2),
                OpCode::Add,
                OpCode::Rot(3),
                OpCode::Pop(2),
                OpCode::Ret,
                //
                // incr().
                //
                OpCode::Rot(2),                                     // [ret0, A]
                OpCode::Psh(Immediate::Number(1)),                  // [ret0, A, 1]
                OpCode::Psh(Immediate::Funcall(0, Arity::Some(2))), // [ret0, A, 1, +()]
                OpCode::Call(1),                                    // [ret0, A, cls0]
                OpCode::Get(2),                                     // [ret0, A, cls0, A]
                OpCode::Get(2),                                     // [ret0, A, cls0, A, cls0]
                OpCode::Call(1),                                    // [ret0, A, cls0, A+1]
                OpCode::Rot(2),                                     // [ret0, A, A+1, cls0]
                OpCode::Pop(1),                                     // [ret0, A, A+1]
                OpCode::Rot(2),                                     // [ret0, A+1, A]
                OpCode::Pop(1),                                     // [ret0, A+1]
                OpCode::Ret,                                        // [A+1]
                //
                // main().
                //
                OpCode::Psh(Immediate::Number(1)),
                OpCode::Psh(Immediate::Funcall(7, Arity::Some(1))),
                OpCode::Call(1),
                OpCode::Ret
            ]
        );
        Ok(())
    }
}
