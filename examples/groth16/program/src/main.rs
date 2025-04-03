//! A program that verifies a Groth16 proof in SP1.

#![no_main]
sp1_zkvm::entrypoint!(main);

use sp1_verifier::Groth16Verifier;

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
        let result = Groth16Verifier::verify(&proof, &sp1_public_values, &sp1_vkey_hash, groth16_vk);
        println!("cycle-tracker-end: verify {}", i);

        match result {
            Ok(()) => println!("Proof {} is valid", i),
            Err(e) => println!("Proof {} failed: {:?}", i, e),
        }
    }
}
