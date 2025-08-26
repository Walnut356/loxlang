// use log::Level;
use rslox::{
    init_tracing,
    vm::{InterpretError, VM},
};
use std::{
    fs::File,
    io::{self, Read, Write},
    rc::Rc,
};
use tracing::{Level, info};

const LOG_LEVEL: Level = Level::INFO;

fn main() -> Result<(), InterpretError> {
    init_tracing(LOG_LEVEL);
    let mut args = std::env::args();
    // skip this exe
    args.next();

    if let Some(file_path) = args.next() {
        run_file(&file_path)
    } else {
        repl()
    }
}

fn repl() -> Result<(), InterpretError> {
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

fn run_file(path: &str) -> Result<(), InterpretError> {
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

// #[cfg(test)]
// mod tests {
//     use rslox::*;
//     use crate::*;
//     #[test]
//     fn fib_flat() -> Result<(), InterpretError> {
//         init_log(Level::Info);

//         run_file(r"C:\coding\Projects\learning\loxlang\test_files\fib.lox")
//     }

//     #[test]
//     fn fib_func() -> Result<(), InterpretError> {
//         init_log(Level::Info);

//         run_file(r"C:\coding\Projects\learning\loxlang\test_files\fib_func.lox")
//     }

//     #[test]
//     fn global_closure() -> Result<(), InterpretError> {
//         init_log(Level::Info);

//         run_file(r"C:\coding\Projects\learning\loxlang\test_files\global_closure.lox")
//     }

//     #[test]
//     fn resolver() -> Result<(), InterpretError> {
//         init_log(Level::Info);

//         run_file(r"C:\coding\Projects\learning\loxlang\test_files\test_resolver.lox")
//     }

//     #[test]
//     fn scope() -> Result<(), InterpretError> {
//         init_log(Level::Info);

//         run_file(r"C:\coding\Projects\learning\loxlang\test_files\scope.lox")
//     }
// }
