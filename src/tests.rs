use crate::{
    compiler::{Compiler, Context, LabelOrOpCode},
    grammar::ListsParser,
    opcodes::{Immediate, OpCode},
};

//
// Parser.
//

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

//
// Compiler.
//

#[test]
fn single_builtin() {
    let parser = ListsParser::new();
    let atoms = parser.parse("(+ 1 2)").unwrap().remove(0);
    let mut context = Context::default();
    let mut compiler = Compiler::default();
    compiler.compile_funcall(&mut context, atoms).unwrap();
    assert_eq!(
        context.stream(),
        &[
            OpCode::Psh(Immediate::Number(1)).into(),
            OpCode::Psh(Immediate::Number(2)).into(),
            OpCode::Add.into(),
        ]
    );
}

#[test]
fn nested_builtin() {
    let parser = ListsParser::new();
    let atoms = parser.parse("(+ (- 3 4) 2)").unwrap().remove(0);
    let mut context = Context::default();
    let mut compiler = Compiler::default();
    compiler.compile_funcall(&mut context, atoms).unwrap();
    assert_eq!(
        context.stream(),
        &[
            OpCode::Psh(Immediate::Number(3)).into(),
            OpCode::Psh(Immediate::Number(4)).into(),
            OpCode::Sub.into(),
            OpCode::Psh(Immediate::Number(2)).into(),
            OpCode::Add.into(),
        ]
    );
}

#[test]
fn if_then() {
    let parser = ListsParser::new();
    let atoms = parser.parse("(if (< 3 0) (+ 1 2))").unwrap().remove(0);
    let mut context = Context::default();
    let mut compiler = Compiler::default();
    compiler.compile_funcall(&mut context, atoms).unwrap();
    assert_eq!(
        context.stream(),
        &[
            OpCode::Psh(Immediate::Number(3)).into(),
            OpCode::Psh(Immediate::Number(0)).into(),
            OpCode::Lt.into(),
            LabelOrOpCode::BranchIfNot("BEGIN_ELSE_0000".to_owned().into_boxed_str()),
            OpCode::Psh(Immediate::Number(1)).into(),
            OpCode::Psh(Immediate::Number(2)).into(),
            OpCode::Add.into(),
            LabelOrOpCode::Branch("END_ELSE_0001".to_owned().into_boxed_str()),
            OpCode::Psh(Immediate::Nil).into(),
        ]
    );
}

#[test]
fn if_then_else() {
    let parser = ListsParser::new();
    let atoms = parser
        .parse("(if (< 3 0) (+ 1 2) (- 4 2))")
        .unwrap()
        .remove(0);
    let mut context = Context::default();
    let mut compiler = Compiler::default();
    compiler.compile_funcall(&mut context, atoms).unwrap();
    assert_eq!(
        context.stream(),
        &[
            OpCode::Psh(Immediate::Number(3)).into(),
            OpCode::Psh(Immediate::Number(0)).into(),
            OpCode::Lt.into(),
            LabelOrOpCode::BranchIfNot("BEGIN_ELSE_0000".to_owned().into_boxed_str()),
            OpCode::Psh(Immediate::Number(1)).into(),
            OpCode::Psh(Immediate::Number(2)).into(),
            OpCode::Add.into(),
            LabelOrOpCode::Branch("END_ELSE_0001".to_owned().into_boxed_str()),
            OpCode::Psh(Immediate::Number(4)).into(),
            OpCode::Psh(Immediate::Number(2)).into(),
            OpCode::Sub.into(),
        ]
    );
}

#[test]
fn nested_if_then() {
    let parser = ListsParser::new();
    let atoms = parser
        .parse("(if (< 3 0) (if (>= 1 4) (- 4 2)))")
        .unwrap()
        .remove(0);
    let mut context = Context::default();
    let mut compiler = Compiler::default();
    compiler.compile_funcall(&mut context, atoms).unwrap();
    assert_eq!(
        context.stream(),
        &[
            OpCode::Psh(Immediate::Number(3)).into(),
            OpCode::Psh(Immediate::Number(0)).into(),
            OpCode::Lt.into(),
            LabelOrOpCode::BranchIfNot("BEGIN_ELSE_0000".to_owned().into_boxed_str()),
            OpCode::Psh(Immediate::Number(1)).into(),
            OpCode::Psh(Immediate::Number(4)).into(),
            OpCode::Ge.into(),
            LabelOrOpCode::BranchIfNot("BEGIN_ELSE_0001".to_owned().into_boxed_str()),
            OpCode::Psh(Immediate::Number(4)).into(),
            OpCode::Psh(Immediate::Number(2)).into(),
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
    let atoms = parser.parse("(let ((a . 1)) (+ a 1))").unwrap().remove(0);
    let mut context = Context::default();
    let mut compiler = Compiler::default();
    compiler.compile_funcall(&mut context, atoms).unwrap();
    assert_eq!(
        context.stream(),
        &[
            OpCode::Psh(Immediate::Number(1)).into(),
            OpCode::Get(1).into(),
            OpCode::Psh(Immediate::Number(1)).into(),
            OpCode::Add.into(),
            OpCode::Rot(2).into(),
            OpCode::Pop(1).into(),
        ]
    )
}

#[test]
fn let_binding_with_a_single_funcall() {
    let parser = ListsParser::new();
    let atoms = parser
        .parse("(let ((a . (+ 1 2))) (+ a 1))")
        .unwrap()
        .remove(0);
    let mut context = Context::default();
    let mut compiler = Compiler::default();
    compiler.compile_funcall(&mut context, atoms).unwrap();
    assert_eq!(
        context.stream(),
        &[
            OpCode::Psh(Immediate::Number(1)).into(),
            OpCode::Psh(Immediate::Number(2)).into(),
            OpCode::Add.into(),
            OpCode::Get(1).into(),
            OpCode::Psh(Immediate::Number(1)).into(),
            OpCode::Add.into(),
            OpCode::Rot(2).into(),
            OpCode::Pop(1).into(),
        ]
    )
}

#[test]
fn let_binding_with_multiple_bindings() {
    let parser = ListsParser::new();
    let atoms = parser
        .parse("(let ((a . (+ 1 2)) (b . 2)) (+ a b))")
        .unwrap()
        .remove(0);
    let mut context = Context::default();
    let mut compiler = Compiler::default();
    compiler.compile_funcall(&mut context, atoms).unwrap();
    assert_eq!(
        context.stream(),
        &[
            OpCode::Psh(Immediate::Number(1)).into(),
            OpCode::Psh(Immediate::Number(2)).into(),
            OpCode::Add.into(),
            OpCode::Psh(Immediate::Number(2)).into(),
            OpCode::Get(2).into(),
            OpCode::Get(2).into(),
            OpCode::Add.into(),
            OpCode::Rot(3).into(),
            OpCode::Pop(2).into(),
        ]
    )
}

#[test]
fn nested_let_bindings() {
    let parser = ListsParser::new();
    let atoms = parser
        .parse("(let ((a . (+ 1 2))) (let ((b . (+ a 3))) (- a b)))")
        .unwrap()
        .remove(0);
    let mut context = Context::default();
    let mut compiler = Compiler::default();
    compiler.compile_funcall(&mut context, atoms).unwrap();
    assert_eq!(
        context.stream(),
        &[
            OpCode::Psh(Immediate::Number(1)).into(),
            OpCode::Psh(Immediate::Number(2)).into(),
            OpCode::Add.into(),
            OpCode::Get(1).into(),
            OpCode::Psh(Immediate::Number(3)).into(),
            OpCode::Add.into(),
            OpCode::Get(2).into(),
            OpCode::Get(2).into(),
            OpCode::Sub.into(),
            OpCode::Rot(2).into(),
            OpCode::Pop(1).into(),
            OpCode::Rot(2).into(),
            OpCode::Pop(1).into(),
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
            OpCode::Get(3),
            OpCode::Get(3),
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
            OpCode::Get(3),
            OpCode::Get(3),
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
            OpCode::Psh(Immediate::Number(1)),
            OpCode::Psh(Immediate::Number(2)),
            OpCode::Psh(Immediate::Number(3)),
            OpCode::Psh(Immediate::Funcall(0)),
            OpCode::Call,
            OpCode::Ret,
        ]
    );
}

#[test]
fn fibonacci() {
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
            OpCode::Get(1),                     // [ret0, N, N]
            OpCode::Psh(Immediate::Number(1)),  // [ret0, N, N, 1]
            OpCode::Le,                         // [ret0, N, T/nil]
            OpCode::Brn(3),                     // [ret0, N]
            OpCode::Get(1),                     // [ret0, N, N]
            OpCode::Br(12),                     //
            OpCode::Get(1),                     // [ret0, N, N]
            OpCode::Psh(Immediate::Number(1)),  // [ret0, N, N, 1]
            OpCode::Sub,                        // [ret0, N, N-1]
            OpCode::Psh(Immediate::Funcall(0)), // [ret0, N, N - 1, fun0]
            OpCode::Call,                       // [ret0, N, N-1, ret1]
            OpCode::Get(2),                     // [ret0, N, R0, N]
            OpCode::Psh(Immediate::Number(2)),  // [ret0, N, R0, N, 2]
            OpCode::Sub,                        // [ret0, N, R0, N-2]
            OpCode::Psh(Immediate::Funcall(0)), // [ret0, N, R0, N-2, fun0]
            OpCode::Call,                       // [ret0, N, N-2, ret1]
            OpCode::Add,                        // [ret0, N, R0+R1]
            OpCode::Rot(2),                     // [ret0, R0+R1, N]
            OpCode::Pop(1),                     // [ret0, R0+R1]
            OpCode::Ret
        ]
    );
}

#[test]
fn lambda() {
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
            OpCode::Get(2),                     // [ret0, a, pak0, a]
            OpCode::Psh(Immediate::Number(1)),  // [ret0, a, pak0, a, 1]
            OpCode::Psh(Immediate::Number(2)),  // [ret0, a, pak0, a, 1, 2]
            OpCode::Get(4),                     // [ret0, a, pak0, a, 1, 2, pak0]
            OpCode::Call,                       // [ret0, a, pak0, a, 1, 2, ret1]
            OpCode::Sub,                        // [ret0, a, pak0, a-3]
            OpCode::Rot(2),                     // [ret0, a, a-3, pak0]
            OpCode::Pop(1),                     // [ret0, a, a-3]
            OpCode::Rot(2),                     // [ret0, a-3, a]
            OpCode::Pop(1),                     // [ret0, a-3]
            OpCode::Ret
        ]
    )
}
