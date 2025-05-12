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
use serde::Deserialize;
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::thread;
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
    // Initialize logger
    utils::setup_logger();

    // Parse CLI arguments
    let args = Args::parse();

    // Create proofs directory if it doesn't exist
    fs::create_dir_all(PREFIX).expect("Failed to create proofs directory");

    // Initialize prover (shared across requests)
    let prover = Arc::new(SP1Prover::<DefaultProverComponents>::new());

    // Build Axum router
    let app = Router::new()
        .route("/first-layer/:index/thread", post(first_layer_thread))
        .route("/first-layer/:index/fork", post(first_layer_fork))
        .route("/two-to-one/thread", post(two_to_one_thread))
        .route("/two-to-one/fork", post(two_to_one_fork))
        .with_state(prover);

    // Start server
    let listener = TcpListener::bind(&args.address).await.expect("Failed to bind to address");
    println!("Server running at {}", args.address);
    axum::serve(listener, app).await.expect("Server failed");
}

// Handler for first-layer compression using a thread
async fn first_layer_thread(
    AxumPath(index): AxumPath<usize>,
    prover: axum::extract::State<Arc<SP1Prover<DefaultProverComponents>>>,
) -> impl IntoResponse {
    let prover = prover.clone();
    let result = thread::spawn(move || {
        run_recursion_first_layer(&prover, index).map_err(|e| format!("Error: {}", e))
    })
    .join();

    match result {
        Ok(Ok(_)) => (StatusCode::OK, "ok".to_string()),
        Ok(Err(e)) => (StatusCode::INTERNAL_SERVER_ERROR, e),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Thread panicked: {:?}", e)),
    }
}

// Handler for first-layer compression using a fork
async fn first_layer_fork(AxumPath(index): AxumPath<usize>) -> impl IntoResponse {
    let status = Command::new(std::env::current_exe().unwrap())
        .arg("--first-layer")
        .arg(index.to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    match status {
        Ok(status) if status.success() => (StatusCode::OK, "ok".to_string()),
        Ok(status) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Forked process failed with code: {}", status),
        ),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to fork: {}", e)),
    }
}

// Handler for two-to-one compression using a thread
async fn two_to_one_thread(
    prover: axum::extract::State<Arc<SP1Prover<DefaultProverComponents>>>,
    Json(req): Json<TwoToOneRequest>,
) -> impl IntoResponse {
    let path1 = Path::new(PREFIX).join(format!("reduced_0_{}.bin", req.index1));
    let path2 = Path::new(PREFIX).join(format!("reduced_0_{}.bin", req.index2));
    let output_filename = format!("reduced_1_{}.bin", req.index1 / 2);
    let output_path = Path::new(PREFIX).join(&output_filename);

    // Verify files exist
    if !path1.exists() || !path2.exists() {
        return (StatusCode::BAD_REQUEST, format!("Proof files not found: {:?}", [path1, path2]));
    }

    let prover = prover.clone();
    let result = thread::spawn(move || {
        let is_final = false;
        run_recursion_two_to_one(&prover, &path1, &path2, &output_path, is_final)
            .map_err(|e| format!("Error: {}", e))
    })
    .join();

    match result {
        Ok(Ok(_)) => (StatusCode::OK, "ok".to_string()),
        Ok(Err(e)) => (StatusCode::INTERNAL_SERVER_ERROR, e),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Thread panicked: {:?}", e)),
    }
}

// Handler for two-to-one compression using a fork
async fn two_to_one_fork(Json(req): Json<TwoToOneRequest>) -> impl IntoResponse {
    let path1 = Path::new(PREFIX).join(format!("reduced_0_{}.bin", req.index1));
    let path2 = Path::new(PREFIX).join(format!("reduced_0_{}.bin", req.index2));

    // Verify files exist
    if !path1.exists() || !path2.exists() {
        return (StatusCode::BAD_REQUEST, format!("Proof files not found: {:?}", [path1, path2]));
    }

    let status = Command::new(std::env::current_exe().unwrap())
        .arg("--two-to-one")
        .arg(req.index1.to_string())
        .arg(req.index2.to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    match status {
        Ok(status) if status.success() => (StatusCode::OK, "ok".to_string()),
        Ok(status) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Forked process failed with code: {}", status),
        ),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to fork: {}", e)),
    }
}
