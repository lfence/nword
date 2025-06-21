use clap::{Parser, Subcommand};
use tracing_subscriber;
use std::time::Instant;
use tracing::info;
use std::io::{self};

mod build;
mod query;
mod trie;

#[derive(Parser)]
#[command(name = "nword")]
#[command(about = "High-Performance N-gram Processor")]
struct Args {
    #[arg(short, long, default_value_t = String::from("warn"), help = "Enable verbose output info|warn")]
    verbose: String,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Build {
        #[arg(help = "Input token file")]
        input: String,

        #[arg(help = "Output directory for n-gram files")]
        output_dir: String,
    },
    Query {
        #[arg(help = "N-gram database directory")]
        database_dir: String,

        #[arg(long, help = "suffix search", short='s')]
        suffix_mode: bool,

        #[arg(long, help = "prefix search", short='p')]
        prefix_mode: bool,

        #[arg(long, short, default_value_t = 4, help = "frequency minimum")]
        freq_min: u32,

        #[arg(long, short, default_value_t = 2, help = "max depth")]
        max_depth: u32,
    },
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();
    let log_level = { args.verbose };
    tracing_subscriber::fmt()
        .with_timer(tracing_subscriber::fmt::time::uptime())
        .with_env_filter(log_level)
        .with_writer(std::io::stderr)
        .init();

    let t = Instant::now();
    let result = match args.command {
        Commands::Build { input, output_dir } => {
            build::run(&input, &output_dir);
            Ok(())
        }
        Commands::Query { database_dir, prefix_mode, suffix_mode, freq_min, max_depth } => {
            let mut opts = query::Options {
                freq_min,
                suffix_mode,
                prefix_mode,
                max_depth,
            };
            // autoselect prefix search if none selected
            opts.prefix_mode |= !opts.suffix_mode && !opts.prefix_mode;
            info!("{:?}", opts);
            match query::run(&database_dir, opts) {
                Ok(_) => Ok(()),
                Err(ref e) if e.kind() == io::ErrorKind::BrokenPipe => Ok(()),
                Err(e) => Err(e)
            }
        }
    };
    info!("Finished in {:.2}s", t.elapsed().as_secs_f64());
    result
}
