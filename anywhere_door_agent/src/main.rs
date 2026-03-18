mod service;
#[cfg(windows)]
mod windows_service;

fn main() {
    #[cfg(windows)]
    {
        if std::env::args().any(|arg| arg == "--windows-service") {
            if let Err(err) = windows_service::run_dispatcher() {
                eprintln!("{err}");
                std::process::exit(1);
            }
            return;
        }
    }

    if let Err(err) = service::run_service() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}
