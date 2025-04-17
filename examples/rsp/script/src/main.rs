use alloy_primitives::B256;
use anyhow::Result;
use bincode;
use clap::Parser;
use rsp_client_executor::{io::ClientExecutorInput, CHAIN_ID_ETH_MAINNET};
use sp1_sdk::{SP1Proof, SP1ProofCommonData, SP1ProofWithPublicValues};
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;

use sp1_sdk::{include_elf, utils, ProverClient, SP1Stdin};

const PREFIX: &str = "./proofs/";

#[derive(Parser, Debug)]
struct Args {
    /// Whether or not to generate a proof.
    #[arg(long, default_value_t = false)]
    prove: bool,
    #[arg(long, default_value_t = false)]
    compress: bool,
}

fn load_input_from_cache(chain_id: u64, block_number: u64) -> ClientExecutorInput {
    let cache_path = PathBuf::from(format!("./input/{}/{}.bin", chain_id, block_number));
    let mut cache_file = std::fs::File::open(cache_path).unwrap();
    let client_input: ClientExecutorInput = bincode::deserialize_from(&mut cache_file).unwrap();

    client_input
}

fn load_proofs() -> Result<SP1ProofWithPublicValues> {
    let common_path = Path::new(PREFIX).join("common_data.bin");
    let mut common_file = File::open(&common_path)?;
    let mut common_serialized = Vec::new();
    common_file.read_to_end(&mut common_serialized)?;
    let common_data: SP1ProofCommonData = bincode::deserialize(&common_serialized)?;

    // Load ShardProofs from proof_0.bin, proof_1.bin, etc.
    let mut shard_proofs = Vec::new();
    let mut index = 0;
    loop {
        let shard_path = Path::new(PREFIX).join(format!("proof_{}.bin", index));
        if !shard_path.exists() {
            break; // Stop when proof_{index}.bin is not found
        }
        let mut shard_file = File::open(&shard_path)?;
        let mut shard_serialized = Vec::new();
        shard_file.read_to_end(&mut shard_serialized)?;
        let shard_proof = bincode::deserialize(&shard_serialized)?;
        shard_proofs.push(shard_proof);
        index += 1;
    }

    Ok(SP1ProofWithPublicValues {
        proof: SP1Proof::Core(shard_proofs),
        stdin: common_data.stdin,
        public_values: common_data.public_values,
        sp1_version: common_data.sp1_version,
    })
}

fn main() {
    // Initialize the logger.
    utils::setup_logger();

    // Parse the command line arguments.
    let args = Args::parse();

    // Load the input from the cache.
    let client_input = load_input_from_cache(CHAIN_ID_ETH_MAINNET, 20526624);

    // Generate the proof.
    let client = ProverClient::new();

    // Setup the proving key and verification key.
    let (pk, vk) = client.setup(include_elf!("rsp-program"));

    // Write the block to the program's stdin.
    let mut stdin = SP1Stdin::new();
    let buffer = bincode::serialize(&client_input).unwrap();
    stdin.write_vec(buffer);

    // Only execute the program.
    let (mut public_values, execution_report) =
        client.execute(&pk.elf, stdin.clone()).run().unwrap();
    println!(
        "Finished executing the block in {} cycles",
        execution_report.total_instruction_count()
    );

    // Read the block hash.
    let block_hash = public_values.read::<B256>();
    println!("success: block_hash={block_hash}");

    // If the `prove` argument was passed in, actually generate the proof.
    // It is strongly recommended you use the network prover given the size of these programs.
    if args.prove {
        println!("Starting shard proof generation.");
        client.prove(&pk, stdin).run_shard_proof().expect("Proving should work.");
        println!("Proof generation finished.");

        let proof = load_proofs().unwrap();
        client.verify(&proof, &vk).expect("proof verification should succeed");
    } else if args.compress {
        println!("[hehe1] Starting compress proof generation.");
        let proof = client.prove(&pk, stdin).compressed().run().expect("Proving should work.");
        println!("Proof generation finished.");

        client.verify(&proof, &vk).expect("[hehe1] proof verification should succeed");
    } else {
        panic!("not supported");
    }
}
