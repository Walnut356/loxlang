use env_logger::Builder;
use log::{Level, info, trace};
use rslox::
    vm::{InterpretError, VM}
;
use std::{
    fs::File,
    io::{self, Read, Write},
    rc::Rc,
};

const LOG_LEVEL: Level = Level::Trace;

fn main() -> Result<(), InterpretError> {
    init_log();
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
    }
}

fn run_file(path: &str) -> Result<(), InterpretError> {
    let mut f = File::open(path).unwrap();
    let mut buffer = String::new();
    f.read_to_string(&mut buffer).unwrap();

    let mut vm = VM::default();

    let source: Rc<str> = Rc::from(buffer);

    vm.interpret(source)
}

fn init_log() {
    unsafe {
        std::env::set_var("RUST_LOG", LOG_LEVEL.as_str());
    }
    let mut builder = Builder::from_default_env();
    builder
        .target(env_logger::Target::Stdout)
        .format_timestamp(None)
        .format_indent(Some(20));
    builder.init();
}
