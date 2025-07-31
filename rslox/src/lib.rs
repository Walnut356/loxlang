pub mod chunk;
pub mod compiler;
pub mod scanner;
pub mod stack;
pub mod table;
pub mod value;
pub mod vm;



pub fn init_log(log_level: log::Level) {
    unsafe {
        std::env::set_var("RUST_LOG", log_level.as_str());
    }
    let mut builder = env_logger::Builder::from_default_env();
    builder
        .target(env_logger::Target::Stdout)
        .format_timestamp(None)
        .format_indent(Some(20));
    builder.init();
}


