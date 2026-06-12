use anyhow::Result;
use scraper::{Html, Selector};

pub struct Scraper {
    client: reqwest::Client,
}

impl Scraper {
    pub fn new() -> Result<Self> {
        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .timeout(std::time::Duration::from_secs(10))
            .build()?;
        Ok(Self { client })
    }

    pub async fn fetch_lyrics(&self, url: &str) -> Result<String> {
        let html = self.client.get(url).send().await?.text().await?;

        let html_with_newlines = html
            .replace("<br>", "\n")
            .replace("<br/>", "\n")
            .replace("<br />", "\n")
            .replace("<BR>", "\n")
            .replace("<BR/>", "\n")
            .replace("<BR />", "\n");

        let document = Html::parse_document(&html_with_newlines);
        let selector = Selector::parse("div[data-lyrics-container='true'], div.lyrics, div[class^='Lyrics__Container']").unwrap();

        let mut extracted_parts = Vec::new();
        for element in document.select(&selector) {
            let part_text = element.text().collect::<Vec<_>>().join("");
            extracted_parts.push(part_text);
        }

        if extracted_parts.is_empty() {
            anyhow::bail!("No lyrics container found in HTML");
        }

        let mut all_lyrics = Vec::new();
        for part in extracted_parts {
            for line in part.lines() {
                all_lyrics.push(line.to_string());
            }
        }

        Ok(Self::clean_lyrics(&all_lyrics))
    }

    fn clean_lyrics(lyrics_lines: &[String]) -> String {
        let mut filtered = Vec::new();
        let mut lines = lyrics_lines.iter();

        if let Some(first) = lines.next().filter(|f| !f.contains("Contributors")) {
            let trimmed = first.trim();
            if !(trimmed.starts_with('[') && trimmed.ends_with(']')) {
                filtered.push(trimmed.to_string());
            }
        }

        for line in lines {
            let trimmed = line.trim();
            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                continue;
            }
            filtered.push(trimmed.to_string());
        }

        let mut cleaned = filtered.join("\n");
        cleaned = cleaned.replace("(\n", "(").replace("\n)", ")");

        let mut collapsed = String::new();
        let mut consecutive_newlines = 0;
        for c in cleaned.chars() {
            if c == '\n' {
                consecutive_newlines += 1;
                if consecutive_newlines <= 2 {
                    collapsed.push(c);
                }
            } else {
                consecutive_newlines = 0;
                collapsed.push(c);
            }
        }

        let mut final_text = collapsed.trim().to_string();
        if final_text.ends_with("Embed") {
            final_text.truncate(final_text.len() - 5);
            while final_text.ends_with(|c: char| c.is_ascii_digit()) {
                final_text.pop();
            }
        }

        final_text.trim().to_string()
    }
}

pub fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| {
            if ['<', '>', ':', '"', '/', '\\', '|', '?', '*'].contains(&c) {
                '_'
            } else {
                c
            }
        })
        .collect()
}
