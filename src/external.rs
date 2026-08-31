use std::{
    error::Error,
    io::Write,
    process::{Command, Stdio},
};

pub fn open_url(url: &str) -> Result<(), Box<dyn Error>> {
    validate_url(url)?;
    let status = Command::new("xdg-open").arg(url).status()?;
    if !status.success() {
        return Err(std::io::Error::other("xdg-open failed").into());
    }
    Ok(())
}

pub fn copy_url(url: &str) -> Result<(), Box<dyn Error>> {
    validate_url(url)?;
    if copy_with("wl-copy", &[], url).is_ok() {
        return Ok(());
    }
    copy_with("xclip", &["-selection", "clipboard"], url)
}

fn copy_with(program: &str, arguments: &[&str], value: &str) -> Result<(), Box<dyn Error>> {
    let mut child = Command::new(program)
        .args(arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    child
        .stdin
        .take()
        .ok_or_else(|| std::io::Error::other("clipboard stdin unavailable"))?
        .write_all(value.as_bytes())?;
    if !child.wait()?.success() {
        return Err(std::io::Error::other("clipboard command failed").into());
    }
    Ok(())
}

fn validate_url(url: &str) -> Result<(), Box<dyn Error>> {
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
