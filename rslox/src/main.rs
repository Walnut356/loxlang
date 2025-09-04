// use log::Level;
use rslox::{init_tracing, repl, run_file, value::Value, vm::InterpretError};
use tracing::Level;

const LOG_LEVEL: Level = Level::INFO;

fn main() -> Result<(), InterpretError> {
    init_tracing(LOG_LEVEL);

    let mut args = std::env::args();
    // skip this exe
    args.next();

    if let Some(file_path) = args.next() {
        if file_path == "bench" {
            bench()
        } else {
            run_file(&file_path)
        }
    } else {
        repl()
    }
}

fn bench() -> Result<(), InterpretError> {
    run_file("./test/benchmark/binary_trees.lox")?;
    run_file("./test/benchmark/equality.lox")?;
    run_file("./test/benchmark/fib.lox")?;
    run_file("./test/benchmark/instantiation.lox")?;
    run_file("./test/benchmark/invocation.lox")?;
    run_file("./test/benchmark/method_call.lox")?;
    run_file("./test/benchmark/properties.lox")?;
    run_file("./test/benchmark/string_equality.lox")?;
    run_file("./test/benchmark/trees.lox")?;
    run_file("./test/benchmark/zoo_batch.lox")?;
    run_file("./test/benchmark/zoo.lox")?;

    Ok(())
}
