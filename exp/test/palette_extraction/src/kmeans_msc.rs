use image::{imageops::FilterType, GenericImageView};
use kmeans_colors::get_kmeans_hamerly;
use minifb::{Key, Window, WindowOptions};
use palette::{FromColor, Lab, Srgb};

const BANDWIDTH: f32 = 16.0;
const WEIGHT_BIAS: f32 = 2.0; 
const CONVERGENCE_EPS: f32 = 0.01;
const MAX_MSC_ITER: usize = 100;

#[derive(Clone, Copy, Debug)]
struct WeightedPoint {
    lab: Lab,
    weight: f32,
    original_ratio: f32,
}

fn run_weighted_msc(seeds: &[WeightedPoint]) -> (Vec<WeightedPoint>, Vec<usize>) {
    let mut converged_positions = Vec::with_capacity(seeds.len());
    let bw_sq = BANDWIDTH * BANDWIDTH;

    for seed in seeds {
        let mut current_pos = seed.lab;
        for _ in 0..MAX_MSC_ITER {
            let (mut sum_l, mut sum_a, mut sum_b, mut total_influence) = (0.0, 0.0, 0.0, 0.0);
            for neighbor in seeds {
                let dist_sq = (current_pos.l - neighbor.lab.l).powi(2)
                    + (current_pos.a - neighbor.lab.a).powi(2)
                    + (current_pos.b - neighbor.lab.b).powi(2);

                let influence = neighbor.weight * (-(dist_sq / (2.0 * bw_sq))).exp();
                sum_l += neighbor.lab.l * influence;
                sum_a += neighbor.lab.a * influence;
                sum_b += neighbor.lab.b * influence;
                total_influence += influence;
            }
            if total_influence > 0.0 {
                let next_pos = Lab::new(sum_l / total_influence, sum_a / total_influence, sum_b / total_influence);
                let shift_sq = (next_pos.l - current_pos.l).powi(2) + (next_pos.a - current_pos.a).powi(2) + (next_pos.b - current_pos.b).powi(2);
                current_pos = next_pos;
                if shift_sq < CONVERGENCE_EPS { break; }
            } else { break; }
        }
        converged_positions.push(current_pos);
    }

    let mut palette: Vec<WeightedPoint> = Vec::new();
    let mut seed_to_palette_map = vec![0; seeds.len()];
    let merge_threshold_sq = 4.0 * 4.0;

    for (seed_idx, pos) in converged_positions.into_iter().enumerate() {
        let mut mode_idx = None;
        for (existing_idx, mode) in palette.iter().enumerate() {
            let d_sq = (pos.l - mode.lab.l).powi(2) + (pos.a - mode.lab.a).powi(2) + (pos.b - mode.lab.b).powi(2);
            if d_sq < merge_threshold_sq {
                mode_idx = Some(existing_idx);
                break;
            }
        }
        if let Some(idx) = mode_idx {
            palette[idx].original_ratio += seeds[seed_idx].original_ratio;
            seed_to_palette_map[seed_idx] = idx;
        } else {
            seed_to_palette_map[seed_idx] = palette.len();
            palette.push(WeightedPoint { lab: pos, weight: 0.0, original_ratio: seeds[seed_idx].original_ratio });
        }
    }
    (palette, seed_to_palette_map)
}

pub fn run_hybrid_msc(path: &str) {
    let img = image::open(path).expect("Failed to open image");
    let start = std::time::Instant::now();

    let img_small = img.resize_exact(256, 256, FilterType::Nearest);
    let mut pixels = Vec::with_capacity(256 * 256);
    for (_, _, p) in img_small.pixels() {
        pixels.push(Lab::from_color(Srgb::new(p[0] as f32 / 255.0, p[1] as f32 / 255.0, p[2] as f32 / 255.0)));
    }

    let result = get_kmeans_hamerly(32, 20, 0.005, false, &pixels, 42);
    let mut counts = vec![0_usize; 32];
    for &idx in &result.indices { counts[idx as usize] += 1; }
    let total_px = result.indices.len() as f32;

    let compressed_points: Vec<WeightedPoint> = result.centroids.iter().enumerate()
        .map(|(i, &lab)| {
            let ratio = counts[i] as f32 / total_px;
            WeightedPoint { lab, weight: ratio.powf(WEIGHT_BIAS), original_ratio: ratio }
        })
        .collect();

    let (palette, seed_to_mode) = run_weighted_msc(&compressed_points);
    
    println!("Hybrid Process took: {:?}", start.elapsed());
    println!("Bias: {:.1}, MSC Palette Size: {}", WEIGHT_BIAS, palette.len());

    let mut sorted_palette = palette.clone();
    sorted_palette.sort_by(|a, b| b.original_ratio.partial_cmp(&a.original_ratio).unwrap());

    println!("\nFinal Hybrid Palette:");
    for (i, p) in sorted_palette.iter().enumerate() {
        println!("  {}: {} | Ratio: {:.4}", i + 1, lab_to_hex(p.lab), p.original_ratio);
    }

    let mut buffer = vec![0u32; 1024 * 1024];
    let draw_block = |buf: &mut Vec<u32>, px_x: usize, px_y: usize, quad_x: usize, quad_y: usize, color: u32| {
        let start_x = (quad_x * 512) + (px_x * 2);
        let start_y = (quad_y * 512) + (px_y * 2);
        for dy in 0..2 { for dx in 0..2 { buf[(start_y + dy) * 1024 + (start_x + dx)] = color; } }
    };

    for y in 0..256 {
        for x in 0..256 {
            let idx = y * 256 + x;
            let p = img_small.get_pixel(x as u32, y as u32);
            let orig = ((p[0] as u32) << 16) | ((p[1] as u32) << 8) | (p[2] as u32);
            let k32 = lab_to_u32(result.centroids[result.indices[idx] as usize]);
            let msc = lab_to_u32(palette[seed_to_mode[result.indices[idx] as usize]].lab);

            draw_block(&mut buffer, x, y, 0, 0, orig);
            draw_block(&mut buffer, x, y, 1, 0, k32);
            draw_block(&mut buffer, x, y, 0, 1, msc);
        }
    }

    let mut curr_ratio_px = 0;
    for p in &sorted_palette {
        let count = (p.original_ratio * 256.0 * 256.0).round() as usize;
        let color = lab_to_u32(p.lab);
        for _ in 0..count {
            if curr_ratio_px >= 256 * 256 { break; }
            draw_block(&mut buffer, curr_ratio_px % 256, curr_ratio_px / 256, 1, 1, color);
            curr_ratio_px += 1;
        }
    }

    let mut window = Window::new("Hybrid (Orig | K32 | MSC | Ratio)", 1024, 1024, WindowOptions::default()).unwrap();
    while window.is_open() && !window.is_key_down(Key::Escape) {
        window.update_with_buffer(&buffer, 1024, 1024).unwrap();
    }
}

fn lab_to_u32(lab: Lab) -> u32 {
    let srgb = Srgb::from_color(lab);
    let r = (srgb.red.clamp(0.0, 1.0) * 255.0) as u32;
    let g = (srgb.green.clamp(0.0, 1.0) * 255.0) as u32;
    let b = (srgb.blue.clamp(0.0, 1.0) * 255.0) as u32;
    (r << 16) | (g << 8) | b
}

fn lab_to_hex(lab: Lab) -> String {
    let srgb = Srgb::from_color(lab);
    format!("#{:02X}{:02X}{:02X}", (srgb.red.clamp(0.0, 1.0) * 255.0).round() as u8, (srgb.green.clamp(0.0, 1.0) * 255.0).round() as u8, (srgb.blue.clamp(0.0, 1.0) * 255.0).round() as u8)
}
