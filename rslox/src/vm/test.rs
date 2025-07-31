use std::{
    io::Read,
    os::windows::prelude::{AsHandle, AsRawHandle},
    rc::Rc,
};

use log::Level;

use crate::{
    init_log,
    value::Value,
    vm::{InterpretError, VM},
};

#[test]
fn call_stack() -> Result<(), InterpretError> {
    let program: Rc<str> = Rc::from(
        "fun first() {
    var a = 1;
    second();
    var b = 2;
    second();
}

fun second() {
    var c = 3;
    var d = 4;
}

first();",
    );

    let mut vm = VM::default();

    vm.compile(program).unwrap();
    vm.step_n(11)?;

    assert_eq!(vm.stack.data[2], Value::Float(1.0));
    assert_eq!(vm.stack.data[4], Value::Float(3.0));
    assert_eq!(vm.stack.data[5], Value::Float(4.0));

    vm.step_n(8)?;
    assert_eq!(vm.stack.data[2], Value::Float(1.0));
    assert_eq!(vm.stack.data[3], Value::Float(2.0));
    assert_eq!(vm.stack.data[5], Value::Float(3.0));
    assert_eq!(vm.stack.data[6], Value::Float(4.0));

    vm.run()?;

    Ok(())
}

#[test]
fn ret_statement() -> Result<(), InterpretError> {
    let program: Rc<str> = Rc::from(
        "fun eef() {
    return \"freef\";
}


print eef();",
    );

    VM::default().interpret(program)?;

    Ok(())
}
