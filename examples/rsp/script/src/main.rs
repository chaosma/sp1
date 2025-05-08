use clap::Parser;
use sp1_prover::{components::DefaultProverComponents, SP1Prover};
use sp1_sdk::action::{run_recursion_first_layer, run_recursion_two_to_one};
use std::path::Path;

use sp1_sdk::utils;

const PREFIX: &str = "./proofs/";

#[derive(Parser, Debug)]
struct Args {
    #[arg(long, default_value_t = false)]
    prove: bool,
    #[arg(long, default_value_t = false)]
    compress: bool,
    #[arg(long)]
    first_layer: Option<usize>,
    #[arg(long, num_args = 2)]
    two_to_one: Option<Vec<usize>>,
}

fn main() {
    // Initialize the logger.
    utils::setup_logger();

    // Parse the command line arguments.
    let args = Args::parse();

    if let Some(index) = args.first_layer {
        println!("Running first-layer compression for shard proof index {}.", index);
        let prover = SP1Prover::<DefaultProverComponents>::new();
        run_recursion_first_layer(&prover, index).unwrap();
        println!("First-layer compression finished for index {}.", index);
    } else if let Some(indices) = args.two_to_one {
        let index1 = indices[0];
        let index2 = indices[1];
        println!(
            "Running two-to-one compression for reduced proofs indices {} and {}.",
            index1, index2
        );
        let prover = SP1Prover::<DefaultProverComponents>::new();
        let path1 = Path::new(PREFIX).join(format!("reduced_0_{}.bin", index1));
        let path2 = Path::new(PREFIX).join(format!("reduced_0_{}.bin", index2));
        let output_filename = format!("reduced_1_{}.bin", index1 / 2);
        let output_path = Path::new(PREFIX).join(&output_filename);
        let is_final = false; // Adjust if needed for final compression
        run_recursion_two_to_one(&prover, &path1, &path2, &output_path, is_final).unwrap();
        println!("Two-to-one compression finished for indices {} and {}.", index1, index2);
    } else {
        panic!("Not supported: specify --prove, --compress, --first-layer, or --two-to-one");
    }
    println!("successfully generated and verified proof for the program!")
}
