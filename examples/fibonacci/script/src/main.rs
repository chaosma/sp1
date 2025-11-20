use std::{
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
    process,
};

use anyhow::{anyhow, bail, Context, Result};
use clap::Parser;
use serde::{Deserialize, Serialize};
use sp1_sdk::{
    include_elf, utils, ProverClient, SP1Proof, SP1ProofWithPublicValues, SP1PublicValues, SP1Stdin,
    SP1VerifyingKey,
};
use std::io::Write;

/// The ELF we want to execute inside the zkVM.
const ELF: &[u8] = include_elf!("fibonacci-program");
const DEFAULT_PROOF_DIR: &str = "./proofs";
const INPUT_N: u32 = 20_000;

#[derive(Parser, Debug)]
#[command(author, version, about = "Run the fibonacci example with various proving modes.")]
struct Args {
    /// Generate a core proof by proving each shard individually.
    #[arg(long, conflicts_with = "compress", default_value_t = false)]
    core: bool,

    /// Generate a compressed proof.
    #[arg(long, conflicts_with = "core", default_value_t = false)]
    compress: bool,

    /// Load previously saved shard proofs and verify them.
    #[arg(long, conflicts_with_all = ["core", "compress"], default_value_t = false)]
    load_shards: bool,

    /// Persist shard proofs to disk after generating a core proof.
    #[arg(long, requires = "core", default_value_t = false)]
    save_shards: bool,

    /// Directory to read / write shard proof files.
    #[arg(long, value_name = "DIR", default_value = DEFAULT_PROOF_DIR)]
    proof_dir: PathBuf,
}

#[derive(Serialize, Deserialize)]
struct ShardProofMetadata {
    public_values: SP1PublicValues,
    sp1_version: String,
}

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err:?}");
        process::exit(1);
    }
}

fn run() -> Result<()> {
    utils::setup_logger();

    let args = Args::parse();
    let client = ProverClient::from_env();

    if args.load_shards {
        let vk = load_vk(&args.proof_dir)?;
        let (proof, shard_count) = load_shard_proofs(&args.proof_dir)?;
        println!("loaded {shard_count} shard proofs from {}", args.proof_dir.display());
        client.verify(&proof, &vk).map_err(|err| anyhow!("verification failed: {err}"))?;
        println!("shard proof verification finished.");
        return Ok(());
    }

    let (pk, vk) = client.setup(ELF);

    if !args.core && !args.compress {
        bail!("please specify one of --core, --compress, or --load-shards");
    }

    let mut stdin = SP1Stdin::new();
    stdin.write(&INPUT_N);

    let (_, report) =
        client.execute(ELF, &stdin).run().context("failed to execute program without proving")?;
    println!("executed program with {} cycles", report.total_instruction_count());

    if args.core {
        save_vk(&args.proof_dir, &vk)?;
        let proof =
            client.prove(&pk, &stdin).core().run().context("failed to generate core proof")?;
        println!("generated core proof");

        client.verify(&proof, &vk).map_err(|err| anyhow!("verification failed: {err}"))?;
        println!("core proof verification finished.");

        if args.save_shards {
            let shard_count = save_shard_proofs(&args.proof_dir, &proof)?;
            println!("saved {shard_count} shard proofs under {}", args.proof_dir.display());
        }
    } else if args.compress {
        save_vk(&args.proof_dir, &vk)?;
        let proof = client
            .prove(&pk, &stdin)
            .compressed()
            .run()
            .context("failed to generate compressed proof")?;
        println!("generated compressed proof");

        client.verify(&proof, &vk).map_err(|err| anyhow!("verification failed: {err}"))?;
        println!("compressed proof verification finished.");
    }

    println!("successfully generated and verified proof for the program!");
    Ok(())
}

fn save_vk(dir: &Path, vk: &SP1VerifyingKey) -> Result<()> {
    let vk_path = dir.join("vk.bin");
    fs::create_dir_all(dir).with_context(|| format!("failed to create proof directory {}", dir.display()))?;
    let mut file = File::create(vk_path)?;
    let serialized_vk = bincode::serialize(vk)?;
    file.write_all(&serialized_vk)?;
    Ok(())
}

fn load_vk(dir: &Path) -> Result<SP1VerifyingKey> {
    let vk_path = dir.join("vk.bin");
    let mut file = File::open(vk_path)?;
    let mut serialized_vk = Vec::new();
    file.read_to_end(&mut serialized_vk)?;
    let vk = bincode::deserialize(&serialized_vk)?;
    Ok(vk)
}

fn save_shard_proofs(dir: &Path, proof: &SP1ProofWithPublicValues) -> Result<usize> {
    let SP1Proof::Core(shards) = &proof.proof else {
        bail!("shard files can only be written for core proofs");
    };

    fs::create_dir_all(dir)
        .with_context(|| format!("failed to create proof directory {}", dir.display()))?;

    let metadata = ShardProofMetadata {
        public_values: proof.public_values.clone(),
        sp1_version: proof.sp1_version.clone(),
    };
    let common_path = dir.join("common_data.bin");
    let common_file =
        File::create(&common_path).with_context(|| format!("failed to create {common_path:?}"))?;
    bincode::serialize_into(common_file, &metadata)
        .with_context(|| format!("failed to write {}", common_path.display()))?;

    for (index, shard_proof) in shards.iter().enumerate() {
        let shard_path = dir.join(format!("proof_{index}.bin"));
        let shard_file = File::create(&shard_path)
            .with_context(|| format!("failed to create {}", shard_path.display()))?;
        bincode::serialize_into(shard_file, shard_proof)
            .with_context(|| format!("failed to write {}", shard_path.display()))?;
    }

    Ok(shards.len())
}

fn load_shard_proofs(dir: &Path) -> Result<(SP1ProofWithPublicValues, usize)> {
    let common_path = dir.join("common_data.bin");
    let common_file =
        File::open(&common_path).with_context(|| format!("failed to open {common_path:?}"))?;
    let metadata: ShardProofMetadata = bincode::deserialize_from(common_file)
        .with_context(|| format!("failed to read {}", common_path.display()))?;

    let mut shard_proofs = Vec::new();
    let mut index = 0;
    loop {
        let shard_path = dir.join(format!("proof_{index}.bin"));
        if !shard_path.exists() {
            break;
        }
        let shard_file = File::open(&shard_path)
            .with_context(|| format!("failed to open {}", shard_path.display()))?;
        let shard = bincode::deserialize_from(shard_file)
            .with_context(|| format!("failed to read {}", shard_path.display()))?;
        shard_proofs.push(shard);
        index += 1;
    }

    if shard_proofs.is_empty() {
        bail!("no shard proofs found under {}", dir.display());
    }

    Ok((
        SP1ProofWithPublicValues {
            proof: SP1Proof::Core(shard_proofs),
            public_values: metadata.public_values,
            sp1_version: metadata.sp1_version,
            tee_proof: None,
        },
        index,
    ))
}