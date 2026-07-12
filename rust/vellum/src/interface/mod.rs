use anyhow::{Context, Result};
use libvellum::utils::expand_path;
use std::process::Stdio;

pub async fn execute(name: Option<String>) -> Result<()> {
    let raw_name = name.unwrap_or_else(|| "default".to_string());
    let name = raw_name.replace('-', "_");
    let config = libvellum::lua::ResolvedConfig::load().context("Failed to load config")?;

    let mut intf_cfg = config.interfaces
        .get(&name)
        .cloned()
        .unwrap_or_default();

    if name == "default" {
        intf_cfg.enable = true;
    }

    if !intf_cfg.enable {
        anyhow::bail!("Interface '{raw_name}' is not enabled in config.");
    }

    let dir_str = intf_cfg.directory.unwrap_or_else(|| format!("~/.local/share/vellum/interfaces/{raw_name}"));
    let dir_path = expand_path(&dir_str);

    let run_str = intf_cfg.run.unwrap_or_else(|| format!("{}/run.sh", dir_path.display()));
    let run_path = expand_path(&run_str);

    if !run_path.exists() {
        anyhow::bail!("Run script not found at {}", run_path.display());
    }

    let mut child = tokio::process::Command::new("sh")
        .arg(&run_path)
        .current_dir(&dir_path)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .context(format!("Failed to spawn interface script at {}", run_path.display()))?;

    tokio::select! {
        res = child.wait() => {
            res.context("Interface child process wait failed")?;
        }
        _ = tokio::signal::ctrl_c() => {
            let _ = child.wait().await;
        }
    }
    Ok(())
}
