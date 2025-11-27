use anyhow::{anyhow, bail, Context, Result};
use clap::Parser;
use sp1_primitives::io::SP1PublicValues;
use sp1_prover::{
    components::CpuProverComponents, CoreSC, SP1CoreProofData, SP1Prover, SP1VerifyingKey,
};
use sp1_sdk::utils;
use sp1_stark::{ShardProof, StarkVerifyingKey};
use srt::serialization::read_serialize_proof;
use std::{fs, path::PathBuf};
use tracing::{info, warn};

#[derive(Debug, Parser)]
struct Args {
    /// Path to the guest program verification key produced by setup (e.g. vk.bin)
    #[arg(long)]
    guest_program_vk_path: PathBuf,

    /// Directory that contains `proof_*.bin` shard proofs. Ignored if `--proof-path` is used.
    #[arg(long)]
    proofs_dir: Option<PathBuf>,

    /// Explicit shard proof paths. Pass multiple times to verify several shards.
    #[arg(long = "proof-path", value_name = "PATH")]
    proof_paths: Vec<PathBuf>,

    /// Optional public values file to print the raw digest (e.g. public_values.bin)
    #[arg(long)]
    public_values_path: Option<PathBuf>,
}

fn main() -> Result<()> {
    utils::setup_logger();
    let args = Args::parse();

    let proof_paths = collect_proof_paths(&args)?;
    info!("🗂️ Using {} shard proof file(s)", proof_paths.len());

    if let Some(path) = &args.public_values_path {
        match fs::read(path) {
            Ok(data) => {
                let pv = SP1PublicValues::from(&data);
                info!("ℹ️ Loaded public values file {:?} ({} bytes)", path, pv.as_slice().len());
            }
            Err(err) => warn!("could not read public values file {path:?}: {err}"),
        }
    }

    let vk = read_serialize_proof::<StarkVerifyingKey<CoreSC>>(&args.guest_program_vk_path)
        .with_context(|| {
            format!(
                "failed to read guest program verification key at {:?}",
                args.guest_program_vk_path
            )
        })?;
    let verifying_key = SP1VerifyingKey { vk };

    let mut shard_proofs = Vec::with_capacity(proof_paths.len());
    for path in proof_paths.iter() {
        let proof = read_serialize_proof::<ShardProof<CoreSC>>(path).with_context(|| {
            format!("failed to read shard proof at {:?}", path)
        })?;
        shard_proofs.push(proof);
    }

    let prover = SP1Prover::<CpuProverComponents>::new();
    info!("🔍 Verifying shard proofs...");
    prover
        .verify(&SP1CoreProofData(shard_proofs), &verifying_key)
        .map_err(|e| anyhow!("core proof verification failed: {e}"))?;

    info!("🎉 Successfully verified shard proofs");
    Ok(())
}

fn collect_proof_paths(args: &Args) -> Result<Vec<PathBuf>> {
    if !args.proof_paths.is_empty() {
        return Ok(args.proof_paths.clone());
    }

    if let Some(dir) = &args.proofs_dir {
        let mut entries = fs::read_dir(dir)
            .with_context(|| format!("failed to read proofs directory at {:?}", dir))?
            .filter_map(|entry| match entry {
                Ok(e) if e.file_type().map(|ft| ft.is_file()).unwrap_or(false) => Some(e.path()),
                _ => None,
            })
            .collect::<Vec<_>>();
        entries.sort();
        if entries.is_empty() {
            bail!("no shard proof files found in {:?}", dir);
        }
        return Ok(entries);
    }

    bail!("either supply at least one --proof-path or set --proofs-dir");
}
