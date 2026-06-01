mod config;
mod kmeans;
mod kmeans_msc;
mod kmeans_wn;
mod shader_window;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Kmeans {
        path: Option<String>,
        #[arg(short, long)]
        args: Option<String>,
    },
    KmeansMsc {
        path: Option<String>,
    },
    KmeansWn {
        path: Option<String>,
        #[arg(short, long)]
        noise: Option<f32>,
        #[arg(short, long)]
        args: Option<String>,
    },
    View {
        path: Option<String>,
        #[arg(long)]
        algo: Option<String>,
        #[arg(short, long)]
        noise: Option<f32>,
        #[arg(short, long)]
        args: Option<String>,
        #[arg(long)]
        speed: Option<f32>,
        #[arg(long)]
        zoom: Option<f32>,
        #[arg(long)]
        blur: Option<f32>,
        #[arg(long)]
        edge_blur: Option<f32>,
        #[arg(long)]
        grain: Option<f32>,
        #[arg(long)]
        equalize: Option<f32>,
        #[arg(long)]
        chroma_bias: Option<f32>,
        #[arg(long)]
        mono_bias: Option<f32>,
    },
}

fn resolve_path(cli_path: Option<String>) -> String {
    if let Some(p) = cli_path {
        p
    } else if let Some(cfg) = config::load_config() {
        cfg.settings.image
    } else {
        eprintln!("Error: No image path provided and config.toml not found/invalid.");
        std::process::exit(1);
    }
}

fn main() {
    let cli = Cli::parse();
    let cfg = config::load_config();

    match cli.command {
        Commands::Kmeans { path, args } => {
            let p = resolve_path(path);
            kmeans::run_pure_kmeans(&p, args.as_deref().unwrap_or(""));
        }
        Commands::KmeansMsc { path } => {
            let p = resolve_path(path);
            kmeans_msc::run_hybrid_msc(&p);
        }
        Commands::KmeansWn { path, noise, args } => {
            let p = resolve_path(path);
            kmeans_wn::run_kmeans_wn(&p, noise, args.as_deref().unwrap_or(""));
        }
        Commands::View { 
            path, 
            algo, 
            noise, 
            args,
            speed,
            zoom,
            blur,
            edge_blur,
            grain,
            equalize,
            chroma_bias,
            mono_bias,
        } => {
            let p = resolve_path(path);
            let algorithm = algo.unwrap_or_else(|| "kmeans".to_string());
            let k = args.as_deref()
                .unwrap_or("")
                .split(',')
                .find(|s| s.trim().starts_with("k="))
                .and_then(|s| s.trim().strip_prefix("k="))
                .and_then(|val| val.parse::<usize>().ok())
                .unwrap_or(10)
                .clamp(1, 24);
            
            let palette = if algorithm == "kmeans-wn" {
                kmeans_wn::extract_palette_wn(&p, k, noise.unwrap_or(0.01))
            } else {
                kmeans::extract_palette(&p, k)
            };

            println!("\nExtracted Palette ({}):", algorithm);
            for (i, (lab, ratio)) in palette.iter().enumerate() {
                let hex = kmeans::lab_to_hex(*lab);
                println!("  {}: {} | Ratio: {:.4}", i + 1, hex, ratio);
            }
            
            let s_cfg = cfg.as_ref().and_then(|c| c.shader.as_ref());
            
            let params = shader_window::ShaderParams {
                speed: speed.or(s_cfg.and_then(|s| s.speed)).unwrap_or(0.003),
                zoom: zoom.or(s_cfg.and_then(|s| s.zoom)).unwrap_or(1.1),
                blur: blur.or(s_cfg.and_then(|s| s.blur)).unwrap_or(0.66),
                edge_blur: edge_blur.or(s_cfg.and_then(|s| s.edge_blur)).unwrap_or(2.0),
                grain: grain.or(s_cfg.and_then(|s| s.grain)).unwrap_or(0.01),
                equalize: equalize.or(s_cfg.and_then(|s| s.equalize)).unwrap_or(0.6),
                chroma_bias: chroma_bias.or(s_cfg.and_then(|s| s.chroma_bias)).unwrap_or(0.0),
                mono_bias: mono_bias.or(s_cfg.and_then(|s| s.mono_bias)).unwrap_or(0.0),
            };
            
            shader_window::run_shader_window(palette, params);
        }
    }
}
