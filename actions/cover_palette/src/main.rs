mod kmeans;
mod kmeansn;
mod kmeansnd;
mod kmeansnh;
mod kmeansnv;
mod mean_shift;

use anyhow::{Context, Result};
use image::imageops::FilterType;
use image::DynamicImage;
use palette::{FromColor, Oklab, Oklch, Srgb};
use serde::Deserialize;
use serde_json::Value;
use std::io::Read;
use std::path::PathBuf;

#[derive(Deserialize, Default)]
struct ScriptConfig {
    #[serde(rename = "type")]
    algo_type: Option<String>,
    sort: Option<String>,
    #[serde(default)]
    args: String,
    threshold: Option<f32>,
}

fn expand_path(path_str: &str) -> PathBuf {
    if path_str.starts_with('~') {
        if let Some(home) = dirs::home_dir() {
            if path_str == "~" {
                return home;
            }
            if let Some(stripped) = path_str.strip_prefix("~/") {
                return home.join(stripped);
            }
        }
    }
    PathBuf::from(path_str)
}

fn main() -> Result<()> {
    let mut stdin_data = String::new();
    std::io::stdin().read_to_string(&mut stdin_data)?;

    let payload: Value = serde_json::from_str(&stdin_data)?;

    let albums = payload.get(0).and_then(Value::as_array).context("Missing albums array")?;
    let config = payload.get(1).context("Missing config")?;

    let library_root_str = config
        .pointer("/storage/library_root")
        .and_then(Value::as_str)
        .context("Missing library_root in payload")?;

    let library_root = expand_path(library_root_str);

    let config_path = expand_path("~/dev/vellum/actions/cover_palette/config.toml");
    let script_config: ScriptConfig = if config_path.exists() {
        let content = std::fs::read_to_string(&config_path)?;
        toml::from_str(&content).unwrap_or_default()
    } else {
        ScriptConfig::default()
    };

    for album_lock in albums {
        let album_path_str = album_lock
            .pointer("/album/id")
            .and_then(Value::as_str)
            .unwrap_or("");

        let cover_path_str = album_lock
            .pointer("/album/covers/main/file/path")
            .and_then(Value::as_str)
            .unwrap_or("cover.jpg");

        let album_dir = library_root.join(album_path_str);
        let cover_path = album_dir.join(cover_path_str);

        if !cover_path.exists() {
            continue;
        }

        if let Ok(img) = image::open(&cover_path) {
            if let Some(palette) = process_image_to_palette(&img, &script_config) {
                let hex_colors: Vec<String> = palette
                    .into_iter()
                    .map(|(srgb, _)| {
                        let r_u8 = (srgb.red.clamp(0.0, 1.0) * 255.0).round() as u8;
                        let g_u8 = (srgb.green.clamp(0.0, 1.0) * 255.0).round() as u8;
                        let b_u8 = (srgb.blue.clamp(0.0, 1.0) * 255.0).round() as u8;
                        format!("#{r_u8:02X}{g_u8:02X}{b_u8:02X}")
                    })
                    .collect();

                let toml_content = format!(
                    "[album]\n\ncover_palette = [\n{}\n]\n",
                    hex_colors
                        .iter()
                        .map(|c| format!("  \"{c}\","))
                        .collect::<Vec<_>>()
                        .join("\n")
                );

                let out_path = album_dir.join("cover_palette.toml");
                let _ = std::fs::write(&out_path, toml_content);
            }
        }
    }

    Ok(())
}

fn process_image_to_palette(
    img: &DynamicImage,
    cfg: &ScriptConfig,
) -> Option<Vec<(Srgb, f32)>> {
    let algo_type = cfg.algo_type.as_deref().unwrap_or("kmeansnv");
    let sort_type = cfg.sort.as_deref().unwrap_or("gradient");
    let args = &cfg.args;

    let sample_dim = args.split(',')
        .find(|s| s.trim().starts_with("dim="))
        .and_then(|s| s.trim().strip_prefix("dim="))
        .and_then(|val| val.parse::<u32>().ok())
        .unwrap_or(512);

    let img_to_process = if sample_dim == 0 {
        img.clone()
    } else {
        img.resize_exact(sample_dim, sample_dim, FilterType::Nearest)
    };

    let candidate_colors = match algo_type {
        "msc" => mean_shift::extract(&img_to_process, args),
        "kmeansn" => kmeansn::extract(&img_to_process, args),
        "kmeansnh" => kmeansnh::extract(&img_to_process, args),
        "kmeansnd" => kmeansnd::extract(&img_to_process, args),
        "kmeansnv" => kmeansnv::extract(&img_to_process, args),
        _ => kmeans::extract(&img_to_process, args),
    };

    if candidate_colors.is_empty() {
        return None;
    }

    let threshold_val = cfg.threshold.unwrap_or(0.001);
    let mut palette = calculate_palette_ratios(&img_to_process, candidate_colors, threshold_val);
    sort_palette(&mut palette, sort_type);

    Some(palette)
}

fn calculate_palette_ratios(
    img_to_process: &DynamicImage,
    candidate_colors: Vec<Srgb>,
    threshold_val: f32,
) -> Vec<(Srgb, f32)> {
    let oklab_centers: Vec<Oklab> = candidate_colors.iter().map(|&c| Oklab::from_color(c)).collect();
    let mut counts = vec![0usize; oklab_centers.len()];

    for p in img_to_process.to_rgb8().pixels() {
        let pixel_oklab = Oklab::from_color(Srgb::new(
            f32::from(p[0]) / 255.0,
            f32::from(p[1]) / 255.0,
            f32::from(p[2]) / 255.0,
        ));
        let mut best_idx = 0;
        let mut min_dist_sq = f32::MAX;
        for (i, center) in oklab_centers.iter().enumerate() {
            let dist_sq = (pixel_oklab.b - center.b).mul_add(pixel_oklab.b - center.b, (pixel_oklab.a - center.a).mul_add(pixel_oklab.a - center.a, (pixel_oklab.l - center.l).powi(2)));
            if dist_sq < min_dist_sq {
                min_dist_sq = dist_sq;
                best_idx = i;
            }
        }
        counts[best_idx] += 1;
    }

    let total_pixels = counts.iter().sum::<usize>() as f32;
    let mut palette: Vec<(Srgb, f32)> = candidate_colors.into_iter().zip(counts).filter_map(|(color, count)| {
        let ratio = if total_pixels > 0.0 { count as f32 / total_pixels } else { 0.0 };
        if ratio > 0.0 { Some((color, ratio)) } else { None }
    }).collect();

    palette.retain(|&(_, ratio)| ratio >= threshold_val);

    let final_total: f32 = palette.iter().map(|(_, r)| r).sum();
    if final_total > 0.0 {
        for item in &mut palette {
            item.1 /= final_total;
        }
    }
    palette
}

fn sort_palette(palette: &mut Vec<(Srgb, f32)>, sort_type: &str) {
    match sort_type {
        "L" => palette.sort_by(|a, b| {
            let l_a = Oklch::from_color(a.0).l;
            let l_b = Oklch::from_color(b.0).l;
            l_b.partial_cmp(&l_a).unwrap_or(std::cmp::Ordering::Equal)
        }),
        "C" => palette.sort_by(|a, b| {
            let c_a = Oklch::from_color(a.0).chroma;
            let c_b = Oklch::from_color(b.0).chroma;
            c_b.partial_cmp(&c_a).unwrap_or(std::cmp::Ordering::Equal)
        }),
        "H" => palette.sort_by(|a, b| {
            let h_a = Oklch::from_color(a.0).hue.into_raw_degrees();
            let h_b = Oklch::from_color(b.0).hue.into_raw_degrees();
            h_a.partial_cmp(&h_b).unwrap_or(std::cmp::Ordering::Equal)
        }),
        "LC" => palette.sort_by(|a, b| {
            let oklch_a = Oklch::from_color(a.0);
            let oklch_b = Oklch::from_color(b.0);
            let val_a = oklch_a.l * oklch_a.chroma;
            let val_b = oklch_b.l * oklch_b.chroma;
            val_b.partial_cmp(&val_a).unwrap_or(std::cmp::Ordering::Equal)
        }),
        "gradient" => sort_palette_gradient(palette),
        "original" => {},
        _ => palette.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)),
    }
}

fn sort_palette_gradient(palette: &mut Vec<(Srgb, f32)>) {
    if palette.is_empty() { return; }
    let mut pool: Vec<(Oklab, Srgb, f32)> = palette.iter()
        .map(|&(srgb, ratio)| (Oklab::from_color(srgb), srgb, ratio))
        .collect();

    let mut sorted = Vec::with_capacity(pool.len());
    
    let start_idx = pool.iter().enumerate()
        .max_by(|(_, (ok_a, _, _)), (_, (ok_b, _, _))| {
            ok_a.l.partial_cmp(&ok_b.l).unwrap_or(std::cmp::Ordering::Equal)
        })
        .map_or(0, |(i, _)| i);

    let first = pool.remove(start_idx);
    let mut current_ok = first.0;
    sorted.push((first.1, first.2));

    let end_node_idx = if pool.is_empty() {
        None
    } else {
        pool.iter().enumerate()
            .min_by(|(_, (ok_a, _, _)), (_, (ok_b, _, _))| {
                ok_a.l.partial_cmp(&ok_b.l).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(i, _)| i)
    };

    let end_node = end_node_idx.map(|idx| pool.remove(idx));

    while !pool.is_empty() {
        let next_idx = pool.iter().enumerate()
            .min_by(|(_, (ok_a, _, _)), (_, (ok_b, _, _))| {
                let dist_a = (ok_a.b - current_ok.b).mul_add(ok_a.b - current_ok.b, (ok_a.a - current_ok.a).mul_add(ok_a.a - current_ok.a, (ok_a.l - current_ok.l).powi(2)));
                let dist_b = (ok_b.b - current_ok.b).mul_add(ok_b.b - current_ok.b, (ok_b.a - current_ok.a).mul_add(ok_b.a - current_ok.a, (ok_b.l - current_ok.l).powi(2)));
                dist_a.partial_cmp(&dist_b).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map_or(0, |(i, _)| i);
        
        let next = pool.remove(next_idx);
        current_ok = next.0;
        sorted.push((next.1, next.2));
    }

    if let Some(node) = end_node {
        sorted.push((node.1, node.2));
    }

    *palette = sorted;
}
