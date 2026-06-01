use serde::Deserialize;
use std::fs;

#[derive(Deserialize)]
pub struct Config {
    pub settings: Settings,
    pub shader: Option<ShaderDefaults>,
}

#[derive(Deserialize)]
pub struct Settings {
    pub image: String,
}

#[derive(Deserialize)]
pub struct ShaderDefaults {
    pub speed: Option<f32>,
    pub zoom: Option<f32>,
    pub blur: Option<f32>,
    pub edge_blur: Option<f32>,
    pub grain: Option<f32>,
    pub equalize: Option<f32>,
    pub chroma_bias: Option<f32>,
    pub mono_bias: Option<f32>,
}

pub fn load_config() -> Option<Config> {
    let content = fs::read_to_string("config.toml").ok()?;
    toml::from_str(&content).ok()
}
