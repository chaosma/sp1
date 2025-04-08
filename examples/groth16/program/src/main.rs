//! A program that verifies a Groth16 proof in SP1.

#![no_main]
sp1_zkvm::entrypoint!(main);

use sha2::{Digest, Sha256};
use sp1_verifier::Groth16Verifier;

pub fn hash(input: &[u8]) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(input);
    hasher.finalize().to_vec()
}

/*
pub fn main() {
    // Read the number of proofs to verify
    let num_proofs: u32 = sp1_zkvm::io::read();

    let sp1_vkey_hash: String = sp1_zkvm::io::read();
    // Verification key (same for all proofs)
    let groth16_vk = *sp1_verifier::GROTH16_VK_BYTES;

    for i in 0..num_proofs {
        // Read each proof triplet
        let proof = sp1_zkvm::io::read_vec();
        let sp1_public_values = sp1_zkvm::io::read_vec();

        println!("cycle-tracker-start: verify {}", i);
        let result =
            Groth16Verifier::verify(&proof, &sp1_public_values, &sp1_vkey_hash, groth16_vk);
        println!("cycle-tracker-end: verify {}", i);

        match result {
            Ok(()) => println!("Proof {} is valid", i),
            Err(e) => println!("Proof {} failed: {:?}", i, e),
        }
    }
}
*/
pub fn main() {
    let num_proofs: u32 = sp1_zkvm::io::read();
    let sp1_vkey_hash: String = sp1_zkvm::io::read();
    let groth16_vk = *sp1_verifier::GROTH16_VK_BYTES;

    let mut proofs = Vec::with_capacity(num_proofs.try_into().unwrap());
    let mut public_inputs = Vec::with_capacity(num_proofs.try_into().unwrap());

    for i in 0..num_proofs {
        let proof = sp1_zkvm::io::read_vec();
        let sp1_public_values = sp1_zkvm::io::read_vec();
        proofs.push(proof);
        public_inputs.push(sp1_public_values);
    }

    let proofs: Vec<&[u8]> = proofs.iter().map(|p| p.as_slice()).collect();
    let public_inputs: Vec<&[u8]> = public_inputs.iter().map(|p| p.as_slice()).collect();

    println!("cycle-tracker-start: batch_verify");
    let result = Groth16Verifier::batch_verify(&proofs, &public_inputs, &sp1_vkey_hash, groth16_vk);
    println!("cycle-tracker-end: batch_verify");

    match result {
        Ok(()) => println!("All proofs are valid"),
        Err(e) => println!("Batch verification failed: {:?}", e),
    }
}
