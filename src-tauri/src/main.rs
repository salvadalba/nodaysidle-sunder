// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Determine log directory: ~/Library/Application Support/com.nodaysidle.sunder/logs/
    let log_dir = dirs_next().join("logs");
    std::fs::create_dir_all(&log_dir).ok();

    // Daily rotating file appender, keep 7 days
    let file_appender = tracing_appender::rolling::daily(&log_dir, "sunder.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    // Set up subscriber with both stdout (dev) and file output
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;

    let fmt_file = tracing_subscriber::fmt::layer()
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_target(true);

    let fmt_stdout = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_ansi(true);

    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info,sunder_lib=debug"));

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt_file)
        .with(fmt_stdout)
        .init();

    tracing::info!("Sunder starting, logs at: {}", log_dir.display());

    sunder_lib::run();
}

/// Resolve the app data directory.
fn dirs_next() -> std::path::PathBuf {
    if let Some(data_dir) = dirs_next_impl() {
        data_dir
    } else {
        // Fallback to current directory
        std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
    }
}

fn dirs_next_impl() -> Option<std::path::PathBuf> {
    #[cfg(target_os = "macos")]
    {
        dirs::data_dir().map(|d| d.join("com.nodaysidle.sunder"))
    }
    #[cfg(target_os = "windows")]
    {
        dirs::data_dir().map(|d| d.join("com.nodaysidle.sunder"))
    }
    #[cfg(target_os = "linux")]
    {
        dirs::data_dir().map(|d| d.join("com.nodaysidle.sunder"))
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        None
    }
}
