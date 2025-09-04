use std::{
    fs::File,
    io::{self, Read, Write},
    rc::Rc,
};

use tracing::{info, level_filters::LevelFilter};
use tracing_subscriber::fmt::format::FmtSpan;

use crate::vm::{InterpretError, VM};

pub mod chunk;
pub mod compiler;
pub mod scanner;
pub mod stack;
pub mod table;
pub mod value;
pub mod vm;

pub fn repl() -> Result<(), InterpretError> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    let mut vm = VM::default();

    loop {
        let start = std::time::Instant::now();
        write!(stdout, "> ").unwrap();
        stdout.flush().unwrap();

        let mut buffer = String::new();
        stdin.read_line(&mut buffer).unwrap();

        if buffer.trim_end() == "exit" {
            return Ok(());
        }

        let source: Rc<str> = Rc::from(buffer);

        match vm.interpret(source) {
            Ok(_) => (),
            Err(e) => println!("{e}"),
        }

        let dur = start.elapsed();

        info!("Execution time: {dur:?}");
    }
}

pub fn run_file(path: &str) -> Result<(), InterpretError> {
    let mut f = File::open(path).unwrap();
    let mut buffer = String::new();
    f.read_to_string(&mut buffer).unwrap();

    let mut vm = VM::default();

    let source: Rc<str> = Rc::from(buffer);

    let start = std::time::Instant::now();

    vm.compile(source)?;

    let dur = start.elapsed();
    info!(target: "Compilation time", "{dur:?}");

    let start = std::time::Instant::now();

    let res = vm.run();
    if res.is_err() {
        vm.print_stack_trace();
    }

    let dur = start.elapsed();
    info!(target: "Execution time", "{dur:?}");

    res
}

pub fn init_tracing(log_level: impl Into<LevelFilter>) {
    tracing_subscriber::fmt()
        .without_time()
        .with_ansi(true)
        .with_file(false)
        .with_line_number(false)
        .with_max_level(log_level)
        .with_span_events(FmtSpan::ENTER)
        .with_thread_names(false)
        .with_thread_ids(false)
        .init();
}

#[cfg(test)]
mod tests {
    use crate::{
        chunk::OpCode,
        scanner::{Scanner, Token, TokenKind},
        vm::VMState,
        *,
    };

    fn read_file(path: &'static str) -> Rc<str> {
        let mut f = File::open(path).unwrap();
        let mut buffer = String::new();
        f.read_to_string(&mut buffer).unwrap();
        Rc::from(buffer)
    }

    fn expect_printed(path: &'static str, cases: &[&'static str]) -> Result<(), InterpretError> {
        let file = read_file(path);
        let mut vm = VM::default();
        vm.compile(file)?;

        let mut c = cases.iter().cloned().enumerate();

        loop {
            match vm.step() {
                Ok(VMState::Running) => {
                    let ip = *vm.ip();
                    if let Some(OpCode::Print) = OpCode::from_repr(vm.chunk().data[ip]) {
                        let (idx, case) = c.next().unwrap();
                        assert!(
                            vm.stack.top().to_string() == case,
                            "[case {idx}] Expected: {:?}, Got: {:?}",
                            case,
                            vm.stack.top().to_string()
                        );
                    }
                }
                Ok(VMState::Done) => {
                    assert!(c.next().is_none());
                    assert!(vm.stack.cursor == 0);
                    return Ok(());
                }
                Err(e) => return Err(e),
            }
        }
    }

    fn expect_compile_error(path: &'static str) -> Result<(), InterpretError> {
        let file = read_file(path);
        let mut vm = VM::default();
        assert!(vm.compile(file).is_err());

        Ok(())
    }

    fn expect_runtime_error(path: &'static str, error: &'static str) -> Result<(), InterpretError> {
        let file = read_file(path);
        let mut vm = VM::default();
        vm.compile(file)?;

        match vm.run() {
            Err(InterpretError::RuntimeError(s)) if s == error => Ok(()),
            Ok(()) => panic!("expected error"),
            Err(x) => Err(x),
        }
    }

    fn expect_scanner(path: &'static str, cases: &[Token]) {
        let file = read_file(path);
        let mut scanner = Scanner::new(file);

        let mut c = cases.iter();

        loop {
            let token = scanner.next_token();
            let case = c.next().unwrap();

            assert_eq!(token, *case);

            if token.kind == TokenKind::EOF {
                break;
            }
        }

        assert!(
            c.size_hint().0 == 0,
            "Not all cases found: {:?}",
            c.collect::<Vec<_>>()
        );
    }

    mod assign {
        use super::*;
        #[test]
        fn associativity() -> Result<(), InterpretError> {
            expect_printed(r"..\test\assignment\associativity.lox", &["c", "c", "c"])
        }

        #[test]
        fn global() -> Result<(), InterpretError> {
            expect_printed(
                r"..\test\assignment\global.lox",
                &["before", "after", "arg", "arg"],
            )
        }

        #[test]
        fn grouping() -> Result<(), InterpretError> {
            expect_compile_error(r"..\test\assignment\grouping.lox")
        }

        #[test]
        fn infix_operator() -> Result<(), InterpretError> {
            expect_compile_error(r"..\test\assignment\infix_operator.lox")
        }

        #[test]
        fn local() -> Result<(), InterpretError> {
            expect_printed(
                r"..\test\assignment\local.lox",
                &["before", "after", "arg", "arg"],
            )
        }

        #[test]
        fn prefix_operator() -> Result<(), InterpretError> {
            expect_compile_error(r"..\test\assignment\prefix_operator.lox")
        }

        #[test]
        fn syntax() -> Result<(), InterpretError> {
            expect_printed(r"..\test\assignment\syntax.lox", &["var", "var"])
        }

        #[test]
        fn to_this() -> Result<(), InterpretError> {
            expect_compile_error(r"..\test\assignment\to_this.lox")
        }

        #[test]
        fn undefined() -> Result<(), InterpretError> {
            expect_runtime_error(
                r"..\test\assignment\undefined.lox",
                "[cycle: 2] Undefined variable 'unknown'.",
            )
        }
    }

    mod block {
        use super::*;

        #[test]
        fn empty() -> Result<(), InterpretError> {
            expect_printed(r"..\test\block\empty.lox", &["ok"])
        }

        #[test]
        fn scope() -> Result<(), InterpretError> {
            expect_printed(r"..\test\block\scope.lox", &["inner", "outer"])
        }
    }

    mod bool {
        use super::*;

        #[test]
        fn equality() -> Result<(), InterpretError> {
            expect_printed(
                r"..\test\bool\equality.lox",
                &[
                    "true", "false", "false", "true", "false", "false", "false", "false", "false",
                    "false", "true", "true", "false", "true", "true", "true", "true", "true",
                ],
            )
        }

        #[test]
        fn not() -> Result<(), InterpretError> {
            expect_printed(r"..\test\bool\not.lox", &["false", "true", "true"])
        }
    }

    mod call {
        use super::*;

        #[test]
        fn bool() -> Result<(), InterpretError> {
            expect_runtime_error(
                r"..\test\call\bool.lox",
                "[cycle: 2] Object 'Bool(true)' is not callable",
            )
        }

        #[test]
        fn nil() -> Result<(), InterpretError> {
            expect_runtime_error(
                r"..\test\call\nil.lox",
                "[cycle: 2] Object 'Nil' is not callable",
            )
        }

        #[test]
        fn num() -> Result<(), InterpretError> {
            expect_runtime_error(
                r"..\test\call\num.lox",
                "[cycle: 2] Object 'Float(123.0)' is not callable",
            )
        }

        #[test]
        fn object() -> Result<(), InterpretError> {
            expect_runtime_error(
                r"..\test\call\bool.lox",
                "[cycle: 2] Object 'Bool(true)' is not callable",
            )
        }

        #[test]
        fn string() -> Result<(), InterpretError> {
            expect_runtime_error(
                r"..\test\call\string.lox",
                "[cycle: 2] Object 'String(\"str\")' is not callable",
            )
        }
    }

    mod class {
        use super::*;

        #[test]
        fn empty() -> Result<(), InterpretError> {
            expect_printed(r"..\test\class\empty.lox", &["Class(\"Foo\")"])
        }

        #[test]
        fn inherit_self() -> Result<(), InterpretError> {
            expect_compile_error(r"..\test\class\inherit_self.lox")
        }

        #[test]
        fn inherited_method() -> Result<(), InterpretError> {
            expect_printed(
                r"..\test\class\inherited_method.lox",
                &["in foo", "in bar", "in baz"],
            )
        }

        #[test]
        fn local_inherit_other() -> Result<(), InterpretError> {
            expect_printed(r"..\test\class\local_inherit_other.lox", &["Class(\"B\")"])
        }

        #[test]
        fn local_inherit_self() -> Result<(), InterpretError> {
            expect_compile_error(r"..\test\class\local_inherit_self.lox")
        }

        #[test]
        fn local_reference_self() -> Result<(), InterpretError> {
            expect_printed(
                r"..\test\class\local_reference_self.lox",
                &["Class(\"Foo\")"],
            )
        }

        #[test]
        fn reference_self() -> Result<(), InterpretError> {
            expect_printed(r"..\test\class\reference_self.lox", &["Class(\"Foo\")"])
        }
    }

    mod closure {
        use super::*;

        #[test]
        fn assign_to_closure() -> Result<(), InterpretError> {
            expect_printed(
                r"..\test\closure\assign_to_closure.lox",
                &["local", "after f", "after f", "after g"],
            )
        }

        #[test]
        fn assign_to_shadowed_later() -> Result<(), InterpretError> {
            expect_printed(
                r"..\test\closure\assign_to_shadowed_later.lox",
                &["inner", "assigned"],
            )
        }

        #[test]
        fn close_over_function_parameter() -> Result<(), InterpretError> {
            expect_printed(
                r"..\test\closure\close_over_function_parameter.lox",
                &["param"],
            )
        }

        #[test]
        fn close_over_later_variable() -> Result<(), InterpretError> {
            expect_printed(
                r"..\test\closure\close_over_later_variable.lox",
                &["b", "a"],
            )
        }

        #[test]
        fn close_over_method_parameter() -> Result<(), InterpretError> {
            expect_printed(
                r"..\test\closure\close_over_method_parameter.lox",
                &["param"],
            )
        }

        #[test]
        fn closed_closure_in_function() -> Result<(), InterpretError> {
            expect_printed(
                r"..\test\closure\closed_closure_in_function.lox",
                &["local"],
            )
        }

        #[test]
        fn nested_closure() -> Result<(), InterpretError> {
            expect_printed(r"..\test\closure\nested_closure.lox", &["a", "b", "c"])
        }

        #[test]
        fn open_closure_in_function() -> Result<(), InterpretError> {
            expect_printed(r"..\test\closure\open_closure_in_function.lox", &["local"])
        }

        #[test]
        fn reference_closure_multiple_times() -> Result<(), InterpretError> {
            expect_printed(
                r"..\test\closure\reference_closure_multiple_times.lox",
                &["a", "a"],
            )
        }

        #[test]
        fn reuse_closure_slot() -> Result<(), InterpretError> {
            expect_printed(r"..\test\closure\reuse_closure_slot.lox", &["a"])
        }

        #[test]
        fn shadow_closure_with_local() -> Result<(), InterpretError> {
            expect_printed(
                r"..\test\closure\shadow_closure_with_local.lox",
                &["closure", "shadow", "closure"],
            )
        }

        #[test]
        fn unused_closure() -> Result<(), InterpretError> {
            expect_printed(r"..\test\closure\unused_closure.lox", &["ok"])
        }

        #[test]
        fn unused_later_closure() -> Result<(), InterpretError> {
            expect_printed(r"..\test\closure\unused_later_closure.lox", &["a"])
        }
    }

    mod comments {
        use super::*;

        #[test]
        fn line_at_eof() -> Result<(), InterpretError> {
            expect_printed(r"..\test\comments\line_at_eof.lox", &["ok"])
        }

        #[test]
        fn only_line_comment_and_line() -> Result<(), InterpretError> {
            run_file(r"..\test\comments\only_line_comment_and_line.lox")
        }

        #[test]
        fn only_line_comment() -> Result<(), InterpretError> {
            run_file(r"..\test\comments\only_line_comment.lox")
        }

        #[test]
        fn unicode() -> Result<(), InterpretError> {
            expect_printed(r"..\test\comments\unicode.lox", &["ok"])
        }
    }

    mod constructor {
        use super::*;

        #[test]
        fn arguments() -> Result<(), InterpretError> {
            expect_printed(r"..\test\constructor\arguments.lox", &["init", "1", "2"])
        }

        #[test]
        fn call_init_early_return() -> Result<(), InterpretError> {
            expect_printed(
                r"..\test\constructor\call_init_early_return.lox",
                &["init", "init", "Foo{}"],
            )
        }

        #[test]
        fn call_init_explicitly() -> Result<(), InterpretError> {
            expect_printed(
                r"..\test\constructor\call_init_explicitly.lox",
                &["Foo.init(one)", "Foo.init(two)", "Foo{field: init}", "init"],
            )
        }

        #[test]
        fn default_arguments() -> Result<(), InterpretError> {
            expect_runtime_error(
                r"..\test\constructor\default_arguments.lox",
                "[cycle 9] Expected 0 arguments but got 3.",
            )
        }

        #[test]
        fn default() -> Result<(), InterpretError> {
            expect_printed(r"..\test\constructor\default.lox", &["Foo{}"])
        }

        #[test]
        fn early_return() -> Result<(), InterpretError> {
            expect_printed(r"..\test\constructor\early_return.lox", &["init", "Foo{}"])
        }

        #[test]
        fn extra_arguments() -> Result<(), InterpretError> {
            expect_runtime_error(
                r"..\test\constructor\extra_arguments.lox",
                "[cycle: 12] Function(init) expects 2 args, got 4.",
            )
        }

        #[test]
        fn init_not_method() -> Result<(), InterpretError> {
            expect_printed(
                r"..\test\constructor\init_not_method.lox",
                &["not initializer"],
            )
        }

        #[test]
        fn missing_arguments() -> Result<(), InterpretError> {
            expect_runtime_error(
                r"..\test\constructor\missing_arguments.lox",
                "[cycle: 9] Function(init) expects 2 args, got 1.",
            )
        }

        #[test]
        fn return_in_nested_function() -> Result<(), InterpretError> {
            expect_printed(
                r"..\test\constructor\return_in_nested_function.lox",
                &["bar", "Foo{}"],
            )
        }

        #[test]
        fn return_value() -> Result<(), InterpretError> {
            expect_compile_error(r"..\test\constructor\return_value.lox")
        }
    }

    mod field {
        use super::*;

        #[test]
        fn call_function_field() -> Result<(), InterpretError> {
            expect_printed(r"..\test\field\call_function_field.lox", &["bar", "1", "2"])
        }

        #[test]
        fn call_nonfunction_field() -> Result<(), InterpretError> {
            expect_runtime_error(
                r"..\test\field\call_nonfunction_field.lox",
                "[cycle 13] Cannot call non-function field 'bar': not fn.",
            )
        }

        #[test]
        fn get_and_set_method() -> Result<(), InterpretError> {
            expect_printed(
                r"..\test\field\get_and_set_method.lox",
                &["other", "1", "method", "2"],
            )
        }

        #[test]
        fn get_on_bool() -> Result<(), InterpretError> {
            expect_runtime_error(
                r"..\test\field\get_on_bool.lox",
                "[cycle: 2] Cannot read property of non-instance: Bool(true)",
            )
        }

        #[test]
        fn get_on_class() -> Result<(), InterpretError> {
            expect_runtime_error(
                r"..\test\field\get_on_class.lox",
                "[cycle: 6] Cannot read property of non-instance: Class(\"Foo\")",
            )
        }

        #[test]
        fn get_on_function() -> Result<(), InterpretError> {
            expect_runtime_error(
                r"..\test\field\get_on_function.lox",
                "[cycle: 4] Cannot read property of non-instance: Closure(\"foo\")",
            )
        }

        #[test]
        fn get_on_nil() -> Result<(), InterpretError> {
            expect_runtime_error(
                r"..\test\field\get_on_nil.lox",
                "[cycle: 2] Cannot read property of non-instance: Nil",
            )
        }

        #[test]
        fn get_on_num() -> Result<(), InterpretError> {
            expect_runtime_error(
                r"..\test\field\get_on_num.lox",
                "[cycle: 2] Cannot read property of non-instance: Float(123.0)",
            )
        }

        #[test]
        fn get_on_string() -> Result<(), InterpretError> {
            expect_runtime_error(
                r"..\test\field\get_on_string.lox",
                "[cycle: 2] Cannot read property of non-instance: String(\"str\")",
            )
        }

        #[test]
        fn many() -> Result<(), InterpretError> {
            expect_printed(
                r"..\test\field\many.lox",
                &[
                    "apple",
                    "apricot",
                    "avocado",
                    "banana",
                    "bilberry",
                    "blackberry",
                    "blackcurrant",
                    "blueberry",
                    "boysenberry",
                    "cantaloupe",
                    "cherimoya",
                    "cherry",
                    "clementine",
                    "cloudberry",
                    "coconut",
                    "cranberry",
                    "currant",
                    "damson",
                    "date",
                    "dragonfruit",
                    "durian",
                    "elderberry",
                    "feijoa",
                    "fig",
                    "gooseberry",
                    "grape",
                    "grapefruit",
                    "guava",
                    "honeydew",
                    "huckleberry",
                    "jabuticaba",
                    "jackfruit",
                    "jambul",
                    "jujube",
                    "juniper",
                    "kiwifruit",
                    "kumquat",
                    "lemon",
                    "lime",
                    "longan",
                    "loquat",
                    "lychee",
                    "mandarine",
                    "mango",
                    "marionberry",
                    "melon",
                    "miracle",
                    "mulberry",
                    "nance",
                    "nectarine",
                    "olive",
                    "orange",
                    "papaya",
                    "passionfruit",
                    "peach",
                    "pear",
                    "persimmon",
                    "physalis",
                    "pineapple",
                    "plantain",
                    "plum",
                    "plumcot",
                    "pomegranate",
                    "pomelo",
                    "quince",
                    "raisin",
                    "rambutan",
                    "raspberry",
                    "redcurrant",
                    "salak",
                    "salmonberry",
                    "satsuma",
                    "strawberry",
                    "tamarillo",
                    "tamarind",
                    "tangerine",
                    "tomato",
                    "watermelon",
                    "yuzu",
                ],
            )
        }

        #[test]
        fn method_binds_this() -> Result<(), InterpretError> {
            expect_printed(r"..\test\field\method_binds_this.lox", &["foo1", "1"])
        }

        #[test]
        fn method() -> Result<(), InterpretError> {
            expect_printed(r"..\test\field\method.lox", &["got method", "arg"])
        }

        #[test]
        fn on_instance() -> Result<(), InterpretError> {
            expect_printed(
                r"..\test\field\on_instance.lox",
                &["bar value", "baz value", "bar value", "baz value"],
            )
        }

        #[test]
        fn set_evaluation_order() -> Result<(), InterpretError> {
            expect_runtime_error(
                r"..\test\field\set_evaluation_order.lox",
                "[cycle: 1] Undefined variable 'undefined1'.",
            )
        }

        #[test]
        fn set_on_bool() -> Result<(), InterpretError> {
            expect_runtime_error(
                r"..\test\field\set_on_bool.lox",
                "[cycle: 3] Cannot write property of non-instance: Bool(true)",
            )
        }

        #[test]
        fn set_on_class() -> Result<(), InterpretError> {
            expect_runtime_error(
                r"..\test\field\set_on_class.lox",
                "[cycle: 7] Cannot write property of non-instance: Class(\"Foo\")",
            )
        }

        #[test]
        fn set_on_function() -> Result<(), InterpretError> {
            expect_runtime_error(
                r"..\test\field\set_on_function.lox",
                "[cycle: 5] Cannot write property of non-instance: Closure(\"foo\")",
            )
        }

        #[test]
        fn set_on_nil() -> Result<(), InterpretError> {
            expect_runtime_error(
                r"..\test\field\set_on_nil.lox",
                "[cycle: 3] Cannot write property of non-instance: Nil",
            )
        }

        #[test]
        fn set_on_num() -> Result<(), InterpretError> {
            expect_runtime_error(
                r"..\test\field\set_on_num.lox",
                "[cycle: 3] Cannot write property of non-instance: Float(123.0)",
            )
        }

        #[test]
        fn set_on_string() -> Result<(), InterpretError> {
            expect_runtime_error(
                r"..\test\field\set_on_string.lox",
                "[cycle: 3] Cannot write property of non-instance: String(\"str\")",
            )
        }

        #[test]
        fn undefined() -> Result<(), InterpretError> {
            expect_runtime_error(
                r"..\test\field\undefined.lox",
                "[cycle 9] Undefined property bar for class Foo",
            )
        }
    }

    mod for_loop {
        use super::*;

        #[test]
        fn class_in_body() -> Result<(), InterpretError> {
            expect_compile_error(r"..\test\for\class_in_body.lox")
        }

        #[test]
        fn closure_in_body() -> Result<(), InterpretError> {
            expect_printed(
                r"..\test\for\closure_in_body.lox",
                &["4", "1", "4", "2", "4", "3"],
            )
        }

        #[test]
        fn fun_in_body() -> Result<(), InterpretError> {
            expect_compile_error(r"..\test\for\fun_in_body.lox")
        }

        #[test]
        fn return_closure() -> Result<(), InterpretError> {
            expect_printed(r"..\test\for\return_closure.lox", &["i"])
        }

        #[test]
        fn return_inside() -> Result<(), InterpretError> {
            expect_printed(r"..\test\for\return_inside.lox", &["i"])
        }

        #[test]
        fn scope() -> Result<(), InterpretError> {
            expect_printed(r"..\test\for\scope.lox", &["0", "-1", "after", "0"])
        }

        #[test]
        fn statement_condition() -> Result<(), InterpretError> {
            expect_compile_error(r"..\test\for\statement_condition.lox")
        }

        #[test]
        fn statement_increment() -> Result<(), InterpretError> {
            expect_compile_error(r"..\test\for\statement_increment.lox")
        }

        #[test]
        fn statement_initializer() -> Result<(), InterpretError> {
            expect_compile_error(r"..\test\for\statement_initializer.lox")
        }

        #[test]
        fn syntax() -> Result<(), InterpretError> {
            expect_printed(
                r"..\test\for\syntax.lox",
                &[
                    "1", "2", "3", "0", "1", "2", "done", "0", "1", "0", "1", "2", "0", "1",
                ],
            )
        }

        #[test]
        fn var_in_body() -> Result<(), InterpretError> {
            expect_compile_error(r"..\test\for\var_in_body.lox")
        }
    }

    mod function {
        use super::*;

        #[test]
        fn body_must_be_block() -> Result<(), InterpretError> {
            expect_compile_error(r"..\test\function\body_must_be_block.lox")
        }

        #[test]
        fn empty_body() -> Result<(), InterpretError> {
            expect_printed(r"..\test\function\empty_body.lox", &["nil"])
        }

        #[test]
        fn extra_arguments() -> Result<(), InterpretError> {
            expect_runtime_error(
                r"..\test\function\extra_arguments.lox",
                "[cycle: 8] Function(f) expects 2 args, got 4.",
            )
        }

        #[test]
        fn local_mutual_recursion() -> Result<(), InterpretError> {
            expect_runtime_error(
                r"..\test\function\local_mutual_recursion.lox",
                "[cycle: 11] Undefined variable 'isOdd'.",
            )
        }

        #[test]
        fn local_recursion() -> Result<(), InterpretError> {
            expect_printed(r"..\test\function\local_recursion.lox", &["21"])
        }

        #[test]
        fn missing_arguments() -> Result<(), InterpretError> {
            expect_runtime_error(
                r"..\test\function\missing_arguments.lox",
                "[cycle: 5] Function(f) expects 2 args, got 1.",
            )
        }

        #[test]
        fn missing_comma_in_parameters() -> Result<(), InterpretError> {
            expect_compile_error(r"..\test\function\missing_comma_in_parameters.lox")
        }

        #[test]
        fn mutual_recursion() -> Result<(), InterpretError> {
            expect_printed(r"..\test\function\mutual_recursion.lox", &["true", "true"])
        }

        #[test]
        fn nested_call_with_arguments() -> Result<(), InterpretError> {
            expect_printed(
                r"..\test\function\nested_call_with_arguments.lox",
                &["hello world"],
            )
        }

        #[test]
        fn parameters() -> Result<(), InterpretError> {
            expect_printed(
                r"..\test\function\parameters.lox",
                &["0", "1", "3", "6", "10", "15", "21", "28", "36"],
            )
        }

        #[test]
        fn print() -> Result<(), InterpretError> {
            expect_printed(
                r"..\test\function\print.lox",
                &["Closure(<fn foo>)", "<native fn>"],
            )
        }

        #[test]
        fn recursion() -> Result<(), InterpretError> {
            expect_printed(r"..\test\function\recursion.lox", &["21"])
        }

        #[test]
        fn too_many_arguments() {
            let _ = expect_compile_error(r"..\test\function\too_many_arguments.lox");
        }

        #[test]
        fn too_many_parameters() {
            let _ = expect_compile_error(r"..\test\function\too_many_parameters.lox");
        }
    }

    mod if_stmt {
        use super::*;

        #[test]
        fn class_in_else() -> Result<(), InterpretError> {
            expect_compile_error(r"..\test\if\class_in_else.lox")
        }

        #[test]
        fn class_in_then() -> Result<(), InterpretError> {
            expect_compile_error(r"..\test\if\class_in_then.lox")
        }

        #[test]
        fn dangling_else() -> Result<(), InterpretError> {
            expect_printed(r"..\test\if\dangling_else.lox", &["good"])
        }

        #[test]
        fn else_() -> Result<(), InterpretError> {
            expect_printed(r"..\test\if\else.lox", &["good", "good", "block"])
        }

        #[test]
        fn fun_in_else() -> Result<(), InterpretError> {
            expect_compile_error(r"..\test\if\fun_in_else.lox")
        }

        #[test]
        fn fun_in_then() -> Result<(), InterpretError> {
            expect_compile_error(r"..\test\if\fun_in_then.lox")
        }

        #[test]
        fn if_() -> Result<(), InterpretError> {
            expect_printed(r"..\test\if\if.lox", &["good", "block", "true"])
        }

        #[test]
        fn truth() -> Result<(), InterpretError> {
            expect_printed(
                r"..\test\if\truth.lox",
                &["false", "nil", "true", "0", "empty"],
            )
        }

        #[test]
        fn var_in_else() -> Result<(), InterpretError> {
            expect_compile_error(r"..\test\if\var_in_else.lox")
        }

        #[test]
        fn var_in_then() -> Result<(), InterpretError> {
            expect_compile_error(r"..\test\if\var_in_then.lox")
        }
    }

    mod inheritance {
        use super::*;

        #[test]
        fn constructor() -> Result<(), InterpretError> {
            expect_printed(r"..\test\inheritance\constructor.lox", &["value"])
        }

        #[test]
        fn inherit_from_function() -> Result<(), InterpretError> {
            expect_runtime_error(
                r"..\test\inheritance\inherit_from_function.lox",
                "[cycle 7] Superclass must be a class. Got Closure(<fn foo>)",
            )
        }

        #[test]
        fn inherit_from_nil() -> Result<(), InterpretError> {
            expect_runtime_error(
                r"..\test\inheritance\inherit_from_nil.lox",
                "[cycle 7] Superclass must be a class. Got nil",
            )
        }

        #[test]
        fn inherit_from_number() -> Result<(), InterpretError> {
            expect_runtime_error(
                r"..\test\inheritance\inherit_from_number.lox",
                "[cycle 7] Superclass must be a class. Got 123",
            )
        }

        #[test]
        fn inherit_methods() -> Result<(), InterpretError> {
            expect_printed(
                r"..\test\inheritance\inherit_methods.lox",
                &["foo", "bar", "bar"],
            )
        }

        #[test]
        fn parenthesized_superclass() -> Result<(), InterpretError> {
            expect_compile_error(r"..\test\inheritance\parenthesized_superclass.lox")
        }

        #[test]
        fn set_fields_from_base_class() -> Result<(), InterpretError> {
            expect_printed(
                r"..\test\inheritance\set_fields_from_base_class.lox",
                &["foo 1", "foo 2", "bar 1", "bar 2", "bar 1", "bar 2"],
            )
        }
    }

    mod limit {
        use super::*;

        #[test]
        #[should_panic]
        fn loop_too_large() {
            let _ = expect_compile_error(r"..\test\limit\loop_too_large.lox");
        }

        #[test]
        fn no_reuse_constants() -> Result<(), InterpretError> {
            run_file(r"..\test\limit\no_reuse_constants.lox")
        }

        #[test]
        fn stack_overflow() -> Result<(), InterpretError> {
            expect_runtime_error(
                r"..\test\limit\stack_overflow.lox",
                "[cycle: 1138] Stack overflow",
            )
        }

        #[test]
        #[should_panic]
        fn too_many_constants() {
            let _ = expect_compile_error(r"..\test\limit\too_many_constants.lox");
        }

        #[test]
        fn too_many_locals() -> Result<(), InterpretError> {
            expect_compile_error(r"..\test\limit\too_many_locals.lox")
        }

        #[test]
        #[should_panic]
        fn too_many_upvalues() {
            let _ = expect_compile_error(r"..\test\limit\too_many_upvalues.lox");
        }
    }

    mod logical_operator {
        use super::*;

        #[test]
        fn and_truth() -> Result<(), InterpretError> {
            expect_printed(
                r"..\test\logical_operator\and_truth.lox",
                &["false", "nil", "ok", "ok", "ok"],
            )
        }

        #[test]
        fn and() -> Result<(), InterpretError> {
            expect_printed(
                r"..\test\logical_operator\and.lox",
                &["false", "1", "false", "true", "3", "true", "false"],
            )
        }

        #[test]
        fn or_truth() -> Result<(), InterpretError> {
            expect_printed(
                r"..\test\logical_operator\or_truth.lox",
                &["ok", "ok", "true", "0", "s"],
            )
        }

        #[test]
        fn or() -> Result<(), InterpretError> {
            expect_printed(
                r"..\test\logical_operator\or.lox",
                &["1", "1", "true", "false", "false", "false", "true"],
            )
        }
    }

    mod method {
        use super::*;

        #[test]
        fn arity() -> Result<(), InterpretError> {
            expect_printed(
                r"..\test\method\arity.lox",
                &["no args", "1", "3", "6", "10", "15", "21", "28", "36"],
            )
        }

        #[test]
        fn empty_block() -> Result<(), InterpretError> {
            expect_printed(r"..\test\method\empty_block.lox", &["nil"])
        }

        #[test]
        fn extra_arguments() -> Result<(), InterpretError> {
            expect_runtime_error(
                r"..\test\method\extra_arguments.lox",
                "[cycle: 13] Function(method) expects 2 args, got 4.",
            )
        }

        #[test]
        fn missing_arguments() -> Result<(), InterpretError> {
            expect_runtime_error(
                r"..\test\method\missing_arguments.lox",
                "[cycle: 10] Function(method) expects 2 args, got 1.",
            )
        }

        #[test]
        fn not_found() -> Result<(), InterpretError> {
            expect_runtime_error(
                r"..\test\method\not_found.lox",
                "[cycle 7] Undefined method unknown for class Foo",
            )
        }

        #[test]
        fn print_bound_method() -> Result<(), InterpretError> {
            expect_printed(
                r"..\test\method\print_bound_method.lox",
                &["BoundMethod(class:Foo, method:Closure(<fn method>))"],
            )
        }

        #[test]
        fn refer_to_name() -> Result<(), InterpretError> {
            expect_runtime_error(
                r"..\test\method\refer_to_name.lox",
                "[cycle: 10] Undefined variable 'method'.",
            )
        }

        #[test]
        fn too_many_arguments() -> Result<(), InterpretError> {
            expect_compile_error(r"..\test\method\too_many_arguments.lox")
        }

        #[test]
        fn too_many_parameters() -> Result<(), InterpretError> {
            expect_compile_error(r"..\test\method\too_many_parameters.lox")
        }
    }

    mod nil {
        use super::*;

        #[test]
        fn literal() -> Result<(), InterpretError> {
            expect_printed(r"..\test\nil\literal.lox", &["nil"])
        }
    }

    mod number {
        use super::*;

        #[test]
        fn decimal_point_at_eof() -> Result<(), InterpretError> {
            expect_compile_error(r"..\test\number\decimal_point_at_eof.lox")
        }

        #[test]
        fn leading_dot() -> Result<(), InterpretError> {
            expect_compile_error(r"..\test\number\leading_dot.lox")
        }

        #[test]
        fn literals() -> Result<(), InterpretError> {
            expect_printed(
                r"..\test\number\literals.lox",
                &["123", "987654", "0", "-0", "123.456", "-0.001"],
            )
        }

        #[test]
        fn nan_equality() -> Result<(), InterpretError> {
            expect_printed(
                r"..\test\number\nan_equality.lox",
                &["false", "true", "false", "true"],
            )
        }

        #[test]
        fn trailing_dot() -> Result<(), InterpretError> {
            expect_compile_error(r"..\test\number\trailing_dot.lox")
        }
    }

    mod operator {
        use super::*;

        #[test]
        fn add_bool_nil() -> Result<(), InterpretError> {
            expect_runtime_error(
                r"..\test\operator\add_bool_nil.lox",
                "Add called with non-number/non-string operands: (Bool(true), Nil)",
            )
        }

        #[test]
        fn add_bool_num() -> Result<(), InterpretError> {
            expect_runtime_error(
                r"..\test\operator\add_bool_num.lox",
                "Add called with non-number/non-string operands: (Bool(true), Float(123.0))",
            )
        }

        #[test]
        fn add_bool_string() -> Result<(), InterpretError> {
            expect_runtime_error(
                r"..\test\operator\add_bool_string.lox",
                "Add called with non-number/non-string operands: (Bool(true), String(\"s\"))",
            )
        }

        #[test]
        fn add_nil_nil() -> Result<(), InterpretError> {
            expect_runtime_error(
                r"..\test\operator\add_nil_nil.lox",
                "Add called with non-number/non-string operands: (Nil, Nil)",
            )
        }

        #[test]
        fn add_num_nil() -> Result<(), InterpretError> {
            expect_runtime_error(
                r"..\test\operator\add_num_nil.lox",
                "Add called with non-number/non-string operands: (Float(1.0), Nil)",
            )
        }

        #[test]
        fn add_string_nil() -> Result<(), InterpretError> {
            expect_runtime_error(
                r"..\test\operator\add_string_nil.lox",
                "Add called with non-number/non-string operands: (String(\"s\"), Nil)",
            )
        }

        #[test]
        fn add() -> Result<(), InterpretError> {
            expect_printed(r"..\test\operator\add.lox", &["579", "string"])
        }

        #[test]
        fn comparison() -> Result<(), InterpretError> {
            expect_printed(
                r"..\test\operator\comparison.lox",
                &[
                    "true", "false", "false", "true", "true", "false", "false", "false", "true",
                    "false", "true", "true", "false", "false", "false", "false", "true", "true",
                    "true", "true",
                ],
            )
        }

        #[test]
        fn divide_nonnum_num() -> Result<(), InterpretError> {
            expect_runtime_error(
                r"..\test\operator\divide_nonnum_num.lox",
                "Div called with non-number operand(s): (String(\"1\"), Float(1.0))",
            )
        }

        #[test]
        fn divide_num_nonnum() -> Result<(), InterpretError> {
            expect_runtime_error(
                r"..\test\operator\divide_num_nonnum.lox",
                "Div called with non-number operand(s): (Float(1.0), String(\"1\"))",
            )
        }

        #[test]
        fn divide() -> Result<(), InterpretError> {
            expect_printed(r"..\test\operator\divide.lox", &["4", "1"])
        }

        #[test]
        fn equals_class() -> Result<(), InterpretError> {
            expect_printed(
                r"..\test\operator\equals_class.lox",
                &[
                    "true", "false", "false", "true", "false", "false", "false", "false",
                ],
            )
        }

        #[test]
        fn equals_method() -> Result<(), InterpretError> {
            expect_printed(r"..\test\operator\equals_method.lox", &["true", "false"])
        }

        #[test]
        fn equals() -> Result<(), InterpretError> {
            expect_printed(
                r"..\test\operator\equals.lox",
                &[
                    "true", "true", "false", "true", "false", "true", "false", "false", "false",
                    "false",
                ],
            )
        }

        #[test]
        fn greater_nonnum_num() -> Result<(), InterpretError> {
            expect_runtime_error(
                r"..\test\operator\greater_nonnum_num.lox",
                "Greater-than called on non-number operand: (String(\"1\"), Float(1.0))",
            )
        }

        #[test]
        fn greater_num_nonnum() -> Result<(), InterpretError> {
            expect_runtime_error(
                r"..\test\operator\greater_num_nonnum.lox",
                "Greater-than called on non-number operand: (Float(1.0), String(\"1\"))",
            )
        }

        #[test]
        fn greater_or_equal_nonnum_num() -> Result<(), InterpretError> {
            expect_runtime_error(
                r"..\test\operator\greater_or_equal_nonnum_num.lox",
                "Greater-than-or-equal called on non-number operand: (String(\"1\"), Float(1.0))",
            )
        }

        #[test]
        fn greater_or_equal_num_nonnum() -> Result<(), InterpretError> {
            expect_runtime_error(
                r"..\test\operator\greater_or_equal_num_nonnum.lox",
                "Greater-than-or-equal called on non-number operand: (Float(1.0), String(\"1\"))",
            )
        }

        #[test]
        fn less_nonnum_num() -> Result<(), InterpretError> {
            expect_runtime_error(
                r"..\test\operator\less_nonnum_num.lox",
                "Less-than called on non-number operand: (String(\"1\"), Float(1.0))",
            )
        }

        #[test]
        fn less_num_nonnum() -> Result<(), InterpretError> {
            expect_runtime_error(
                r"..\test\operator\less_num_nonnum.lox",
                "Less-than called on non-number operand: (Float(1.0), String(\"1\"))",
            )
        }

        #[test]
        fn less_or_equal_nonnum_num() -> Result<(), InterpretError> {
            expect_runtime_error(
                r"..\test\operator\less_or_equal_nonnum_num.lox",
                "Less-than-or-equal called on non-number operand: (String(\"1\"), Float(1.0))",
            )
        }

        #[test]
        fn less_or_equal_num_nonnum() -> Result<(), InterpretError> {
            expect_runtime_error(
                r"..\test\operator\less_or_equal_num_nonnum.lox",
                "Less-than-or-equal called on non-number operand: (Float(1.0), String(\"1\"))",
            )
        }

        #[test]
        fn multiply_nonnum_num() -> Result<(), InterpretError> {
            expect_runtime_error(
                r"..\test\operator\multiply_nonnum_num.lox",
                "Mul called on non-number operand(s): (String(\"1\"), Float(1.0))",
            )
        }

        #[test]
        fn multiply_num_nonnum() -> Result<(), InterpretError> {
            expect_runtime_error(
                r"..\test\operator\multiply_num_nonnum.lox",
                "Mul called on non-number operand(s): (Float(1.0), String(\"1\"))",
            )
        }

        #[test]
        fn multiply() -> Result<(), InterpretError> {
            expect_printed(r"..\test\operator\multiply.lox", &["15", "3.702"])
        }

        #[test]
        fn negate_nonnum() -> Result<(), InterpretError> {
            expect_runtime_error(
                r"..\test\operator\negate_nonnum.lox",
                "Negate called with non-number operand: String(\"s\")",
            )
        }

        #[test]
        fn negate() -> Result<(), InterpretError> {
            expect_printed(r"..\test\operator\negate.lox", &["-3", "3", "-3"])
        }

        #[test]
        fn not_class() -> Result<(), InterpretError> {
            expect_printed(r"..\test\operator\not_class.lox", &["false", "false"])
        }

        #[test]
        fn not_equals() -> Result<(), InterpretError> {
            expect_printed(
                r"..\test\operator\not_equals.lox",
                &[
                    "false", "false", "true", "false", "true", "false", "true", "true", "true",
                    "true",
                ],
            )
        }

        #[test]
        fn not() -> Result<(), InterpretError> {
            expect_printed(
                r"..\test\operator\not.lox",
                &[
                    "false", "true", "true", "false", "false", "true", "false", "false",
                ],
            )
        }

        #[test]
        fn subtract_nonnum_num() -> Result<(), InterpretError> {
            expect_runtime_error(
                r"..\test\operator\subtract_nonnum_num.lox",
                "Sub called on non-number operand(s): (String(\"1\"), Float(1.0))",
            )
        }

        #[test]
        fn subtract_num_nonnum() -> Result<(), InterpretError> {
            expect_runtime_error(
                r"..\test\operator\subtract_num_nonnum.lox",
                "Sub called on non-number operand(s): (Float(1.0), String(\"1\"))",
            )
        }

        #[test]
        fn subtract() -> Result<(), InterpretError> {
            expect_printed(r"..\test\operator\subtract.lox", &["1", "0"])
        }
    }

    mod print {
        use super::*;

        #[test]
        fn missing_argument() -> Result<(), InterpretError> {
            expect_compile_error(r"..\test\print\missing_argument.lox")
        }
    }

    mod regression {
        use super::*;

        #[test]
        fn _40() -> Result<(), InterpretError> {
            expect_printed(r"..\test\regression\40.lox", &["false"])
        }

        #[test]
        fn _394() -> Result<(), InterpretError> {
            expect_printed(r"..\test\regression\394.lox", &["Class(\"B\")"])
        }
    }

    mod return_stmt {
        use super::*;

        #[test]
        fn after_else() -> Result<(), InterpretError> {
            expect_printed(r"..\test\return\after_else.lox", &["ok"])
        }

        #[test]
        fn after_if() -> Result<(), InterpretError> {
            expect_printed(r"..\test\return\after_if.lox", &["ok"])
        }

        #[test]
        fn after_while() -> Result<(), InterpretError> {
            expect_printed(r"..\test\return\after_while.lox", &["ok"])
        }

        #[test]
        fn at_top_level() -> Result<(), InterpretError> {
            expect_compile_error(r"..\test\return\at_top_level.lox")
        }

        #[test]
        fn in_function() -> Result<(), InterpretError> {
            expect_printed(r"..\test\return\in_function.lox", &["ok"])
        }

        #[test]
        fn in_method() -> Result<(), InterpretError> {
            expect_printed(r"..\test\return\in_method.lox", &["ok"])
        }

        #[test]
        fn return_nil_if_no_value() -> Result<(), InterpretError> {
            expect_printed(r"..\test\return\return_nil_if_no_value.lox", &["nil"])
        }
    }

    mod scanning {
        use super::*;

        type T = Token;
        use TokenKind::*;

        #[test]
        fn identifiers() {
            expect_scanner(
                r"..\test\scanning\identifiers.lox",
                &[
                    T::new(Ident, "andy", 1),
                    T::new(Ident, "formless", 1),
                    T::new(Ident, "fo", 1),
                    T::new(Ident, "_", 1),
                    T::new(Ident, "_123", 1),
                    T::new(Ident, "_abc", 1),
                    T::new(Ident, "ab123", 1),
                    T::new(
                        Ident,
                        "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890_",
                        2,
                    ),
                    T::new(EOF, "", 13),
                ],
            )
        }

        #[test]
        fn keywords() {
            expect_scanner(
                r"..\test\scanning\keywords.lox",
                &[
                    T::new(And, "and", 1),
                    T::new(Class, "class", 1),
                    T::new(Else, "else", 1),
                    T::new(False, "false", 1),
                    T::new(For, "for", 1),
                    T::new(Fun, "fun", 1),
                    T::new(If, "if", 1),
                    T::new(Nil, "nil", 1),
                    T::new(Or, "or", 1),
                    T::new(Return, "return", 1),
                    T::new(Super, "super", 1),
                    T::new(This, "this", 1),
                    T::new(True, "true", 1),
                    T::new(Var, "var", 1),
                    T::new(While, "while", 1),
                    T::new(EOF, "", 19),
                ],
            )
        }

        #[test]
        fn numbers() {
            expect_scanner(
                r"..\test\scanning\numbers.lox",
                &[
                    T::new(Number, "123", 1),
                    T::new(Number, "123.456", 2),
                    T::new(Dot, ".", 3),
                    T::new(Number, "456", 3),
                    T::new(Number, "123", 4),
                    T::new(Dot, ".", 4),
                    T::new(EOF, "", 13),
                ],
            )
        }

        #[test]
        fn punctuators() {
            expect_scanner(
                r"..\test\scanning\punctuators.lox",
                &[
                    T::new(LeftParen, "(", 1),
                    T::new(RightParen, ")", 1),
                    T::new(LeftBrace, "{", 1),
                    T::new(RightBrace, "}", 1),
                    T::new(Semicolon, ";", 1),
                    T::new(Comma, ",", 1),
                    T::new(Plus, "+", 1),
                    T::new(Minus, "-", 1),
                    T::new(Star, "*", 1),
                    T::new(NotEq, "!=", 1),
                    T::new(EqEq, "==", 1),
                    T::new(LtEq, "<=", 1),
                    T::new(GtEq, ">=", 1),
                    T::new(NotEq, "!=", 1),
                    T::new(Lt, "<", 1),
                    T::new(Gt, ">", 1),
                    T::new(Slash, "/", 1),
                    T::new(Dot, ".", 1),
                    T::new(EOF, "", 22),
                ],
            )
        }

        #[test]
        fn strings() {
            expect_scanner(
                r"..\test\scanning\strings.lox",
                &[
                    T::new(String, "\"\"", 1),
                    T::new(String, "\"string\"", 2),
                    T::new(EOF, "", 7),
                ],
            )
        }

        #[test]
        fn whitespace() {
            expect_scanner(
                r"..\test\scanning\whitespace.lox",
                &[
                    T::new(Ident, "space", 1),
                    T::new(Ident, "tabs", 1),
                    T::new(Ident, "newlines", 1),
                    T::new(Ident, "end", 6),
                    T::new(EOF, "", 13),
                ],
            )
        }
    }

    mod string {
        use super::*;

        #[test]
        fn error_after_multiline() -> Result<(), InterpretError> {
            expect_runtime_error(
                r"..\test\string\error_after_multiline.lox",
                "[cycle: 3] Undefined variable 'err'.",
            )
        }

        #[test]
        fn literals() -> Result<(), InterpretError> {
            expect_printed(
                r"..\test\string\literals.lox",
                &["()", "a string", "A~¶Þॐஃ"],
            )
        }

        #[test]
        fn multiline() -> Result<(), InterpretError> {
            expect_printed(r"..\test\string\multiline.lox", &["1\r\n2\r\n3"])
        }

        #[test]
        fn unterminated() -> Result<(), InterpretError> {
            expect_compile_error(r"..\test\string\unterminated.lox")
        }
    }

    mod super_ {
        use super::*;

        #[test]
        fn bound_method() -> Result<(), InterpretError> {
            expect_printed(r"..\test\super\bound_method.lox", &["A.method(arg)"])
        }

        #[test]
        fn call_other_method() -> Result<(), InterpretError> {
            expect_printed(
                r"..\test\super\call_other_method.lox",
                &["Derived.bar()", "Base.foo()"],
            )
        }

        #[test]
        fn call_same_method() -> Result<(), InterpretError> {
            expect_printed(
                r"..\test\super\call_same_method.lox",
                &["Derived.foo()", "Base.foo()"],
            )
        }

        #[test]
        fn closure() -> Result<(), InterpretError> {
            expect_printed(r"..\test\super\closure.lox", &["Base"])
        }

        #[test]
        fn constructor() -> Result<(), InterpretError> {
            expect_printed(
                r"..\test\super\constructor.lox",
                &["Derived.init()", "Base.init(a, b)"],
            )
        }

        #[test]
        fn extra_arguments() -> Result<(), InterpretError> {
            expect_runtime_error(
                r"..\test\super\extra_arguments.lox",
                "[cycle: 28] Function(foo) expects 2 args, got 4.",
            )
        }

        #[test]
        fn indirectly_inherited() -> Result<(), InterpretError> {
            expect_printed(
                r"..\test\super\indirectly_inherited.lox",
                &["C.foo()", "A.foo()"],
            )
        }

        #[test]
        fn missing_arguments() -> Result<(), InterpretError> {
            expect_runtime_error(r"..\test\super\missing_arguments.lox", "[cycle: 23] Function(foo) expects 2 args, got 1.")
        }

        #[test]
        fn no_superclass_bind() -> Result<(), InterpretError> {
            expect_compile_error(r"..\test\super\no_superclass_bind.lox")
        }

        #[test]
        fn no_superclass_call() -> Result<(), InterpretError> {
            expect_compile_error(r"..\test\super\no_superclass_call.lox")
        }

        #[test]
        fn no_superclass_method() -> Result<(), InterpretError> {
            expect_runtime_error(
                r"..\test\super\no_superclass_method.lox",
                "[cycle 21] Undefined method doesNotExist for class Base",
            )
        }

        #[test]
        fn parenthesized() -> Result<(), InterpretError> {
            expect_compile_error(r"..\test\super\parenthesized.lox")
        }

        #[test]
        fn reassign_superclass() -> Result<(), InterpretError> {
            expect_printed(
                r"..\test\super\reassign_superclass.lox",
                &["Base.method()", "Base.method()"],
            )
        }

        #[test]
        fn super_at_top_level() -> Result<(), InterpretError> {
            expect_compile_error(r"..\test\super\super_at_top_level.lox")
        }

        #[test]
        fn super_in_closure_in_inherited_method() -> Result<(), InterpretError> {
            expect_printed(
                r"..\test\super\super_in_closure_in_inherited_method.lox",
                &["A"],
            )
        }

        #[test]
        fn super_in_cinherited_method() -> Result<(), InterpretError> {
            expect_printed(r"..\test\super\super_in_inherited_method.lox", &["A"])
        }

        #[test]
        fn super_in_top_level_function() -> Result<(), InterpretError> {
            expect_compile_error(r"..\test\super\super_in_top_level_function.lox")
        }

        #[test]
        fn super_without_dot() -> Result<(), InterpretError> {
            expect_compile_error(r"..\test\super\super_without_dot.lox")
        }

        #[test]
        fn super_without_name() -> Result<(), InterpretError> {
            expect_compile_error(r"..\test\super\super_without_name.lox")
        }
    }

    mod this {
        use super::*;

        #[test]
        fn closure() -> Result<(), InterpretError> {
            expect_printed(r"..\test\this\closure.lox", &["Foo"])
        }

        #[test]
        fn nested_class() -> Result<(), InterpretError> {
            expect_printed(
                r"..\test\this\nested_class.lox",
                &["Outer{}", "Outer{}", "Inner{}"],
            )
        }

        #[test]
        fn nested_closure() -> Result<(), InterpretError> {
            expect_printed(r"..\test\this\nested_closure.lox", &["Foo"])
        }

        #[test]
        fn this_at_top_level() -> Result<(), InterpretError> {
            expect_compile_error(r"..\test\this\this_at_top_level.lox")
        }

        #[test]
        fn this_in_method() -> Result<(), InterpretError> {
            expect_printed(r"..\test\this\this_in_method.lox", &["baz"])
        }

        #[test]
        fn this_in_top_level_function() -> Result<(), InterpretError> {
            expect_compile_error(r"..\test\this\this_in_top_level_function.lox")
        }
    }

    mod variable {
        use super::*;

        #[test]
        fn collide_with_parameters() -> Result<(), InterpretError> {
            expect_compile_error(r"..\test\variable\collide_with_parameter.lox")
        }

        #[test]
        fn duplicate_local() -> Result<(), InterpretError> {
            expect_compile_error(r"..\test\variable\duplicate_local.lox")
        }

        #[test]
        fn duplicate_parameter() -> Result<(), InterpretError> {
            expect_compile_error(r"..\test\variable\duplicate_parameter.lox")
        }

        #[test]
        fn early_bound() -> Result<(), InterpretError> {
            expect_printed(r"..\test\variable\early_bound.lox", &["outer", "outer"])
        }

        #[test]
        fn in_middle_of_block() -> Result<(), InterpretError> {
            expect_printed(
                r"..\test\variable\in_middle_of_block.lox",
                &["a", "a b", "a c", "a b d"],
            )
        }

        #[test]
        fn in_nested_block() -> Result<(), InterpretError> {
            expect_printed(r"..\test\variable\in_nested_block.lox", &["outer"])
        }

        #[test]
        fn local_from_method() -> Result<(), InterpretError> {
            expect_printed(r"..\test\variable\local_from_method.lox", &["variable"])
        }

        #[test]
        fn redeclare_global() -> Result<(), InterpretError> {
            expect_printed(r"..\test\variable\redeclare_global.lox", &["nil"])
        }

        #[test]
        fn redefine_global() -> Result<(), InterpretError> {
            expect_printed(r"..\test\variable\redefine_global.lox", &["2"])
        }

        #[test]
        fn scope_reuse_in_different_blocks() -> Result<(), InterpretError> {
            expect_printed(
                r"..\test\variable\scope_reuse_in_different_blocks.lox",
                &["first", "second"],
            )
        }

        #[test]
        fn shadow_and_local() -> Result<(), InterpretError> {
            expect_printed(
                r"..\test\variable\shadow_and_local.lox",
                &["outer", "inner"],
            )
        }

        #[test]
        fn shadow_global() -> Result<(), InterpretError> {
            expect_printed(r"..\test\variable\shadow_global.lox", &["shadow", "global"])
        }

        #[test]
        fn shadow_local() -> Result<(), InterpretError> {
            expect_printed(r"..\test\variable\shadow_local.lox", &["shadow", "local"])
        }

        #[test]
        fn undefined_global() -> Result<(), InterpretError> {
            expect_runtime_error(
                r"..\test\variable\undefined_global.lox",
                "[cycle: 1] Undefined variable 'notDefined'.",
            )
        }

        #[test]
        fn undefined_local() -> Result<(), InterpretError> {
            expect_runtime_error(
                r"..\test\variable\undefined_local.lox",
                "[cycle: 1] Undefined variable 'notDefined'.",
            )
        }

        #[test]
        fn uninitialized() -> Result<(), InterpretError> {
            expect_printed(r"..\test\variable\uninitialized.lox", &["nil"])
        }

        #[test]
        fn unreached_undefinied() -> Result<(), InterpretError> {
            expect_printed(r"..\test\variable\unreached_undefined.lox", &["ok"])
        }

        #[test]
        fn use_false_as_var() -> Result<(), InterpretError> {
            expect_compile_error(r"..\test\variable\use_false_as_var.lox")
        }

        #[test]
        fn use_global_in_initializer() -> Result<(), InterpretError> {
            expect_printed(
                r"..\test\variable\use_global_in_initializer.lox",
                &["value"],
            )
        }

        #[test]
        fn use_local_in_initializer() -> Result<(), InterpretError> {
            expect_compile_error(r"..\test\variable\use_local_in_initializer.lox")
        }

        #[test]
        fn use_nil_as_var() -> Result<(), InterpretError> {
            expect_compile_error(r"..\test\variable\use_nil_as_var.lox")
        }

        #[test]
        fn use_this_as_var() -> Result<(), InterpretError> {
            expect_compile_error(r"..\test\variable\use_this_as_var.lox")
        }
    }

    mod while_loop {
        use super::*;

        #[test]
        fn class_in_body() -> Result<(), InterpretError> {
            expect_compile_error(r"..\test\while\class_in_body.lox")
        }

        #[test]
        fn closure_in_body() -> Result<(), InterpretError> {
            expect_printed(r"..\test\while\closure_in_body.lox", &["1", "2", "3"])
        }

        #[test]
        fn fun_in_body() -> Result<(), InterpretError> {
            expect_compile_error(r"..\test\while\fun_in_body.lox")
        }

        #[test]
        fn return_closure() -> Result<(), InterpretError> {
            expect_printed(r"..\test\while\return_closure.lox", &["i"])
        }

        #[test]
        fn return_inside() -> Result<(), InterpretError> {
            expect_printed(r"..\test\while\return_inside.lox", &["i"])
        }

        #[test]
        fn syntax() -> Result<(), InterpretError> {
            expect_printed(r"..\test\while\syntax.lox", &["1", "2", "3", "0", "1", "2"])
        }

        #[test]
        fn var_in_body() -> Result<(), InterpretError> {
            expect_compile_error(r"..\test\while\var_in_body.lox")
        }
    }

    #[test]
    fn precendence() -> Result<(), InterpretError> {
        expect_printed(
            r"..\test\precedence.lox",
            &[
                "14", "8", "4", "0", "true", "true", "true", "true", "0", "0", "0", "0", "4",
            ],
        )
    }
}
