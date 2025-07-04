use crate::data::virtual_file::VirtualFile;

pub fn calc_size(size: u64) -> u64 {
    ((size + 1023) as i64 & -1024) as u64
}

pub fn calculate_size(files: &[VirtualFile]) -> std::io::Result<u64> {
    let total = files
            .iter()
            .map(|f| {
                let metadata = std::fs::metadata(&f.file_path)?;
                Ok(512 + calc_size(metadata.len()))
            })
            .collect::<std::io::Result<Vec<u64>>>()?
            .into_iter()
            .sum::<u64>();

    Ok((512 * 3) + total) // First 3 entries + total size of files
}
