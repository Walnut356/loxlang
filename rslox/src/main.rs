use env_logger::Builder;
use log::{Level, info};
use rslox::{chunk::*, value::Value, vm::VM};
use std::{fs::File, io::{self, BufReader, Read, Write}};

const LOG_LEVEL: Level = Level::Trace;

fn main() {
    let mut args = std::env::args();
    // skip this exe
    args.next();

    if let Some(file_path) = args.next() {
        run_file(&file_path)
    } else {
        repl()
    }

    // info!("{:?}", vm.interpret(&chunk));
}

fn repl() {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    let mut vm = VM::default();

    let mut buffer = String::new();

    loop {
        write!(stdout, "> ").unwrap();

        stdin.read_line(&mut buffer).unwrap();

        info!("{:?}", vm.interpret(&buffer));

        // TODO interpret
    }
}

fn run_file(path: &str) {
    let mut f = File::open(path).unwrap();
    let mut buffer = String::new();
    f.read_to_string(&mut buffer).unwrap();

    let mut vm = VM::default();

    info!("{:?}", vm.interpret(&buffer));
}

fn init_log() {
    unsafe {
        std::env::set_var("RUST_LOG", LOG_LEVEL.as_str());
    }
    let mut builder = Builder::from_default_env();
    builder.target(env_logger::Target::Stdout);
    builder.init();
}
