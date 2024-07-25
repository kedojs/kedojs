use std::thread::sleep;

// use bundler::BundleArgs;
use clap::{Parser, Subcommand};
use kedo_core::runtime::Runtime;

const STD_INDEX: &str = include_str!("../src/@std/dist/index.js");

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
    /// Build a project
    Bundle {
        /// Path to the project
        #[arg(short, long)]
        output: String,

        #[arg(short, long)]
        entry: String,

        /// Minify the output
        #[arg(short, long)]
        minify: bool,
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
            // Load the standard library
            // let result =
            //     runtime.evaluate_module_from_source(STD_INDEX, "src/@std/index.js", None);
            // assert!(result.is_ok());
            // println!(
            //     "Result Check: {:?}",
            //     runtime.check_syntax("console.log('Kevin')", None).unwrap()
            // );
            let result = runtime.evaluate_module(file);
            println!("Result: {:?}", "Complete");
            match result {
                Ok(_) => {
                    runtime.idle().await;
                    // sleep(std::time::Duration::from_secs(5));
                    // println!(
                    //     "Result Check: {:?}",
                    //     runtime.check_syntax("console.log('Kevin')", None).unwrap()
                    // );
                    // println!(
                    //     "Result Check: {:?}",
                    //     runtime.link_and_evaluate("4343").as_string().unwrap()
                    // );
                }
                Err(e) => {
                    println!("Error CLI: {}", e.message().unwrap());
                }
            }
        }
        Some(Commands::Bundle {
            output,
            entry,
            minify,
        }) => {
            // let args = BundleArgs {
            //     external_modules: vec!["@kedo/internal/utils".to_string()],
            //     entry: entry.into(),
            //     output: output.into(),
            //     minify: *minify,
            // };

            // let result = bundler::bundle(args);
            // match result {
            //     Ok(_) => {
            //         println!("Bundle complete");
            //     }
            //     Err(e) => {
            //         println!("Error: {}", e);
            //     }
            // }
        }
        None => {}
    }
}
