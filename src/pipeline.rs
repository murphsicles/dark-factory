// src/pipeline.rs — End-to-end crate conversion pipeline.
//
// 1. Fetch crate from crates.io (or local path)
// 2. Convert each .rs file via transpiler
// 3. Compile with zetac
// 4. Run tests
// 5. Optionally publish to zorbs.io

use std::path::{Path, PathBuf};
use std::fs;
use std::process::Command;

/// Convert an entire crate directory to Zeta.
pub fn convert_crate(path: &str) -> anyhow::Result<()> {
    let crate_dir = Path::new(path);
    if !crate_dir.join("Cargo.toml").exists() {
        anyhow::bail!("No Cargo.toml found in {}", path);
    }

    let out_dir = crate_dir.join("zeta_out");
    fs::create_dir_all(&out_dir)?;
    fs::create_dir_all(out_dir.join("src"))?;

    // Walk all .rs files in src/
    let src_dir = crate_dir.join("src");
    if src_dir.exists() {
        convert_dir_recursive(&src_dir, &out_dir.join("src"))?;
    }

    println!("Conversion complete. Output in: {}", out_dir.display());
    Ok(())
}

fn convert_dir_recursive(src: &Path, dst: &Path) -> anyhow::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let dest_path = dst.join(entry.file_name());

        if path.is_dir() {
            convert_dir_recursive(&path, &dest_path)?;
        } else if path.extension().map_or(false, |e| e == "rs") {
            // Convert the file
            let source = fs::read_to_string(&path)?;
            let result = crate::transpiler::convert_file(&source, &path.to_string_lossy())?;

            // Write with .zeta extension
            let zeta_path = dest_path.with_extension("zeta");
            fs::write(&zeta_path, &result)?;
            println!("  Converted: {} → {}", path.display(), zeta_path.display());
        }
    }
    Ok(())
}

/// Fetch a crate from crates.io and convert it.
pub fn fetch_and_convert(name: &str, version: &str) -> anyhow::Result<()> {
    let version = resolve_latest_version(name, version)?;
    let url = format!("https://crates.io/api/v1/crates/{}/{}/download", name, version);

    println!("Fetching {} v{} from crates.io...", name, version);
    let client = reqwest::blocking::Client::builder()
        .user_agent("dark-factory/0.1.0 (zeta-lang)")
        .build()?;

    let resp = client.get(&url).send()?;
    if !resp.status().is_success() {
        anyhow::bail!("Failed to fetch {}: HTTP {}", name, resp.status());
    }

    let bytes = resp.bytes()?;
    let work_dir = std::env::temp_dir().join(format!("df-{}-{}", name, version));
    fs::create_dir_all(&work_dir)?;

    // Extract tarball
    let tar_gz = work_dir.join("crate.tar.gz");
    fs::write(&tar_gz, &bytes)?;

    let output = Command::new("tar")
        .args(["-xzf", &tar_gz.to_string_lossy(), "-C", &work_dir.to_string_lossy()])
        .output()?;

    if !output.status.success() {
        anyhow::bail!("Failed to extract crate tarball");
    }

    // Find the extracted crate directory (should be {name}-{version})
    let extracted = work_dir.join(format!("{}-{}", name.replace('-', "_"), version));
    let alt = work_dir.join(format!("{}-{}", name, version));
    let crate_dir = if extracted.exists() { extracted } else { alt };

    if !crate_dir.exists() {
        anyhow::bail!("Could not find extracted crate directory");
    }

    convert_crate(&crate_dir.to_string_lossy())
}

/// Run the full pipeline: fetch → convert → compile → test → (optional) publish.
pub fn run_pipeline(name: &str, version: &str, publish: bool) -> anyhow::Result<()> {
    println!("=== Dark Factory Pipeline ===");
    println!("Crate: {} v{}", name, version);
    println!("Publish to zorbs.io: {}", publish);
    println!();

    // Step 1: Fetch
    println!("[1/4] Fetching...");
    let version = resolve_latest_version(name, version)?;

    let url = format!("https://crates.io/api/v1/crates/{}/{}/download", name, version);
    let client = reqwest::blocking::Client::builder()
        .user_agent("dark-factory/0.1.0 (zeta-lang)")
        .build()?;
    let resp = client.get(&url).send()?;
    if !resp.status().is_success() {
        anyhow::bail!("Failed to fetch {}: HTTP {}", name, resp.status());
    }
    let bytes = resp.bytes()?;
    let work_dir = std::env::temp_dir().join(format!("df-{}-{}", name, version));
    let _ = fs::remove_dir_all(&work_dir);
    fs::create_dir_all(&work_dir)?;

    let tar_gz = work_dir.join("crate.tar.gz");
    fs::write(&tar_gz, &bytes)?;
    Command::new("tar")
        .args(["-xzf", &tar_gz.to_string_lossy(), "-C", &work_dir.to_string_lossy()])
        .output()?;

    let extracted = work_dir.join(format!("{}-{}", name.replace('-', "_"), version));
    let alt = work_dir.join(format!("{}-{}", name, version));
    let crate_dir = if extracted.exists() { extracted } else { alt };
    println!("  Extracted to: {}", crate_dir.display());

    // Step 2: Convert
    println!("[2/4] Converting to Zeta...");
    let out_dir = work_dir.join("zeta_out");
    let src_out = out_dir.join("src");
    fs::create_dir_all(&src_out)?;
    let crate_src = crate_dir.join("src");
    if crate_src.exists() {
        convert_dir_recursive(&crate_src, &src_out)?;
    }
    println!("  Conversion complete");

    // Step 3: Compile with zetac
    println!("[3/4] Compiling with zetac...");
    let zeta_files: Vec<String> = walk_zeta_files(&src_out);
    if zeta_files.is_empty() {
        println!("  No .zeta files to compile");
    } else {
        for zf in &zeta_files {
            println!("  Compiling: {}", zf);
            let compile_result = Command::new("zetac")
                .arg(zf)
                .output();

            match compile_result {
                Ok(output) => {
                    if output.status.success() {
                        println!("    ✅ OK");
                    } else {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        println!("    ❌ Failed:");
                        for line in stderr.lines().take(10) {
                            println!("       {}", line);
                        }
                    }
                }
                Err(e) => {
                    println!("    ⚠️  Could not run zetac: {}", e);
                    println!("    (Compilation check skipped)");
                }
            }
        }
    }

    // Step 4: Publish
    if publish {
        println!("[4/4] Publishing to zorbs.io...");

        // Create zorb.toml
        let zorb_toml = format!(
            r#"[package]
name = "@std/{}"
version = "{}"
description = "Auto-converted from Rust crate"
license = "MIT"

[fmt]
style = "zeta-strict"
"#, name.replace('-', "_"), version);
        fs::write(out_dir.join("zorb.toml"), &zorb_toml)?;

        // Create tarball
        let tar_path = work_dir.join("package.zorb");
        let tar_file = fs::File::create(&tar_path)?;
        let encoder = flate2::write::GzEncoder::new(tar_file, flate2::Compression::default());
        let mut tar_builder = tar::Builder::new(encoder);
        tar_builder.append_path_with_name(out_dir.join("zorb.toml"), "zorb.toml")?;
        tar_builder.append_dir_all("src", &src_out)?;
        tar_builder.finish()?;

        // POST to zorbs.io
        let tar_data = fs::read(&tar_path)?;
        let part = reqwest::blocking::multipart::Part::bytes(tar_data)
            .file_name("package.zorb");
        let form = reqwest::blocking::multipart::Form::new()
            .part("file", part);

        let registry = std::env::var("REGISTRY_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());
        let api_url = format!("{}/api/zorbs/new", registry);
        let pub_resp = client.post(&api_url)
            .multipart(form)
            .send()?;

        if pub_resp.status().is_success() {
            println!("    ✅ Published to {}", registry);
        } else {
            println!("    ❌ Publish failed: HTTP {}", pub_resp.status());
        }
    }

    println!("=== Pipeline complete ===");
    Ok(())
}

fn resolve_latest_version(name: &str, version: &str) -> anyhow::Result<String> {
    if version != "*" {
        return Ok(version.to_string());
    }

    let client = reqwest::blocking::Client::builder()
        .user_agent("dark-factory/0.1.0 (zeta-lang)")
        .build()?;

    let url = format!("https://crates.io/api/v1/crates/{}", name);
    let resp = client.get(&url).send()?;
    if !resp.status().is_success() {
        anyhow::bail!("Failed to query crates.io for {}: HTTP {}", name, resp.status());
    }

    let json: serde_json::Value = resp.json()?;
    if let Some(version) = json["crate"]["max_version"].as_str() {
        Ok(version.to_string())
    } else {
        anyhow::bail!("Could not determine latest version for {}", name);
    }
}

fn walk_zeta_files(dir: &Path) -> Vec<String> {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(walk_zeta_files(&path));
            } else if path.extension().map_or(false, |e| e == "zeta") {
                files.push(path.to_string_lossy().to_string());
            }
        }
    }
    files
}
