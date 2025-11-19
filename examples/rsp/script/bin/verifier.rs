use anyhow::Result;
use clap::Parser;
use std::fs;
use std::time::Instant;
use tracing::{error, info};

use sp1_primitives::io::SP1PublicValues;
use sp1_prover::{
    components::CpuProverComponents, CoreSC, RecursionInput, SP1Prover, SP1VerifyingKey,
};
use sp1_sdk::utils;
use sp1_stark::{ShardProof, StarkVerifyingKey};
use srt::serialization::read_serialize_proof;

#[derive(Parser, Debug)]
struct Args {
    /// Path to the guest program verification key file
    #[arg(long)]
    guest_program_vk_path: String,

    /// Path to the final proof file
    #[arg(long)]
    final_proof_path: String,

    /// Path to the final verification key file
    #[arg(long)]
    final_vk_path: String,

    /// Path to the public values file
    #[arg(long)]
    public_values_path: String,
}

fn main() -> Result<()> {
    // Initialize the logger.
    utils::setup_logger();

    if let Err(e) = run() {
        error!("❌ Verification failed: {}", e);
        std::process::exit(1);
    }

    Ok(())
}

fn run() -> Result<()> {
    // Parse command line arguments
    let args = Args::parse();
    info!("📋 Parsed command line arguments");
    info!("  Guest program verification key path: {}", args.guest_program_vk_path);
    info!("  Final proof path: {}", args.final_proof_path);
    info!("  Final verification key path: {}", args.final_vk_path);
    info!("  Public values path: {}", args.public_values_path);

    info!("🔄 Starting file loading...");
    let file_loading_start = Instant::now();

    let guest_program_vk = read_serialize_proof::<StarkVerifyingKey<CoreSC>>(
        &args.guest_program_vk_path,
    )
    .map_err(|e| {
        anyhow::anyhow!(
            "Failed to read guest program verification key file '{}': {}",
            args.guest_program_vk_path,
            e
        )
    })?;
    let guest_program_vk = SP1VerifyingKey { vk: guest_program_vk };
    info!("✅ Successfully loaded guest program verification key");

    // Load finalproof from file
    let final_proof =
        read_serialize_proof::<ShardProof<CoreSC>>(&args.final_proof_path).map_err(|e| {
            anyhow::anyhow!("Failed to read final proof file '{}': {}", args.final_proof_path, e)
        })?;
    info!("✅ Successfully loaded final proof file");

    // Load final verification key from file
    let final_vk =
        read_serialize_proof::<StarkVerifyingKey<CoreSC>>(&args.final_vk_path).map_err(|e| {
            anyhow::anyhow!(
                "Failed to read final verification key file '{}': {}",
                args.final_vk_path,
                e
            )
        })?;
    info!("✅ Successfully loaded final verification key file");

    // Load public values from file
    let public_values_data = fs::read(&args.public_values_path).map_err(|e| {
        anyhow::anyhow!("Failed to read public values file '{}': {}", args.public_values_path, e)
    })?;
    let public_values = SP1PublicValues::from(&public_values_data);
    info!("✅ Successfully loaded public values file ({} bytes)", public_values_data.len());

    let file_loading_duration = file_loading_start.elapsed();
    info!("📁 File loading completed in {:?}", file_loading_duration);

    info!("🔍 Starting proof verification...");
    let verification_start = Instant::now();
    let input = RecursionInput::Single { vk: final_vk, proof: final_proof, is_first_shard: false };

    // Verify the proof
    SP1Prover::<DefaultProverComponents>::new()
        .verify_final_compressed(guest_program_vk, input, public_values)
        .map_err(|e| anyhow::anyhow!("Proof verification failed: {}", e))?;

    let verification_duration = verification_start.elapsed();
    info!("🎉 Proof verification completed successfully in {:?}", verification_duration);

    Ok(())
}
