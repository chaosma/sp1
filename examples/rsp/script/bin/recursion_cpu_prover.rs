use anyhow::{anyhow, Result};
#[cfg(feature = "mmap")]
use memmap2::MmapMut;
#[cfg(feature = "mmap")]
use mmap_buffer_rs::{mmap_buffer_destroy, mmap_buffer_prefault_end};
use sp1_prover::{
    components::CpuProverComponents, CoreSC, InnerSC, RecursionInput, RecursionOutput,
    SP1Prover, recursion_trace_generation_first_layer, recursion_trace_generation_two_to_one,
};
use sp1_sdk::{
    utils,
};
use sp1_stark::{ShardProof, StarkVerifyingKey};
use srt::agent::agent_api::{
    task_agent_client::TaskAgentClient, GetTaskRequest, SubmitTaskRequest, TaskStatus, TaskType,
};
use srt::compose::calculate_total_levels;
use srt::deserialize_task;
#[cfg(feature = "mmap")]
use srt::serialization::{prepare_mmap_buffer, write_matrixes_to_mmap_buffer};
use srt::serialization::{
    read_serialize_proof, write_serialize_proof, write_strings_to_file, write_u32_to_file,
};
use srt::task::Task;
#[cfg(feature = "mmap")]
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use tracing::{error, info};

fn log_task_timing(
    input_load_duration: Duration,
    mmap_init_duration: Duration,
    recursion_duration: Duration,
    serialize_duration: Duration,
    submit_duration: Duration,
    total_duration: Duration,
) {
    info!("Loading input took: {}ms", input_load_duration.as_millis());
    info!("Mmap initialization took: {}ms", mmap_init_duration.as_millis());
    info!("Recursion execution took: {}ms", recursion_duration.as_millis());
    info!("Serialization took: {}ms", serialize_duration.as_millis());
    info!("Submit took: {}ms", submit_duration.as_millis());
    info!("Total duration: {}ms", total_duration.as_millis());
}

#[cfg(feature = "mmap")]
const ONE_TO_ONE_BUFFER_SIZE_MB: usize = 400;
#[cfg(feature = "mmap")]
const TWO_TO_ONE_BUFFER_SIZE_MB: usize = 300;

#[cfg(feature = "mmap")]
fn serialize_recursion_output_mmap(
    result: &RecursionOutput,
    output_paths: &[String],
    mut preprocessed_trace_mmap_buffer: MmapMut,
    mut trace_mmap_buffer: MmapMut,
    preprocessed_trace_prefault_threads: Vec<JoinHandle<std::io::Result<()>>>,
    trace_prefault_threads: Vec<JoinHandle<std::io::Result<()>>>,
) -> anyhow::Result<()> {
    // Write traces to mmap buffers
    mmap_buffer_prefault_end(preprocessed_trace_prefault_threads)
        .map_err(|e| anyhow::anyhow!("Failed to end preprocessed trace prefault: {}", e))?;
    write_matrixes_to_mmap_buffer(&result.preprocessed_traces, &mut preprocessed_trace_mmap_buffer)
        .map_err(|e| anyhow::anyhow!("Failed to write preprocessed traces to mmap buffer: {}", e))?;
    mmap_buffer_destroy(preprocessed_trace_mmap_buffer)
        .map_err(|e| anyhow::anyhow!("Failed to destroy preprocessed trace mmap buffer: {}", e))?;

    mmap_buffer_prefault_end(trace_prefault_threads)
        .map_err(|e| anyhow::anyhow!("Failed to end trace prefault: {}", e))?;
    write_matrixes_to_mmap_buffer(&result.traces, &mut trace_mmap_buffer)
        .map_err(|e| anyhow::anyhow!("Failed to write traces to mmap buffer: {}", e))?;
    mmap_buffer_destroy(trace_mmap_buffer)
        .map_err(|e| anyhow::anyhow!("Failed to destroy trace mmap buffer: {}", e))?;

    // Write metadata files
    serialize_recursion_output_files(result, output_paths)
}

fn serialize_recursion_output_files(
    result: &RecursionOutput,
    output_paths: &[String],
) -> anyhow::Result<()> {
    // Extract trace names from new format (small string copy only)
    let trace_names: Vec<String> = result.traces.iter()
        .map(|(name, _)| name.clone())
        .collect();

    // For non-mmap case, we skip the binary trace data (output_paths[0] and [1])
    // and only write the metadata files
    write_strings_to_file(&result.preprocessed_trace_names, &output_paths[2])?;
    write_strings_to_file(&trace_names, &output_paths[3])?;
    write_serialize_proof(result.vk.clone(), &output_paths[4])?;
    write_u32_to_file(&result.public_values, &output_paths[5])?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    utils::setup_logger();
    info!("Starting recursion CPU prover ");

    // Get task agent IP address from environment variable
    let task_agent_ip_address =
        std::env::var("TASK_AGENT_IP_ADDRESS").unwrap_or_else(|_| "127.0.0.1:50052".to_string());

    let mut task_agent_client =
        TaskAgentClient::connect(format!("http://{}", task_agent_ip_address.clone()))
            .await
            .map_err(|e| {
                anyhow!("Failed to connect to task agent at {}: {:?}", task_agent_ip_address, e)
            })?;

    let prover = SP1Prover::<CpuProverComponents>::new();

    loop {
        let get_task_request =
            tonic::Request::new(GetTaskRequest { task_type: TaskType::RecursionCpu.into() });
        match task_agent_client.get_task(get_task_request).await {
            Ok(response) => {
                let task = &response.into_inner().task;
                match task {
                    Some(task_proto) => {
                        let task = deserialize_task(&task_proto.task_id)?;
                        info!("Received task: {:?}", &task);
                        match task {
                            Task::RecursionCpu(recursion_task) => {
                                if recursion_task.height == 0 {
                                    let start_time = Instant::now();

                                    // Load input data
                                    let input_load_start = Instant::now();
                                    let proof = read_serialize_proof::<ShardProof<CoreSC>>(
                                        &task_proto.input_paths[0],
                                    )?;
                                    let vk = read_serialize_proof::<StarkVerifyingKey<CoreSC>>(
                                        &task_proto.input_paths[1],
                                    )?;
                                    let input_load_duration = input_load_start.elapsed();

                                    let mmap_init_start = Instant::now();
                                    #[cfg(feature = "mmap")]
                                    let (
                                        preprocessed_trace_mmap_buffer,
                                        preprocessed_trace_prefault_threads,
                                    ) = prepare_mmap_buffer(
                                        &task_proto.output_paths[0],
                                        ONE_TO_ONE_BUFFER_SIZE_MB,
                                    )?;
                                    #[cfg(feature = "mmap")]
                                    let (trace_mmap_buffer, trace_prefault_threads) =
                                        prepare_mmap_buffer(
                                            &task_proto.output_paths[1],
                                            ONE_TO_ONE_BUFFER_SIZE_MB,
                                        )?;
                                    let mmap_init_duration = mmap_init_start.elapsed();

                                    // Run recursion
                                    let recursion_start = Instant::now();
                                    let result = recursion_trace_generation_first_layer(
                                        &prover,
                                        RecursionInput::Single {
                                            vk,
                                            proof,
                                            is_first_shard: recursion_task.index == 0,
                                        },
                                    )?;
                                    let recursion_duration = recursion_start.elapsed();

                                    // Serialize outputs
                                    let serialize_start = Instant::now();
                                    #[cfg(feature = "mmap")]
                                    serialize_recursion_output_mmap(
                                        &result,
                                        &task_proto.output_paths,
                                        preprocessed_trace_mmap_buffer,
                                        trace_mmap_buffer,
                                        preprocessed_trace_prefault_threads,
                                        trace_prefault_threads,
                                    )?;
                                    #[cfg(not(feature = "mmap"))]
                                    serialize_recursion_output_files(
                                        &result,
                                        &task_proto.output_paths,
                                    )?;
                                    let serialize_duration = serialize_start.elapsed();

                                    let submit_start = Instant::now();
                                    let total_duration_for_request = start_time.elapsed();
                                    let submit_task_request =
                                        tonic::Request::new(SubmitTaskRequest {
                                            stream_key: task_proto.stream_key.clone(),
                                            message_id: task_proto.message_id.clone(),
                                            task_id: task_proto.task_id.clone(),
                                            status: TaskStatus::Success.into(),
                                            error_message: None,
                                            extra_data: "".to_string(),
                                            total_duration_ms: total_duration_for_request
                                                .as_millis()
                                                as u32,
                                        });
                                    task_agent_client.submit_task(submit_task_request).await?;
                                    let submit_duration = submit_start.elapsed();

                                    let total_duration = start_time.elapsed();

                                    log_task_timing(
                                        input_load_duration,
                                        mmap_init_duration,
                                        recursion_duration,
                                        serialize_duration,
                                        submit_duration,
                                        total_duration,
                                    );
                                } else {
                                    let start_time = Instant::now();
                                    let is_complete = match recursion_task.total_shards {
                                        Some(total_shards) => {
                                            recursion_task.index == 0
                                                && recursion_task.height + 1
                                                    == calculate_total_levels(total_shards)
                                        }
                                        None => false,
                                    };

                                    // Load input data
                                    let input_load_start = Instant::now();
                                    let proof1 = read_serialize_proof::<ShardProof<InnerSC>>(
                                        &task_proto.input_paths[0],
                                    )?;
                                    let vk1 = read_serialize_proof::<StarkVerifyingKey<InnerSC>>(
                                        &task_proto.input_paths[1],
                                    )?;
                                    let proof2 = read_serialize_proof::<ShardProof<InnerSC>>(
                                        &task_proto.input_paths[2],
                                    )?;
                                    let vk2 = read_serialize_proof::<StarkVerifyingKey<InnerSC>>(
                                        &task_proto.input_paths[3],
                                    )?;
                                    let input_load_duration = input_load_start.elapsed();

                                    let mmap_init_start = Instant::now();
                                    #[cfg(feature = "mmap")]
                                    let (
                                        preprocessed_trace_mmap_buffer,
                                        preprocessed_trace_prefault_threads,
                                    ) = prepare_mmap_buffer(
                                        &task_proto.output_paths[0],
                                        TWO_TO_ONE_BUFFER_SIZE_MB,
                                    )?;
                                    #[cfg(feature = "mmap")]
                                    let (trace_mmap_buffer, trace_prefault_threads) =
                                        prepare_mmap_buffer(
                                            &task_proto.output_paths[1],
                                            TWO_TO_ONE_BUFFER_SIZE_MB,
                                        )?;
                                    let mmap_init_duration = mmap_init_start.elapsed();

                                    // Run recursion
                                    let recursion_start = Instant::now();
                                    let result = recursion_trace_generation_two_to_one(
                                        &prover,
                                        RecursionInput::Single {
                                            vk: vk1,
                                            proof: proof1,
                                            is_first_shard: false,
                                        },
                                        RecursionInput::Single {
                                            vk: vk2,
                                            proof: proof2,
                                            is_first_shard: false,
                                        },
                                        is_complete,
                                    )?;
                                    let recursion_duration = recursion_start.elapsed();

                                    // Serialize outputs
                                    let serialize_start = Instant::now();
                                    #[cfg(feature = "mmap")]
                                    serialize_recursion_output_mmap(
                                        &result,
                                        &task_proto.output_paths,
                                        preprocessed_trace_mmap_buffer,
                                        trace_mmap_buffer,
                                        preprocessed_trace_prefault_threads,
                                        trace_prefault_threads,
                                    )?;
                                    #[cfg(not(feature = "mmap"))]
                                    serialize_recursion_output_files(
                                        &result,
                                        &task_proto.output_paths,
                                    )?;

                                    let serialize_duration = serialize_start.elapsed();

                                    let submit_start = Instant::now();
                                    let total_duration_for_request = start_time.elapsed();
                                    let submit_task_request =
                                        tonic::Request::new(SubmitTaskRequest {
                                            stream_key: task_proto.stream_key.clone(),
                                            message_id: task_proto.message_id.clone(),
                                            task_id: task_proto.task_id.clone(),
                                            status: TaskStatus::Success.into(),
                                            error_message: None,
                                            extra_data: "".to_string(),
                                            total_duration_ms: total_duration_for_request
                                                .as_millis()
                                                as u32,
                                        });
                                    task_agent_client.submit_task(submit_task_request).await?;
                                    let submit_duration = submit_start.elapsed();

                                    let total_duration = start_time.elapsed();

                                    log_task_timing(
                                        input_load_duration,
                                        mmap_init_duration,
                                        recursion_duration,
                                        serialize_duration,
                                        submit_duration,
                                        total_duration,
                                    );
                                }
                            }
                            _ => {
                                info!("Received unexpected task: {:?}", task);
                            }
                        }
                    }
                    None => {
                        continue;
                    }
                }
            }
            Err(e) => {
                error!("GetTask request failed: {:?}", e);
            }
        }
    }
}
