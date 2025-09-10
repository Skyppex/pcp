use indicatif::{style::TemplateError, ProgressBar, ProgressStyle};

pub fn create_progress_bar(total_size: u64) -> Result<ProgressBar, TemplateError> {
    let progress_bar = ProgressBar::new(total_size);

    progress_bar.set_style(ProgressStyle::default_bar().template(
        "{percent:3}% [{bar:.cyan/blue}] {msg} {bytes_per_sec:.green} {bytes:.yellow}/{total_bytes:.magenta} ({eta:.cyan})",
    )?);

    Ok(progress_bar)
}
