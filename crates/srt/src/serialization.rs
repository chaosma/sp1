use std::{
    fs::{create_dir_all, File},
    io::{BufReader, Write},
};

use anyhow::{anyhow, Result};
#[cfg(feature = "mmap")]
use memmap2::MmapMut;
#[cfg(feature = "mmap")]
use mmap_buffer_rs::{align_to_huge_page, mmap_buffer_create, mmap_buffer_prefault_start};
#[cfg(feature = "mmap")]
use p3_field::PrimeField32;
#[cfg(feature = "mmap")]
use p3_matrix::dense::RowMajorMatrix;
#[cfg(feature = "mmap")]
use sp1_prover::InnerSC;

use sp1_sdk::provers::serialize_primitives::SerializeProof;
#[cfg(feature = "mmap")]
use sp1_stark::Val;
use std::io::BufWriter;
use std::path::Path;
#[cfg(feature = "mmap")]
use std::thread::JoinHandle;

#[cfg(feature = "mmap")]
const BYTES_PER_MB: usize = 1024 * 1024;

#[cfg(feature = "mmap")]
const U32_SIZE_BYTES: usize = 4;

#[cfg(feature = "mmap")]
const MATRIX_DIMENSIONS_SIZE_BYTES: usize = 8; // height (4 bytes) + width (4 bytes)

pub fn write_u32_to_file(values: &[u32], file_path: &str) -> Result<()> {
    // Create parent directory if it doesn't exist
    if let Some(parent) = Path::new(file_path).parent() {
        create_dir_all(parent)
            .map_err(|e| anyhow!("Failed to create parent directory for '{}': {}", file_path, e))?;
    }

    // Create the file
    let mut file = File::create(file_path)
        .map_err(|e| anyhow!("Failed to create file '{}': {}", file_path, e))?;

    // Write each u32 value as 4 bytes in little-endian format
    for value in values {
        file.write_all(&value.to_le_bytes())
            .map_err(|e| anyhow!("Failed to write u32 values to '{}': {}", file_path, e))?;
    }

    file.flush().map_err(|e| anyhow!("Failed to flush file '{}': {}", file_path, e))?;
    Ok(())
}

pub fn write_strings_to_file(strings: &Vec<String>, file_path: &str) -> Result<()> {
    // Create parent directory if it doesn't exist
    if let Some(parent) = Path::new(file_path).parent() {
        create_dir_all(parent)
            .map_err(|e| anyhow!("Failed to create parent directory for '{}': {}", file_path, e))?;
    }

    let file = File::create(file_path)
        .map_err(|e| anyhow!("Failed to create file '{}': {}", file_path, e))?;
    let mut writer = BufWriter::new(file);

    // Write strings separated by commas
    for (i, string) in strings.iter().enumerate() {
        if i > 0 {
            write!(writer, ",").map_err(|e| {
                anyhow!("Failed to write comma separator to '{}': {}", file_path, e)
            })?;
        }
        write!(writer, "{}", string)
            .map_err(|e| anyhow!("Failed to write string to '{}': {}", file_path, e))?;
    }

    writer.flush().map_err(|e| anyhow!("Failed to flush file '{}': {}", file_path, e))?;
    Ok(())
}

pub fn write_serialize_proof<T>(data: T, file_path: &str) -> Result<()>
where
    T: SerializeProof,
{
    // Create parent directory if it doesn't exist
    if let Some(parent) = Path::new(file_path).parent() {
        create_dir_all(parent)
            .map_err(|e| anyhow!("Failed to create parent directory for '{}': {}", file_path, e))?;
    }

    let mut file = File::create(file_path)
        .map_err(|e| anyhow!("Failed to create file '{}': {}", file_path, e))?;

    data.to_bytes(&mut file)
        .map_err(|e| anyhow!("Failed to inputs data to '{}': {}", file_path, e))?;

    file.flush().map_err(|e| anyhow!("Failed to flush file '{}': {}", file_path, e))?;

    Ok(())
}

pub fn read_serialize_proof<T>(file_path: &str) -> Result<T>
where
    T: SerializeProof,
{
    let proof_file = File::open(file_path)
        .map_err(|e| anyhow!("Failed to open proof file '{}': {}", file_path, e))?;

    let mut reader = BufReader::new(proof_file);

    let proof = T::from_bytes(&mut reader)
        .map_err(|e| anyhow!("Failed to deserialize proof from '{}': {}", file_path, e))?;
    Ok(proof)
}

#[cfg(feature = "mmap")]
pub fn prepare_mmap_buffer(
    path: &str,
    buffer_size_mb: usize,
) -> Result<(MmapMut, Vec<JoinHandle<std::io::Result<()>>>)> {
    // Create parent directories if they don't exist
    if let Some(parent) = Path::new(path).parent() {
        create_dir_all(parent)
            .map_err(|e| anyhow!("Failed to create parent directory for '{}': {}", path, e))?;
    }

    // Create mmap buffer for traces
    let buffer_size = align_to_huge_page(buffer_size_mb * BYTES_PER_MB);
    let mmap_buffer = mmap_buffer_create(path, buffer_size)
        .map_err(|e| anyhow!("Failed to create mmap buffer for '{}': {}", path, e))?;
    let prefault_threads = mmap_buffer_prefault_start(&mmap_buffer, buffer_size)
        .map_err(|e| anyhow!("Failed to prefault mmap buffer for '{}': {}", path, e))?;
    Ok((mmap_buffer, prefault_threads))
}

#[cfg(feature = "mmap")]
pub trait MatrixProvider {
    fn get_matrices(&self) -> Vec<&RowMajorMatrix<Val<InnerSC>>>;
}

#[cfg(feature = "mmap")]
impl MatrixProvider for Vec<RowMajorMatrix<Val<InnerSC>>> {
    fn get_matrices(&self) -> Vec<&RowMajorMatrix<Val<InnerSC>>> {
        self.iter().collect()
    }
}

#[cfg(feature = "mmap")]
impl MatrixProvider for Vec<(String, RowMajorMatrix<Val<InnerSC>>)> {
    fn get_matrices(&self) -> Vec<&RowMajorMatrix<Val<InnerSC>>> {
        self.iter().map(|(_, matrix)| matrix).collect()
    }
}

#[cfg(feature = "mmap")]
pub fn write_matrixes_to_mmap_buffer<T: MatrixProvider>(
    data: &T,
    buffer: &mut MmapMut,
) -> Result<usize, std::io::Error> {
    let matrices = data.get_matrices();
    let num_matrices = matrices.len() as u32;
    let mut total_size = U32_SIZE_BYTES; // bytes for number of matrices
    total_size += matrices.len() * MATRIX_DIMENSIONS_SIZE_BYTES; // bytes per matrix for dimensions (height, width)

    // Add size for each matrix data
    for matrix in matrices.iter() {
        let values = &matrix.values;
        let matrix_data_size = values.len() * U32_SIZE_BYTES; // bytes per value
        total_size += matrix_data_size;
    }

    // Validate buffer size
    if buffer.len() < total_size {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("Buffer too small: need {} bytes, got {} bytes", total_size, buffer.len()),
        ));
    }

    // Write directly to mmap buffer
    let mut offset = 0;

    // Write total number of matrices
    let num_matrices_bytes = num_matrices.to_le_bytes();
    buffer[offset..offset + U32_SIZE_BYTES].copy_from_slice(&num_matrices_bytes);
    offset += U32_SIZE_BYTES;

    // Write all matrix dimensions first
    for matrix in matrices.iter() {
        let values = &matrix.values;
        let width = matrix.width;
        let height = values.len() / width;

        // Write dimensions
        let height_bytes = (height as u32).to_le_bytes();
        buffer[offset..offset + U32_SIZE_BYTES].copy_from_slice(&height_bytes);
        offset += U32_SIZE_BYTES;

        let width_bytes = (width as u32).to_le_bytes();
        buffer[offset..offset + U32_SIZE_BYTES].copy_from_slice(&width_bytes);
        offset += U32_SIZE_BYTES;
    }

    // Write all matrix data
    for matrix in matrices.iter() {
        let values = &matrix.values;

        // Write data - convert each value to u32
        for value in values {
            let value_bytes = value.as_canonical_u32().to_le_bytes();
            buffer[offset..offset + U32_SIZE_BYTES].copy_from_slice(&value_bytes);
            offset += U32_SIZE_BYTES;
        }
    }
    Ok(offset)
}
