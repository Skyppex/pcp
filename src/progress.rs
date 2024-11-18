use indicatif::{style::TemplateError, ProgressBar, ProgressStyle};

pub fn create_progress_bar(total_size: u64) -> Result<ProgressBar, TemplateError> {
    let progress_bar = ProgressBar::new(total_size);

    progress_bar.set_style(ProgressStyle::default_bar().template(
        "{percent}% {bytes_per_sec:.green} {bytes:.yellow}/{total_bytes:.magenta} [{bar:.cyan/blue}] {msg} ({eta:.cyan})",
    )?);

    Ok(progress_bar)
}
