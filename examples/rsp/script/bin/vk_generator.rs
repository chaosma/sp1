use anyhow::{anyhow, Result};
use std::fs::File;
use std::path::PathBuf;
use std::env;
use tracing::info;


use sp1_sdk::{utils, ProverClient};
use sp1_sdk::provers::serialize_primitives::SerializeProof;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize the logger.
    utils::setup_logger();
    let client = ProverClient::new();

    // Read ELF file path from environment variable (fallback to default)
    let elf_path = env::var("ELF_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/mnt/shared_ramdisk/srt/program/eth-block"));
    let program_bytes = std::fs::read(&elf_path)
        .map_err(|e| anyhow!("Failed to read ELF file from {}: {}", elf_path.display(), e))?;
    info!("Successfully read ELF file from: {:?}", elf_path);

    // Setup the proving key and verification key.
    let (_pk, vk) = client.setup(&program_bytes);

    // Read vk path from environment variable (fallback to default)
    let vk_path = env::var("VK_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/mnt/shared_ramdisk/srt/program/vk.bin"));
    let mut vk_file = File::create(&vk_path).map_err(|e| anyhow!("Failed to create file: {}", e))?;
    vk.vk.to_bytes(&mut vk_file).map_err(|e| anyhow!("Failed to serialize: {}", e))?;
    info!("Successfully serialized verification key to: {:?}", vk_path);

    Ok(())
}