use image::DynamicImage;
use palette::{FromColor, Oklab, Srgb};

pub fn extract(img: &DynamicImage, args: &str) -> Vec<Srgb> {
    let bw = args.split(',')
        .find(|s| s.trim().starts_with("bw="))
        .and_then(|s| s.trim().strip_prefix("bw="))
        .and_then(|val| val.parse::<f32>().ok())
        .unwrap_or(0.12);
        
    let eps = args.split(',')
        .find(|s| s.trim().starts_with("eps="))
        .and_then(|s| s.trim().strip_prefix("eps="))
        .and_then(|val| val.parse::<f32>().ok())
        .unwrap_or(0.0001);
        
    let max_iter = args.split(',')
        .find(|s| s.trim().starts_with("iter="))
        .and_then(|s| s.trim().strip_prefix("iter="))
        .and_then(|val| val.parse::<usize>().ok())
        .unwrap_or(20);

    let merge_threshold = args.split(',')
        .find(|s| s.trim().starts_with("mt="))
        .and_then(|s| s.trim().strip_prefix("mt="))
        .and_then(|val| val.parse::<f32>().ok())
        .unwrap_or(0.10);

    let chroma_gravity = args.split(',')
        .find(|s| s.trim().starts_with("cg="))
        .and_then(|s| s.trim().strip_prefix("cg="))
        .and_then(|val| val.parse::<f32>().ok())
        .unwrap_or(0.0);

    let k = args.split(',')
        .find(|s| s.trim().starts_with("k="))
        .and_then(|s| s.trim().strip_prefix("k="))
        .and_then(|val| val.parse::<usize>().ok())
        .unwrap_or(0);

    let img_small = img.resize_exact(64, 64, image::imageops::FilterType::Nearest);
    let sample_pixels: Vec<Oklab> = img_small.to_rgb8().pixels().map(|p| {
        Oklab::from_color(Srgb::new(
            f32::from(p[0]) / 255.0,
            f32::from(p[1]) / 255.0,
            f32::from(p[2]) / 255.0,
        ))
    }).collect();
    
    let bw_sq = bw * bw;
    let mut converged = Vec::with_capacity(sample_pixels.len());
    
    for &seed in &sample_pixels {
        let mut current = seed;
        for _ in 0..max_iter {
            let mut sum_l = 0.0;
            let mut sum_a = 0.0;
            let mut sum_b = 0.0;
            let mut total_weight = 0.0;
            
            for &p in &sample_pixels {
                let dist_sq = (current.b - p.b).mul_add(current.b - p.b, (current.a - p.a).mul_add(current.a - p.a, (current.l - p.l).powi(2)));
                if dist_sq < bw_sq {
                    let chroma = p.a.hypot(p.b);
                    let weight = 1.0 + (chroma_gravity * chroma);
                    
                    sum_l += p.l * weight;
                    sum_a += p.a * weight;
                    sum_b += p.b * weight;
                    total_weight += weight;
                }
            }
            
            if total_weight > 0.0 {
                let next = Oklab::new(sum_l / total_weight, sum_a / total_weight, sum_b / total_weight);
                let shift_sq = (next.b - current.b).mul_add(next.b - current.b, (next.a - current.a).mul_add(next.a - current.a, (next.l - current.l).powi(2)));
                current = next;
                if shift_sq < eps {
                    break;
                }
            } else {
                break;
            }
        }
        converged.push(current);
    }
    
    let merge_threshold_sq = merge_threshold * merge_threshold;
    let mut centers: Vec<Oklab> = Vec::new();
    
    for pos in converged {
        let mut found = false;
        for center in &centers {
            let dist_sq = (pos.b - center.b).mul_add(pos.b - center.b, (pos.a - center.a).mul_add(pos.a - center.a, (pos.l - center.l).powi(2)));
            if dist_sq < merge_threshold_sq {
                found = true;
                break;
            }
        }
        if !found {
            centers.push(pos);
        }
    }
    
    centers.sort_by(|a, b| {
        let chroma_a = a.a.hypot(a.b);
        let chroma_b = b.a.hypot(b.b);
        chroma_b.partial_cmp(&chroma_a).unwrap_or(std::cmp::Ordering::Equal)
    });

    if k > 0 && centers.len() > k {
        centers.truncate(k);
    }

    centers.into_iter().map(Srgb::from_color).collect()
}
