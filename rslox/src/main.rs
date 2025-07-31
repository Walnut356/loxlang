use log::{Level, info};
use rslox::{init_log,
    vm::{InterpretError, VM}}
;
use std::{
    fs::File,
    io::{self, Read, Write},
    rc::Rc,
};

const LOG_LEVEL: Level = Level::Trace;

fn main() -> Result<(), InterpretError> {
    init_log(LOG_LEVEL);
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
            Err(e) => println!("{e}")
        }

        let dur = start.elapsed();

        info!("Execution time: {dur:?}");
    }
}

fn run_file(path: &str) -> Result<(), InterpretError> {
    let start = std::time::Instant::now();
    let mut f = File::open(path).unwrap();
    let mut buffer = String::new();
    f.read_to_string(&mut buffer).unwrap();

    let mut vm = VM::default();

    let source: Rc<str> = Rc::from(buffer);

    let res = vm.interpret(source);

    let dur = start.elapsed();

    info!("Execution time: {dur:?}");

    res
}
