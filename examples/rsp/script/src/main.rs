use clap::Parser;
use sp1_prover::{components::DefaultProverComponents, SP1Prover};
use sp1_sdk::action::{run_recursion_first_layer, run_recursion_two_to_one};

use sp1_sdk::utils;

use axum::{
    extract::{Json, Path as AxumPath},
    http::StatusCode,
    response::IntoResponse,
    routing::post,
    Router,
};
use nix::sys::wait::waitpid;
use nix::unistd::{fork, ForkResult};
use serde::Deserialize;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tokio::net::TcpListener;

const PREFIX: &str = "./proofs/";

#[derive(Parser, Debug)]
struct Args {
    #[arg(long, default_value = "127.0.0.1:3000")]
    address: String,
}

#[derive(Deserialize)]
struct TwoToOneRequest {
    index1: usize,
    index2: usize,
}

#[tokio::main]
async fn main() {
    utils::setup_logger();
    let args = Args::parse();
    fs::create_dir_all(PREFIX).expect("Failed to create proofs directory");

    let prover = Arc::new(SP1Prover::<DefaultProverComponents>::new());
    let app = Router::new()
        .route("/first-layer/:index", post(first_layer))
        .route("/two-to-one", post(two_to_one))
        .with_state(prover);
    let listener = TcpListener::bind(&args.address).await.expect("Failed to bind to address");
    println!("Server running at {}", args.address);
    axum::serve(listener, app).await.expect("Server failed");
}

async fn first_layer(
    AxumPath(index): AxumPath<usize>,
    prover: axum::extract::State<Arc<SP1Prover<DefaultProverComponents>>>,
) -> impl IntoResponse {
    let prover = prover.clone();
    match unsafe { fork() }.expect("Fork failed") {
        ForkResult::Child => {
            let result = run_recursion_first_layer(&prover, index);
            std::process::exit(match result {
                Ok(_) => 0,
                Err(e) => {
                    eprintln!("Child error: {}", e);
                    1
                }
            });
        }
        ForkResult::Parent { child } => match waitpid(child, None) {
            Ok(nix::sys::wait::WaitStatus::Exited(_, 0)) => (StatusCode::OK, "ok".to_string()),
            Ok(status) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Forked process failed with status: {:?}", status),
            ),
            Err(e) => {
                (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to wait for child: {}", e))
            }
        },
    }
}

async fn two_to_one(
    prover: axum::extract::State<Arc<SP1Prover<DefaultProverComponents>>>,
    Json(req): Json<TwoToOneRequest>,
) -> impl IntoResponse {
    let path1 = Path::new(PREFIX).join(format!("reduced_0_{}.bin", req.index1));
    let path2 = Path::new(PREFIX).join(format!("reduced_0_{}.bin", req.index2));
    let output_filename = format!("reduced_1_{}.bin", req.index1 / 2);
    let output_path = Path::new(PREFIX).join(&output_filename);

    if !path1.exists() || !path2.exists() {
        return (StatusCode::BAD_REQUEST, format!("Proof files not found: {:?}", [path1, path2]));
    }

    let prover = prover.clone();
    match unsafe { fork() }.expect("Fork failed") {
        ForkResult::Child => {
            let result = run_recursion_two_to_one(&prover, &path1, &path2, &output_path, false);
            std::process::exit(match result {
                Ok(_) => 0,
                Err(e) => {
                    eprintln!("Child error: {}", e);
                    1
                }
            });
        }
        ForkResult::Parent { child } => match waitpid(child, None) {
            Ok(nix::sys::wait::WaitStatus::Exited(_, 0)) => (StatusCode::OK, "ok".to_string()),
            Ok(status) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Forked process failed with status: {:?}", status),
            ),
            Err(e) => {
                (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to wait for child: {}", e))
            }
        },
    }
}
