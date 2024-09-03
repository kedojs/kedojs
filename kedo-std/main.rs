use bundler::BundleArgs;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "Kedo")]
#[command(
    version = "0.0.1",
    about = "kedo-std",
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
    let std_path = "kedo-std/@std";
    let entries = vec![
        "index.ts".to_string(),
        "ds/index.ts".to_string(),
        "events/events.ts".to_string(),
        "stream/stream.ts".to_string(),
        "assert/assert.js".to_string(),
        // "web/Headers.ts".to_string(),
        // "web/AbortSignal.ts".to_string(),
        "web/internals.ts".to_string(),
    ];

    let external_modules = vec![
        "@kedo/internal/utils".to_string(),
        "@kedo/ds".to_string(),
        "@kedo/stream".to_string(),
        "@kedo/web/internals".to_string(),
        "@kedo/events".to_string(),
    ];

    match &cli.command {
        Some(Commands::Bundle { output, minify }) => {
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
                        println!("Bundled in {:?}ms", build_result.duration.as_millis());
                    }
                    Err(e) => {
                        println!("Error K: {}", e);
                    }
                }
            }
        }
        None => {}
    }
}
