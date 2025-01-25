mod module_manager;
mod module_scanner;

use bundler::BundleArgs;
use clap::{Parser, Subcommand};
use module_manager::ModuleManager;

#[derive(Parser)]
#[command(name = "Kedo")]
#[command(
    version = "0.0.1",
    about = "kedo_js",
    long_about = "Kedo standard library"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Turn debugging information on
    #[arg(short, long, action = clap::ArgAction::Count)]
    debug: u8,
}

#[derive(Subcommand)]
enum Commands {
    /// Build a project
    Bundle {
        /// Path where the output files will be written
        #[arg(short, long)]
        output: String,

        /// Minify the output
        #[arg(short, long)]
        minify: bool,
    },
}

fn main() {
    let cli = Cli::parse();
    let std_path = "kedo_js/@std";
    let mut manager = ModuleManager::new(std_path);
    manager.scan().expect("Failed to scan modules");

    manager.add_external_module("@kedo/internal/utils".to_string());
    manager.add_entry("index.ts".to_string());

    let entries = manager.get_entries().clone();
    let external_modules = manager.get_externals().clone();

    // print total modules and list of modules found
    println!("Total modules: \x1b[32m{}\x1b[0m", manager.len());
    for module in &entries {
        println!("Found module: \x1b[32m{}\x1b[0m", module);
    }

    match &cli.command {
        Some(Commands::Bundle { output, minify }) => {
            let mut total_time = 0;

            for entry in &entries {
                let entry_path = format!("{}/{}", std_path, entry);
                let output_path =
                    format!("{}/{}.js", output, entry.split('.').next().unwrap());
                let args = BundleArgs {
                    external_modules: external_modules.clone(),
                    entries: vec![(entry_path.clone(), entry_path.clone().into())],
                    outputs: vec![output_path.into()],
                    minify: *minify,
                };

                let result = bundler::bundle(args);
                match result {
                    Ok(build_result) => {
                        total_time += build_result.duration.as_millis();
                        println!(
                            "Bundled in \x1b[32m{:?}ms\x1b[0m",
                            build_result.duration.as_millis()
                        );
                    }
                    Err(e) => {
                        panic!("Failed to bundle: {:?}", e);
                    }
                }
            }

            println!("Total time: \x1b[32m{:?}ms\x1b[0m", total_time);
        }
        None => {}
    }
}
