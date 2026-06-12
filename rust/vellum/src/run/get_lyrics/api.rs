use anyhow::Result;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct SearchResponse {
    pub response: SearchResponseBody,
}

#[derive(Deserialize)]
pub struct SearchResponseBody {
    pub hits: Vec<Hit>,
}

#[derive(Deserialize)]
pub struct Hit {
    pub result: SongResult,
}

#[derive(Deserialize)]
pub struct SongResult {
    pub url: String,
}

pub struct GeniusApi {
    client: reqwest::Client,
    api_key: String,
}

impl GeniusApi {
    pub fn new(api_key: String) -> Result<Self> {
        let client = reqwest::Client::builder()
            .user_agent("Vellum/1.0")
            .build()?;
        Ok(Self { client, api_key })
    }

    pub async fn search(&self, query: &str) -> Result<Option<SongResult>> {
        let res = self
            .client
            .get("https://api.genius.com/search")
            .query(&[("q", query)])
            .bearer_auth(&self.api_key)
            .send()
            .await?
            .json::<SearchResponse>()
            .await?;

        Ok(res.response.hits.into_iter().next().map(|h| h.result))
    }
}
