use clap::Parser;
use nix::{
    sys::wait::{waitpid, WaitPidFlag},
    unistd::{fork, ForkResult},
};
use serde::Deserialize;
use sp1_prover::{components::DefaultProverComponents, SP1Prover};
use sp1_sdk::action::{run_recursion_first_layer, run_recursion_two_to_one};
use std::{
    fs,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::Path,
    process,
    sync::Arc,
    time::Duration,
};

const PREFIX: &str = "./proofs/";

#[derive(Parser, Debug)]
struct Args {
    /// listening address, e.g. 0.0.0.0:3000
    #[arg(long, default_value = "127.0.0.1:3000")]
    address: String,
}

#[derive(Deserialize)]
struct TwoToOneRequest {
    index1: usize,
    index2: usize,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    fs::create_dir_all(PREFIX)?;

    let listener = TcpListener::bind(&args.address)?;
    println!("parent: bound to {}", args.address);

    // let workers = num_cpus::get();
    let workers = 16;
    println!("parent: forking {workers} workers");

    for _ in 0..workers {
        match unsafe { fork()? } {
            ForkResult::Parent { .. } => {}
            ForkResult::Child => {
                // Heavy initialisation AFTER fork -- one per worker
                let prover = Arc::new(SP1Prover::<DefaultProverComponents>::new());
                println!("worker {} ready", process::id());

                worker_loop(listener.try_clone()?, prover)?;
                process::exit(0);
            }
        }
    }

    // Parent: reap zombies, nothing else
    loop {
        let _ = waitpid(None, Some(WaitPidFlag::WNOHANG));
        std::thread::sleep(Duration::from_secs(1));
    }
}

// ======================================================================
//                      Worker code
// ======================================================================
fn worker_loop(
    listener: TcpListener,
    prover: Arc<SP1Prover<DefaultProverComponents>>,
) -> anyhow::Result<()> {
    println!("worker pid {} ready", process::id());

    loop {
        let (mut stream, addr) = listener.accept()?; // blocking
        stream.set_read_timeout(Some(Duration::from_secs(5))).ok();
        if let Err(e) = handle_connection(&mut stream, &prover) {
            eprintln!("worker {} – {} – {e}", process::id(), addr);
        }
    }
}

// ---------------------------- request handling -------------------------
fn handle_connection(
    stream: &mut TcpStream,
    prover: &Arc<SP1Prover<DefaultProverComponents>>,
) -> anyhow::Result<()> {
    let mut buf = [0u8; 8192];
    let n = stream.read(&mut buf)?;
    if n == 0 {
        return Ok(());
    }

    // --- minimal HTTP parsing -----------------------------------------
    let mut headers = [httparse::EMPTY_HEADER; 16];
    let mut req = httparse::Request::new(&mut headers);
    let parsed = req.parse(&buf[..n])?;
    if !parsed.is_complete() {
        return write_resp(stream, 400, "bad request");
    }
    if req.method != Some("POST") {
        return write_resp(stream, 405, "POST only");
    }
    let path = req.path.unwrap_or("");

    let body = &buf[parsed.unwrap()..n];

    // --- routing -------------------------------------------------------
    if let Some(idx) = path.strip_prefix("/first-layer/") {
        let index: usize = idx.parse().unwrap_or(usize::MAX);
        println!("worker {} → first-layer idx={index}", process::id());

        match run_recursion_first_layer(prover, index) {
            Ok(_) => write_resp(stream, 200, "ok"),
            Err(e) => write_resp(stream, 500, &format!("error: {e}")),
        }
    } else if path == "/two-to-one" {
        let req_json: TwoToOneRequest = serde_json::from_slice(body)?;
        let p1 = Path::new(PREFIX).join(format!("reduced_0_{}.bin", req_json.index1));
        let p2 = Path::new(PREFIX).join(format!("reduced_0_{}.bin", req_json.index2));
        let out = Path::new(PREFIX).join(format!("reduced_1_{}.bin", req_json.index1 / 2));

        if !p1.exists() || !p2.exists() {
            return write_resp(stream, 400, "missing proofs");
        }

        println!("worker {} → 2-to-1 {} {}", process::id(), req_json.index1, req_json.index2);

        match run_recursion_two_to_one(prover, &p1, &p2, &out, false) {
            Ok(_) => write_resp(stream, 200, "ok"),
            Err(e) => write_resp(stream, 500, &format!("error: {e}")),
        }
    } else {
        write_resp(stream, 404, "not found")
    }
}

// ---------------------------- helpers ----------------------------------
fn write_resp(stream: &mut TcpStream, code: u16, msg: &str) -> anyhow::Result<()> {
    let body = msg.as_bytes();
    write!(
        stream,
        "HTTP/1.1 {code}\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )?;
    stream.write_all(body)?;
    Ok(())
}
