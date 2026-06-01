use image::{imageops::FilterType, GenericImageView};
use kmeans_colors::get_kmeans_hamerly;
use palette::{FromColor, Lab, Srgb};
use rand::{SeedableRng, rngs::StdRng};
use rand_distr::{Distribution, Normal};

pub fn run_kmeans_wn(path: &str, noise_std_dev: Option<f32>, arg_str: &str) -> Vec<(Lab, f32)> {
    let k = parse_arg(arg_str, "k=", 10).clamp(1, 24);
    let noise_std_dev = noise_std_dev.unwrap_or(0.01);

    let start = std::time::Instant::now();
    let palette = extract_palette_wn(path, k, noise_std_dev);
    println!("K-Means WN (CIELAB) took: {:?}", start.elapsed());

    println!("\nPalette:");
    for (i, (lab, ratio)) in palette.iter().enumerate() {
        let hex = crate::kmeans::lab_to_hex(*lab);
        println!("  {}: {} | Ratio: {:.4}", i + 1, hex, ratio);
    }
    
    palette
}

pub fn extract_palette_wn(path: &str, k: usize, noise_std_dev: f32) -> Vec<(Lab, f32)> {
    let sample_dim = 512;
    let n_pixels = sample_dim * sample_dim;
    let max_iter = 20;
    let convergence = 0.000;
    let seed = 42;
    let discard_threshold = 0.0000_f32;

    let img = image::open(path).expect("Failed to open image");
    let img_small = img.resize_exact(sample_dim, sample_dim, FilterType::Nearest);
    let mut pixels: Vec<Lab> = Vec::with_capacity(n_pixels as usize);

    // Use a deterministic RNG to ensure reproducible metadata compilations
    let mut rng = StdRng::seed_from_u64(seed);
    let normal = Normal::new(0.0, noise_std_dev).unwrap();

    for (_, _, p) in img_small.pixels() {
        let noise = normal.sample(&mut rng);
        let mut srgb = Srgb::new(
            f32::from(p[0]) / 255.0,
            f32::from(p[1]) / 255.0,
            f32::from(p[2]) / 255.0,
        );
        
        srgb.red = (srgb.red + noise).clamp(0.0, 1.0);
        srgb.green = (srgb.green + noise).clamp(0.0, 1.0);
        srgb.blue = (srgb.blue + noise).clamp(0.0, 1.0);

        pixels.push(Lab::from_color(srgb));
    }

    let result = get_kmeans_hamerly(k, max_iter, convergence, false, &pixels, seed);
    let actual_k = result.centroids.len();
    let total_px = n_pixels as f32;

    let mut counts = vec![0_usize; actual_k];
    for &idx in &result.indices {
        counts[idx as usize] += 1;
    }

    let mut keep_indices = Vec::new();
    let mut discard_indices = Vec::new();

    for i in 0..actual_k {
        let ratio = counts[i] as f32 / total_px;
        if ratio >= discard_threshold {
            keep_indices.push(i);
        } else {
            discard_indices.push(i);
        }
    }

    if keep_indices.is_empty() {
        let max_idx = counts.iter().enumerate()
            .max_by_key(|&(_, count)| count)
            .map(|(i, _)| i)
            .unwrap_or(0);
        keep_indices.push(max_idx);
        discard_indices.retain(|&i| i != max_idx);
    }

    let mut final_counts = counts.clone();
    for &d_idx in &discard_indices {
        let d_lab = result.centroids[d_idx];
        let mut best_target = keep_indices[0];
        let mut min_dist_sq = f32::MAX;

        for &k_idx in &keep_indices {
            let k_lab = result.centroids[k_idx];
            let dist_sq = (d_lab.l - k_lab.l).powi(2) 
                        + (d_lab.a - k_lab.a).powi(2) 
                        + (d_lab.b - k_lab.b).powi(2);
            
            if dist_sq < min_dist_sq {
                min_dist_sq = dist_sq;
                best_target = k_idx;
            }
        }
        final_counts[best_target] += counts[d_idx];
    }

    let mut palette: Vec<(Lab, f32)> = keep_indices.iter()
        .map(|&i| (result.centroids[i], final_counts[i] as f32 / total_px))
        .collect();
    
    palette.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    palette
}

fn parse_arg<T: std::str::FromStr>(args: &str, key: &str, default: T) -> T {
    args.split(',')
        .find(|s| s.trim().starts_with(key))
        .and_then(|s| s.trim().strip_prefix(key))
        .and_then(|val| val.parse::<T>().ok())
        .unwrap_or(default)
}
