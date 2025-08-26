use tracing::level_filters::LevelFilter;
use tracing_subscriber::fmt::format::FmtSpan;

pub mod chunk;
pub mod compiler;
pub mod scanner;
pub mod stack;
pub mod table;
pub mod value;
pub mod vm;

// pub fn init_log(log_level: log::Level) {
//     unsafe {
//         std::env::set_var("RUST_LOG", log_level.as_str());
//     }
//     let mut builder = env_logger::Builder::from_default_env();
//     builder
//         .target(env_logger::Target::Stdout)
//         .format_timestamp(None)
//         .format_module_path(false)
//         .format_source_path(true)
//         .format_indent(Some(20));
//     builder.init();
// }

pub fn init_tracing(log_level: impl Into<LevelFilter>) {
    tracing_subscriber::fmt()
        .without_time()
        .with_ansi(true)
        .with_file(false)
        .with_line_number(false)
        .with_max_level(log_level)
        .with_span_events(FmtSpan::ACTIVE)
        .with_thread_names(false)
        .with_thread_ids(false)
        .init();
}
