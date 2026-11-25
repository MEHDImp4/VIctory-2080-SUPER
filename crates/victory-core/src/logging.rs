//! One tracing setup for every binary.
//!
//! Filter with `VICTORY_LOG` (falls back to `RUST_LOG`), e.g.
//! `VICTORY_LOG=victory_loader=trace,info`. Debug and trace output exist
//! only in builds with the `diagnostics` feature; a plain release build
//! logs info and above whatever the filter asks for.
//! `VICTORY_LOG_FILE` also appends these events to a plain-text file.

use tracing_subscriber::EnvFilter;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::fmt::writer::{BoxMakeWriter, MakeWriterExt};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

/// The most verbose level this build can log.
const CAP: LevelFilter = if cfg!(feature = "diagnostics") {
    LevelFilter::TRACE
} else {
    LevelFilter::INFO
};

pub fn init(default_level: &str) {
    let filter = std::env::var("VICTORY_LOG")
        .or_else(|_| std::env::var("RUST_LOG"))
        .ok()
        .and_then(|v| EnvFilter::try_new(v).ok())
        .unwrap_or_else(|| EnvFilter::new(default_level));
    let writer = std::env::var_os("VICTORY_LOG_FILE").and_then(|path| {
        match std::fs::OpenOptions::new().create(true).append(true).open(&path) {
            Ok(file) => Some(BoxMakeWriter::new(std::io::stderr.and(std::sync::Mutex::new(file)))),
            Err(error) => {
                eprintln!("cannot open VIctory log {}: {error}", std::path::Path::new(&path).display());
                None
            }
        }
    });
    let _ = tracing_subscriber::registry()
        .with(CAP)
        .with(filter)
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(true)
                .with_ansi(writer.is_none())
                .with_writer(writer.unwrap_or_else(|| BoxMakeWriter::new(std::io::stderr))),
        )
        .try_init();
}
