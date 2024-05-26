use clap::{Parser, Subcommand};
use kedo_core::runtime::Runtime;

#[derive(Parser)]
#[command(name = "Kedo")]
#[command(
    version = "0.0.1",
    about = "Kedo",
    long_about = "Kedo is a JavaScript runtime written in Rust and powered by JavaScriptCore"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Sets a custom config file
    // #[arg(short, long, value_name = "FILE")]
    // config: Option<PathBuf>,

    /// Turn debugging information on
    #[arg(short, long, action = clap::ArgAction::Count)]
    debug: u8,
}

#[derive(Subcommand)]
enum Commands {
    /// Run a script
    Run {
        /// Enable strict mode
        #[arg(short, long)]
        strict: bool,

        /// Path to the script
        file: String,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::Run { strict, file }) => {
            if *strict {
                println!("Strict mode enabled");
            }

            let mut runtime = Runtime::new();
            let result = runtime.evaluate_module(file);

            match result {
                Ok(_) => {
                    runtime.idle().await;
                }
                Err(e) => {
                    println!("Error: {}", e);
                }
            }
        }
        None => {}
    }
}
