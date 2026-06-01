use std::path::PathBuf;
use std::time::{Duration, Instant};
use walkdir::WalkDir;
use rand::seq::IndexedRandom;
use anyhow::{Result, anyhow};
use fast_image_resize as fr;
use fast_image_resize::images::Image;

fn main() -> Result<()> {
    let library_root = "/run/media/lewelove/1000xhome/backup-everything/FB2K/Library Historyfied!/";
    let target_size = 600;
    let sample_count = 20;

    println!("Scanning for cover files...");
    let mut covers = Vec::new();
    for entry in WalkDir::new(library_root).max_depth(5).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_file() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_lowercase();
            if name == "cover.jpg" || name == "cover.png" {
                covers.push(path.to_path_buf());
            }
        }
    }

    if covers.is_empty() { return Err(anyhow!("No covers found.")); }

    let mut rng = rand::rng();
    let samples: Vec<PathBuf> = covers.sample(&mut rng, sample_count).cloned().collect();

    println!("Starting SIMD Benchmark ({}px Mitchell-Netravali)...", target_size);
    println!("{: <30} | {: <10} | {: <10} | {: <10}", "File", "Decode", "Resize", "Total");
    println!("{}", "-".repeat(70));

    let mut total_decode = Duration::ZERO;
    let mut total_resize = Duration::ZERO;
    let mut successful_runs = 0;

    let mut resizer = fr::Resizer::new();

    for path in samples {
        // 1. DECODE PHASE
        let start_decode = Instant::now();
        let img = match image::open(&path) {
            Ok(m) => m.to_rgba8(),
            Err(_) => continue,
        };
        let decode_dur = start_decode.elapsed();

        // 2. RESIZE PHASE
        let start_resize = Instant::now();
        
        let width = img.width();
        let height = img.height();
        
        let src_image = Image::from_vec_u8(
            width,
            height,
            img.into_raw(),
            fr::PixelType::U8x4,
        ).map_err(|e| anyhow!("Src image error: {:?}", e))?;

        let mut dst_image = Image::new(
            target_size,
            target_size,
            fr::PixelType::U8x4,
        );

        let mut options = fr::ResizeOptions::default();
        // SWAP: FilterType::Lanczos3 -> FilterType::Mitchell
        options.algorithm = fr::ResizeAlg::Convolution(fr::FilterType::Mitchell);

        resizer.resize(&src_image, &mut dst_image, &options)
            .map_err(|e| anyhow!("Resize error: {:?}", e))?;
        
        let resize_dur = start_resize.elapsed();

        total_decode += decode_dur;
        total_resize += resize_dur;
        successful_runs += 1;

        println!(
            "{: <30} | {: >7}ms | {: >7}ms | {: >7}ms",
            path.file_name().unwrap().to_string_lossy(),
            decode_dur.as_millis(),
            resize_dur.as_millis(),
            (decode_dur + resize_dur).as_millis()
        );
    }

    if successful_runs > 0 {
        let avg_decode = total_decode.as_millis() / successful_runs as u128;
        let avg_resize = total_resize.as_millis() / successful_runs as u128;
        println!("{}", "-".repeat(70));
        println!("Average Decode: {}ms", avg_decode);
        println!("Average Resize: {}ms (Mitchell SIMD)", avg_resize);
        println!("Average Total : {}ms", avg_decode + avg_resize);
        println!("Estimated throughput: {:.2} images/sec", 1000.0 / ((avg_decode + avg_resize) as f64));
    }

    Ok(())
}
