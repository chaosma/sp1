use std::{fs::File, io::Read};

use super::primitives::SerializeProof;
use sp1_prover::{SP1CircuitWitness, SP1CoreProofData, SP1ProofWithMetadata};

const FILE: &[u8] = include_bytes!("fib_1000.bin");

#[test]
fn deserialize() {
    let mut buffer: &[u8] = FILE;
    // deserialize
    let proof = SP1ProofWithMetadata::<SP1CoreProofData>::from_bytes(&mut buffer).unwrap();

    // serialize again to check
    let mut serialized: Vec<u8> = Vec::new();
    let written = proof.to_bytes(&mut serialized).unwrap();

    // compare lengths
    assert_eq!(FILE.len(), written);
    // compare byte by byte
    assert_eq!(&FILE, &serialized);
}

// It will fail if made a text as the file contains another type, but
// it can be seen that it compiles.
#[allow(dead_code)]
fn deserialize_input() {
    let mut buffer: &[u8] = FILE;
    // deserialize
    let proof = SP1CircuitWitness::from_bytes(&mut buffer).unwrap();

    // serialize again to check
    let mut serialized: Vec<u8> = Vec::new();
    let written = proof.to_bytes(&mut serialized).unwrap();

    // compare lengths
    assert_eq!(FILE.len(), written);
    // compare byte by byte
    assert_eq!(&FILE, &serialized);
}

// It will fail if made a text as the file contains another type, but
// it can be seen that it compiles.
#[test]
fn deserialize_inputs() {
    for i in 0..11 {
        let path = format!("src/provers/inputs/input_{i}");
        let mut file = File::open(path).unwrap();
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).unwrap();
        let mut buff: &[u8] = &bytes;
        // deserialize
        let proof = SP1CircuitWitness::from_bytes(&mut buff).unwrap();

        // serialize again to check
        let mut serialized: Vec<u8> = Vec::new();
        let written = proof.to_bytes(&mut serialized).unwrap();

        //compare lengths
        assert_eq!(bytes.len(), written);
        // compare byte by byte
        assert_eq!(&bytes, &serialized);
    }
}
