// use log::Level;
use rslox::{init_tracing, repl, run_file, value::Value, vm::InterpretError};
use tracing::Level;

const LOG_LEVEL: Level = Level::TRACE;

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
