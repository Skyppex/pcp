use indicatif::{style::TemplateError, ProgressBar, ProgressStyle};

pub fn create_progress_bar(total_size: u64) -> Result<ProgressBar, TemplateError> {
    let progress_bar = ProgressBar::new(total_size);

    progress_bar.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")?
        .progress_chars("#>-"));

    Ok(progress_bar)
}
