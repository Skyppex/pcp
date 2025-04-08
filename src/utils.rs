use walkdir::DirEntry;

pub fn calculate_total_size(files: &[DirEntry]) -> u64 {
    files
        .iter()
        .map(|src| std::fs::metadata(src.path()).map(|m| m.len()).unwrap_or(0))
        .sum()
}
