use clap::{Parser, Subcommand};
use kedojs::{kedo::Kedo, JsError, JsValue};

#[derive(Parser)]
#[command(name = "Kedojs")]
#[command(
  version = "0.0.1",
  about = "Kedojs",
  long_about = "Kedojs CLI tool for running JavaScript code and managing Kedojs runtime."
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

async fn evaluate_script(path: &str) -> Result<JsValue, JsError> {
  Kedo::new().execute(path).await?
}

#[tokio::main]
async fn main() {
  let cli = Cli::parse();

  match &cli.command {
    Some(Commands::Run { strict, file }) => {
      if *strict {
        println!("Strict mode enabled");
      }

      let result = evaluate_script(file).await;
      match result {
        Ok(resul) => {
          println!("{:?}", resul);
        }
        Err(e) => {
          println!("Error: {}", e);
        }
      }
    }
    None => {}
  }
}
