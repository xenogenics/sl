use crate::{
    compiler::Compiler,
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
    let mut compiler = Compiler::default();
    let result = compiler.compile_funcall(atoms, Vec::new()).unwrap();
    assert_eq!(
        result,
        vec![
            OpCode::Psh(Immediate::Number(1)),
            OpCode::Psh(Immediate::Number(2)),
            OpCode::Add,
        ]
    );
}

#[test]
fn nested_builtin() {
    let parser = ListsParser::new();
    let atoms = parser.parse("(+ (- 3 4) 2)").unwrap().remove(0);
    let mut compiler = Compiler::default();
    let result = compiler.compile_funcall(atoms, Vec::new()).unwrap();
    assert_eq!(
        result,
        vec![
            OpCode::Psh(Immediate::Number(3)),
            OpCode::Psh(Immediate::Number(4)),
            OpCode::Sub,
            OpCode::Psh(Immediate::Number(2)),
            OpCode::Add,
        ]
    );
}

#[test]
fn if_then() {
    let parser = ListsParser::new();
    let atoms = parser.parse("(if (< 3 0) (+ 1 2))").unwrap().remove(0);
    let mut compiler = Compiler::default();
    let result = compiler.compile_funcall(atoms, Vec::new()).unwrap();
    assert_eq!(
        result,
        vec![
            OpCode::Psh(Immediate::Number(3)),
            OpCode::Psh(Immediate::Number(0)),
            OpCode::Lt,
            OpCode::Brn(5),
            OpCode::Psh(Immediate::Number(1)),
            OpCode::Psh(Immediate::Number(2)),
            OpCode::Add,
            OpCode::Br(2),
            OpCode::Psh(Immediate::Nil),
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
    let mut compiler = Compiler::default();
    let result = compiler.compile_funcall(atoms, Vec::new()).unwrap();
    assert_eq!(
        result,
        vec![
            OpCode::Psh(Immediate::Number(3)),
            OpCode::Psh(Immediate::Number(0)),
            OpCode::Lt,
            OpCode::Brn(5),
            OpCode::Psh(Immediate::Number(1)),
            OpCode::Psh(Immediate::Number(2)),
            OpCode::Add,
            OpCode::Br(4),
            OpCode::Psh(Immediate::Number(4)),
            OpCode::Psh(Immediate::Number(2)),
            OpCode::Sub,
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
    let mut compiler = Compiler::default();
    let result = compiler.compile_funcall(atoms, Vec::new()).unwrap();
    assert_eq!(
        result,
        vec![
            OpCode::Psh(Immediate::Number(3)),
            OpCode::Psh(Immediate::Number(0)),
            OpCode::Lt,
            OpCode::Brn(11),
            OpCode::Psh(Immediate::Number(1)),
            OpCode::Psh(Immediate::Number(4)),
            OpCode::Ge,
            OpCode::Brn(5),
            OpCode::Psh(Immediate::Number(4)),
            OpCode::Psh(Immediate::Number(2)),
            OpCode::Sub,
            OpCode::Br(2),
            OpCode::Psh(Immediate::Nil),
            OpCode::Br(2),
            OpCode::Psh(Immediate::Nil),
        ]
    );
}

#[test]
fn let_binding_with_a_single_constant() {
    let parser = ListsParser::new();
    let atoms = parser.parse("(let ((a . 1)) (+ a 1))").unwrap().remove(0);
    let mut compiler = Compiler::default();
    let result = compiler.compile_funcall(atoms, Vec::new()).unwrap();
    assert_eq!(
        result,
        vec![
            OpCode::Psh(Immediate::Number(1)),
            OpCode::Pck(1),
            OpCode::Psh(Immediate::Number(1)),
            OpCode::Add,
            OpCode::Rot(2),
            OpCode::Pop(1),
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
    let mut compiler = Compiler::default();
    let result = compiler.compile_funcall(atoms, Vec::new()).unwrap();
    assert_eq!(
        result,
        vec![
            OpCode::Psh(Immediate::Number(1)),
            OpCode::Psh(Immediate::Number(2)),
            OpCode::Add,
            OpCode::Pck(1),
            OpCode::Psh(Immediate::Number(1)),
            OpCode::Add,
            OpCode::Rot(2),
            OpCode::Pop(1),
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
    let mut compiler = Compiler::default();
    let result = compiler.compile_funcall(atoms, Vec::new()).unwrap();
    assert_eq!(
        result,
        vec![
            OpCode::Psh(Immediate::Number(1)),
            OpCode::Psh(Immediate::Number(2)),
            OpCode::Add,
            OpCode::Psh(Immediate::Number(2)),
            OpCode::Pck(2),
            OpCode::Pck(2),
            OpCode::Add,
            OpCode::Rot(3),
            OpCode::Pop(2),
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
    let mut compiler = Compiler::default();
    let result = compiler.compile_funcall(atoms, Vec::new()).unwrap();
    assert_eq!(
        result,
        vec![
            OpCode::Psh(Immediate::Number(1)),
            OpCode::Psh(Immediate::Number(2)),
            OpCode::Add,
            OpCode::Pck(1),
            OpCode::Psh(Immediate::Number(3)),
            OpCode::Add,
            OpCode::Pck(2),
            OpCode::Pck(2),
            OpCode::Sub,
            OpCode::Rot(2),
            OpCode::Pop(1),
            OpCode::Rot(2),
            OpCode::Pop(1),
        ]
    )
}

#[test]
fn def_single_statement() {
    let parser = ListsParser::new();
    let atoms = parser.parse("(def ADD (A B) (+ A B))").unwrap();
    let mut compiler = Compiler::default();
    let (_, result) = compiler.compile(atoms).unwrap();
    assert_eq!(
        result,
        vec![
            OpCode::Rot(3),
            OpCode::Pck(2),
            OpCode::Pck(2),
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
    let mut compiler = Compiler::default();
    let (_, result) = compiler.compile(atoms).unwrap();
    assert_eq!(
        result,
        vec![
            OpCode::Rot(4),
            OpCode::Pck(3),
            OpCode::Pck(3),
            OpCode::Add,
            OpCode::Pop(1),
            OpCode::Pck(3),
            OpCode::Pck(2),
            OpCode::Sub,
            OpCode::Rot(4),
            OpCode::Pop(3),
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
    let mut compiler = Compiler::default();
    let (_, result) = compiler.compile(atoms).unwrap();
    assert_eq!(
        result,
        vec![
            OpCode::Rot(2),                    // [ret0, N]
            OpCode::Pck(1),                    // [ret0, N, N]
            OpCode::Psh(Immediate::Number(1)), // [ret0, N, N, 1]
            OpCode::Le,                        // [ret0, N, T/nil]
            OpCode::Brn(3),                    // [ret0, N]
            OpCode::Pck(1),                    // [ret0, N, N]
            OpCode::Br(10),                    //
            OpCode::Pck(1),                    // [ret0, N, N]
            OpCode::Psh(Immediate::Number(1)), // [ret0, N, N, 1]
            OpCode::Sub,                       // [ret0, N, N-1]
            OpCode::Brl(0),                    // [ret0, N, N-1, ret1]
            OpCode::Pck(2),                    // [ret0, N, R0, N]
            OpCode::Psh(Immediate::Number(2)), // [ret0, N, R0, N, 2]
            OpCode::Sub,                       // [ret0, N, R0, N-2]
            OpCode::Brl(0),                    // [ret0, N, N-2, ret1]
            OpCode::Add,                       // [ret0, N, R0+R1]
            OpCode::Rot(2),                    // [ret0, R0+R1, N]
            OpCode::Pop(1),                    // [ret0, R0+R1]
            OpCode::Ret
        ]
    );
}
