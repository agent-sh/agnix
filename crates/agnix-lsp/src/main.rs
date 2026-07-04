#[cfg(target_os = "linux")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && (args[1] == "--version" || args[1] == "-V") {
        println!("agnix-lsp {}", env!("CARGO_PKG_VERSION"));
        return;
    }

    init_tracing();
    tracing::debug!("starting agnix-lsp");

    if let Err(e) = agnix_lsp::start_server().await {
        tracing::error!(error = %e, "LSP server failed");
        eprintln!("LSP server error: {e}");
        std::process::exit(1);
    }
}

fn init_tracing() {
    use tracing_subscriber::{EnvFilter, fmt, prelude::*};

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("agnix_lsp=info,agnix_core=warn"));

    let _ = tracing_subscriber::registry()
        .with(
            fmt::layer()
                .with_target(true)
                .with_level(true)
                .with_writer(std::io::stderr),
        )
        .with(filter)
        .try_init();
}
