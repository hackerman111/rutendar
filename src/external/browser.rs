use std::{error::Error, process::Command};

pub fn open_url(url: &str) -> Result<(), Box<dyn Error>> {
    validate_url(url)?;
    let status = Command::new("xdg-open").arg(url).status()?;
    if !status.success() {
        return Err(std::io::Error::other("xdg-open failed").into());
    }
    Ok(())
}

pub(crate) fn validate_url(url: &str) -> Result<(), Box<dyn Error>> {
    if url.starts_with("https://") || url.starts_with("http://") {
        Ok(())
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "only http and https links are supported",
        )
        .into())
    }
}
