mod compile;
mod harvest;
mod manifest;
mod query;
mod x;
mod server;
mod update;
mod interface;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use libvellum::utils::expand_path;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Harvest {
        #[arg(value_name = "PATHS", required = true, num_args = 1..)]
        paths: Vec<String>,
        #[arg(long)]
        pretty: bool,
        #[arg(long, short = 'j')]
        jobs: Option<usize>,
    },
    Server {
        #[arg(long, default_value = "8000")]
        port: u16,
    },
    Interface {
        #[arg(value_name = "NAME")]
        name: Option<String>,
    },
    Compile {
        #[arg(value_name = "PATH", required = true)]
        path: String,
        #[arg(long)]
        stdout: bool,
        #[arg(long)]
        intermediary: bool,
        #[arg(long)]
        pretty: bool,
        #[arg(long, value_delimiter = ',')]
        flags: Vec<String>,
    },
    Update {
        #[arg(value_name = "PATH")]
        path: Option<String>,
        #[arg(long)]
        force: bool,
        #[arg(long, short = 'j')]
        jobs: Option<usize>,
        #[arg(long)]
        verbose: bool,
        #[arg(long)]
        silent: bool,
    },
    Manifest {
        #[arg(value_name = "PATH")]
        path: Option<String>,
        #[arg(long)]
        force: bool,
        #[arg(long, required_unless_present = "library", conflicts_with = "library")]
        album: bool,
        #[arg(long, required_unless_present = "album", conflicts_with = "album")]
        library: bool,
        #[arg(long)]
        stdout: bool,
    },
    X {
        #[arg(value_name = "NAME", required = true)]
        name: String,
        #[arg(long, short = 'p')]
        playing: bool,
        #[arg(long)]
        id: Option<String>,
        #[arg(long, short = 'q', conflicts_with_all = ["playing", "id"])]
        query: Option<String>,
        #[arg(long, short = 'f', conflicts_with_all = ["playing", "id", "query"])]
        file: Option<String>,
        #[arg(long)]
        debug: bool,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    Query {
        #[arg(value_name = "QUERY")]
        query_str: Option<String>,
        #[arg(long)]
        playing: bool,
        #[arg(long)]
        lock: bool,
        #[arg(long)]
        id: bool,
        #[arg(long)]
        uid: bool,
        #[arg(long)]
        json: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    simple_logger::SimpleLogger::new()
        .with_level(log::LevelFilter::Info)
        .with_module_level("mpd_protocol", log::LevelFilter::Warn)
        .with_module_level("mpd_client", log::LevelFilter::Warn)
        .with_module_level("tracing", log::LevelFilter::Warn)
        .env()
        .init()
        .ok();

    let cli = Cli::parse();

    match cli.command {
        Commands::Harvest {
            paths,
            pretty,
            jobs,
        } => {
            if let Some(j) = jobs {
                rayon::ThreadPoolBuilder::new()
                    .num_threads(j)
                    .build_global()
                    .context("Failed to build thread pool")?;
            }

            let mut targets = Vec::new();
            for p in paths {
                let expanded = expand_path(&p);
                if let Ok(canon) = expanded.canonicalize() {
                    targets.push(canon);
                } else {
                    targets.push(expanded);
                }
            }

            harvest::run(targets, pretty);
            Ok(())
        }
        Commands::Server { port } => server::run(port).await,
        Commands::Interface { name } => interface::execute(name).await,
        Commands::Compile {
            path,
            stdout,
            intermediary,
            pretty,
            flags,
        } => {
            let expanded = expand_path(&path);
            let options = compile::CompileOptions {
                target_path: expanded,
                flags,
                specific_albums: None,
                jobs: None,
                notify_tx: None,
                compile_flags: compile::CompileFlags {
                    mode: if intermediary {
                        compile::CompileMode::Intermediary
                    } else {
                        compile::CompileMode::Standard
                    },
                    target: if stdout {
                        compile::ExportTarget::Stdout
                    } else {
                        compile::ExportTarget::File
                    },
                    pretty,
                },
            };
            compile::run(options).await
        }
        Commands::Update {
            path,
            force,
            jobs,
            verbose,
            silent,
        } => {
            let expanded = path.map(|p| expand_path(&p));
            update::run(expanded, force, jobs, verbose, silent).await
        }
        Commands::Manifest { force, path, album, library: _, stdout } => {
            let expanded = path.map(|p| expand_path(&p));
            let mode = if album { manifest::ManifestMode::Album } else { manifest::ManifestMode::Library };
            let options = manifest::ManifestOptions { mode, force, stdout };
            manifest::run(expanded, &options)
        }
        Commands::X { name, playing, id, query, file, debug, args } => x::execute(name, playing, id, query, file, debug, args).await,
        Commands::Query { query_str, playing, lock, id, uid, json } => {
            let flags = query::QueryFlags { playing, lock, id, uid, json };
            query::run(query_str, flags).await
        }
    }
}
