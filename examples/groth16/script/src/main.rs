//! A script that generates a Groth16 proof for the Fibonacci program, and verifies the
//! Groth16 proof in SP1.

use sp1_sdk::{include_elf, utils, HashableKey, ProverClient, SP1Stdin};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

/// The ELF for the Groth16 verifier program.
const GROTH16_ELF: &[u8] = include_elf!("groth16-verifier-program");

/// The ELF for the Fibonacci program.
const FIBONACCI_ELF: &[u8] = include_elf!("fibonacci-program");

const PROOF_DIR: &str = "proofs";
const NUM_PROOFS: usize = 5;

fn save_proof_set(
    index: usize,
    proof: Vec<u8>,
    public_inputs: Vec<u8>,
    vk: String,
) -> std::io::Result<()> {
    let dir = "proofs";
    std::fs::create_dir_all(dir)?;
    let path = format!("{}/proof_{}.bin", dir, index);
    let mut file = BufWriter::new(File::create(path)?);

    // Format: lengths + binary blob
    file.write_all(&(proof.len() as u32).to_le_bytes())?;
    file.write_all(&proof)?;

    file.write_all(&(public_inputs.len() as u32).to_le_bytes())?;
    file.write_all(&public_inputs)?;

    file.write_all(&(vk.len() as u32).to_le_bytes())?;
    file.write_all(vk.as_bytes())?;

    Ok(())
}

fn load_proof_set(path: &str) -> std::io::Result<(Vec<u8>, Vec<u8>, String)> {
    use std::io::{BufReader, Read};
    let mut file = BufReader::new(File::open(path)?);

    let mut len_buf = [0u8; 4];

    file.read_exact(&mut len_buf)?;
    let proof_len = u32::from_le_bytes(len_buf) as usize;
    let mut proof = vec![0u8; proof_len];
    file.read_exact(&mut proof)?;

    file.read_exact(&mut len_buf)?;
    let pub_len = u32::from_le_bytes(len_buf) as usize;
    let mut pub_inputs = vec![0u8; pub_len];
    file.read_exact(&mut pub_inputs)?;

    file.read_exact(&mut len_buf)?;
    let vk_len = u32::from_le_bytes(len_buf) as usize;
    let mut vk_bytes = vec![0u8; vk_len];
    file.read_exact(&mut vk_bytes)?;
    let vk = String::from_utf8(vk_bytes).expect("Invalid UTF-8 for vk");

    Ok((proof, pub_inputs, vk))
}

fn generate_and_save_proofs(count: usize) {
    utils::setup_logger();
    for i in 0..count {
        let (proof, public_inputs, vk) = generate_fibonacci_proof();
        save_proof_set(i, proof, public_inputs, vk).expect("Failed to save proof set");
    }
}

/// Generates the proof, public values, and vkey hash for the Fibonacci program in a format that
/// can be read by `sp1-verifier`.
///
/// Returns the proof bytes, public values, and vkey hash.
fn generate_fibonacci_proof() -> (Vec<u8>, Vec<u8>, String) {
    // Create an input stream and write '20' to it.
    let n = 20u32;

    // The input stream that the program will read from using `sp1_zkvm::io::read`. Note that the
    // types of the elements in the input stream must match the types being read in the program.
    let mut stdin = SP1Stdin::new();
    stdin.write(&n);

    // Create a `ProverClient`.
    let client = ProverClient::from_env();

    // Generate the groth16 proof for the Fibonacci program.
    let (pk, vk) = client.setup(FIBONACCI_ELF);
    println!("vk: {:?}", vk.bytes32());
    let proof = client.prove(&pk, &stdin).groth16().run().unwrap();
    (proof.bytes(), proof.public_values.to_vec(), vk.bytes32())
}

fn main() {
    utils::setup_logger();

    // === Phase 1: Generate and save proofs ===
    for i in 0..NUM_PROOFS {
        let (proof, public_inputs, vk) = generate_fibonacci_proof();
        save_proof_set(i, proof, public_inputs, vk).expect("Failed to save proof set");
        println!("[+] Saved proof set {}", i);
    }

    // === Phase 2: Load and verify saved proofs ===
    let client = ProverClient::from_env();

    for i in 0..NUM_PROOFS {
        let (proof, public_inputs, vk) = load_proof_set(&format!("{}/proof_{}.bin", PROOF_DIR, i))
            .expect("Failed to load proof set");

        let mut stdin = SP1Stdin::new();
        stdin.write_vec(proof);
        stdin.write_vec(public_inputs);
        stdin.write(&vk);

        let (_, report) = client.execute(GROTH16_ELF, &stdin).run().unwrap();
        println!("[✓] Verified proof {} with {} cycles", i, report.total_instruction_count());
        println!("report {} is {}", i, report);
    }
}
