use clap::{Parser, Subcommand};
use kedo_runtime::runtime::Runtime;

mod std_loader;

const STD_INDEX: &str = include_str!("../build/@std/dist/index.js");

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

    // Sets a custom config file
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

fn create_tokio_runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .enable_io()
        .enable_time()
        .event_interval(61)
        .max_io_events_per_tick(3024)
        .global_queue_interval(31)
        .max_blocking_threads(8)
        .build()
        .unwrap()
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::Run { strict, file }) => {
            if *strict {
                println!("Strict mode enabled");
            }

            let mut runtime = Runtime::new();
            runtime.add_loader(std_loader::StdModuleLoader::default());
            // Load the standard library
            let result =
                runtime.evaluate_module_from_source(STD_INDEX, "src/@std/index.js", None);
            // let result = runtime.evaluate_module("./build/@std/dist/index.js");
            match result {
                Ok(_) => {}
                Err(e) => {
                    println!("Error: {}", e.message().unwrap());
                }
            }

            create_tokio_runtime().block_on(async {
                let result = runtime.evaluate_module(file);
                match result {
                    Ok(_) => {
                        runtime.idle().await;
                    }
                    Err(e) => {
                        println!("Error CLI: {}", e.message().unwrap());
                    }
                }
            });
        }
        // Some(Commands::Bundle {
        //     output,
        //     entry,
        //     minify,
        // }) => {
        //     // let args = BundleArgs {
        //     //     external_modules: vec!["@kedo:op/web".to_string()],
        //     //     entry: entry.into(),
        //     //     output: output.into(),
        //     //     minify: *minify,
        //     // };

        //     // let result = bundler::bundle(args);
        //     // match result {
        //     //     Ok(_) => {
        //     //         println!("Bundle complete");
        //     //     }
        //     //     Err(e) => {
        //     //         println!("Error: {}", e);
        //     //     }
        //     // }
        // }
        _ => {}
    }
}
