#!/usr/bin/env -S cargo +nightly -Zscript
//! Check for Nix Docker image updates and create PR if needed
//!
//! This script:
//! - Compares current NIX_IMAGE version with latest Nix release
//! - Verifies new image exists on Docker Hub
//! - Creates a branch and PR with the update if available
//!
//! ```cargo
//! [dependencies]
//! reqwest = { version = "0.11", features = ["json"] }
//! serde = { version = "1.0", features = ["derive"] }
//! serde_json = "1.0"
//! tokio = { version = "1.0", features = ["full"] }
//! anyhow = "1.0"
//! regex = "1.0"
//! octocrab = "0.41"
//! ```

use anyhow::{Context, Result, anyhow};
use octocrab::Octocrab;
use regex::Regex;
use std::env;
use std::process::{Command, Stdio};

#[derive(serde::Deserialize)]
struct DockerHubTag {
    name: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("🐳 Checking for Nix image updates...");

    let nix_image = env::var("NIX_IMAGE").context("NIX_IMAGE environment variable not set")?;

    println!("Current Nix image: {}", nix_image);

    // Extract current version from NIX_IMAGE
    let version_regex = Regex::new(r"nixos/nix:(\d+\.\d+\.\d+)")?;
    let current_version = version_regex
        .captures(&nix_image)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str())
        .ok_or_else(|| anyhow!("Could not extract version from NIX_IMAGE: {}", nix_image))?;

    println!("Current version: {}", current_version);

    // Get latest Nix release from GitHub API using octocrab
    let github_token =
        env::var("GITHUB_TOKEN").context("GITHUB_TOKEN environment variable not set")?;
    let octocrab = Octocrab::builder()
        .personal_token(github_token)
        .build()
        .context("Failed to create GitHub client")?;

    let latest_release = octocrab
        .repos("NixOS", "nix")
        .releases()
        .get_latest()
        .await
        .context("Failed to fetch latest Nix release")?;

    let latest_version = latest_release.tag_name;
    println!("Latest version: {}", latest_version);

    if current_version == latest_version {
        println!("📋 Nix image is up to date: {}", current_version);
        return Ok(());
    }

    println!(
        "🆙 Nix image update available: {} → {}",
        current_version, latest_version
    );

    // Check if the Docker image exists on Docker Hub
    let new_image = format!("nixos/nix:{}", latest_version);
    let dockerhub_url = format!(
        "https://registry.hub.docker.com/v2/repositories/nixos/nix/tags/{}",
        latest_version
    );

    let dockerhub_response = client
        .get(&dockerhub_url)
        .send()
        .await
        .context("Failed to check Docker Hub for new image")?;

    if !dockerhub_response.status().is_success() {
        return Err(anyhow!(
            "❌ New Nix image not found on Docker Hub: {}",
            new_image
        ));
    }

    println!("✅ New Nix image exists on Docker Hub: {}", new_image);

    // Create branch and update
    let branch_name = format!("maintenance/nix-image-update-{}", latest_version);

    // Create branch
    run_command(&["git", "checkout", "-b", &branch_name]).context("Failed to create branch")?;

    // Update NIX_IMAGE in .gitlab-ci.yml
    let sed_pattern = format!(r#"s|NIX_IMAGE: "nixos/nix:.*"|NIX_IMAGE: "{}"|"#, new_image);
    run_command(&["sed", "-i", &sed_pattern, ".gitlab-ci.yml"])
        .context("Failed to update .gitlab-ci.yml")?;

    // Commit changes
    run_command(&["git", "add", ".gitlab-ci.yml"]).context("Failed to stage changes")?;

    let commit_message = format!(
        "chore: update Nix image to {}\n\nAutomated monthly update of Nix Docker image.\n\nChanges:\n- Updated NIX_IMAGE from nixos/nix:{} to {}\n- This affects nix_build_cache and flake_update jobs",
        latest_version, current_version, new_image
    );

    run_command(&["git", "commit", "-m", &commit_message]).context("Failed to commit changes")?;

    // Push branch
    run_command(&["git", "push", "origin", &branch_name]).context("Failed to push branch")?;

    // Create PR using octocrab
    let github_repo =
        env::var("GITHUB_REPO").context("GITHUB_REPO environment variable not set")?;

    // Parse owner/repo from GITHUB_REPO
    let repo_parts: Vec<&str> = github_repo.split('/').collect();
    if repo_parts.len() != 2 {
        return Err(anyhow!(
            "Invalid GITHUB_REPO format. Expected 'owner/repo', got: {}",
            github_repo
        ));
    }
    let (owner, repo) = (repo_parts[0], repo_parts[1]);

    let pr_title = format!("chore: update Nix image to {}", latest_version);
    let pr_body = format!(
        r#"## 🐳 Monthly Nix Image Update

This is an automated pull request to update the Nix Docker image.

### Changes
- Updated `NIX_IMAGE` variable from `nixos/nix:{}` to `nixos/nix:{}`
- Affects the following jobs:
  - `nix_build_cache`
  - `flake_update`

### Testing Required
- [ ] Verify `nix_build_cache` job works with new image
- [ ] Verify `flake_update` job works with new image
- [ ] Check that Nix functionality is preserved"#,
        current_version, latest_version, latest_version, latest_version
    );

    let _pr = octocrab
        .pulls(owner, repo)
        .create(pr_title, branch_name, "main")
        .body(pr_body)
        .send()
        .await
        .context("Failed to create pull request")?;

    println!("✅ Pull request created successfully");
    Ok(())
}

fn run_command(args: &[&str]) -> Result<()> {
    let output = Command::new(args[0])
        .args(&args[1..])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .context(format!("Failed to execute command: {:?}", args))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("Command failed: {:?}\nError: {}", args, stderr));
    }

    Ok(())
}
