use anyhow::Result;
use discogs_rs::{Auth, DiscogsClient};

fn build_client() -> Result<DiscogsClient> {
    let token = std::env::var("DISCOGS_TOKEN").unwrap_or_default();
    let mut builder = DiscogsClient::with_default_user_agent();
    if !token.is_empty() {
        builder = builder.auth(Auth::UserToken { token });
    }
    Ok(builder.build()?)
}

pub async fn fetch_discogs_master(id: u64) -> Result<discogs_rs::Master> {
    let client = build_client()?;
    let res = client.database().get_master(id).await?;
    Ok(res.data)
}

pub async fn fetch_discogs_master_from_release(id: u64) -> Result<u64> {
    let client = build_client()?;
    let res = client.database().get_release(id, None).await?;
    if let Some(master_id) = res.data.master_id {
        Ok(master_id)
    } else {
        anyhow::bail!("Release {id} does not have a master release");
    }
}

pub async fn download_discogs_cover(url: &str, dest: &std::path::Path) -> Result<()> {
    let token = std::env::var("DISCOGS_TOKEN").unwrap_or_default();
    let client = reqwest::Client::builder()
        .user_agent("Vellum/0.1.0")
        .build()?;
    let mut req = client.get(url);
    if !token.is_empty() {
        req = req.header("Authorization", format!("Discogs token={token}"));
    }
    let res = req.send().await?;
    if !res.status().is_success() {
        anyhow::bail!("Failed to download cover: {}", res.status());
    }
    let bytes = res.bytes().await?;
    std::fs::write(dest, bytes)?;
    Ok(())
}

pub fn format_artist_credits(artists: &[discogs_rs::ArtistCredit]) -> String {
    let mut out = String::new();
    for artist in artists {
        let name = clean_artist_name(&artist.name);
        let join = artist.join.as_deref().unwrap_or("");
        out.push_str(&name);
        if !join.is_empty() {
            out.push_str(join);
        } else {
            out.push_str(", ");
        }
    }
    out.trim_end_matches(", ").to_string()
}

fn clean_artist_name(name: &str) -> String {
    let re = regex::Regex::new(r" \(\d+\)$").unwrap();
    re.replace(name, "").to_string()
}

pub fn parse_discogs_position(
    pos: &str,
    disc_counter: &mut u32,
    track_counter: &mut u32,
) -> (u32, u32) {
    if pos.contains('-') {
        let parts: Vec<&str> = pos.split('-').collect();
        let d = parts[0].parse().unwrap_or(1);
        let t = parts[1].parse().unwrap_or(1);
        *disc_counter = d;
        *track_counter = t;
        (d, t)
    } else if let Ok(t) = pos.parse::<u32>() {
        *track_counter = t;
        (*disc_counter, t)
    } else {
        *track_counter += 1;
        (*disc_counter, *track_counter)
    }
}
