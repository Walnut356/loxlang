use env_logger::Builder;
use log::{Level, info};
use rslox::{chunk::*, value::Value, vm::VM};

const LOG_LEVEL: Level = Level::Trace;

fn main() {
    init_log();

    let mut chunk = Chunk::default();

    chunk.push_opcode(OpCode::Constant, 0);
    let idx = chunk.push_constant(Value::Float(10.0));
    chunk.data.push(idx);
    chunk.push_opcode(OpCode::Negate, 1);
    chunk.insert_constant(Value::Float(20.0), 2);
    chunk.push_opcode(OpCode::Add, 3);
    chunk.push_opcode(OpCode::Return, 4);

    let mut vm = VM::default();

    info!("{:?}", vm.interpret(&chunk));
}

fn init_log() {
    unsafe {
        std::env::set_var("RUST_LOG", LOG_LEVEL.as_str());
    }
    let mut builder = Builder::from_default_env();
    builder.target(env_logger::Target::Stdout);
    builder.init();
}
