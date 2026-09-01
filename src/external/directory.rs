use std::{error::Error, ffi::OsString, path::Path, process::Command};

pub fn open_shell(directory: &Path) -> Result<(), Box<dyn Error>> {
    if !directory.is_dir() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "event directory does not exist",
        )
        .into());
    }
    let shell = std::env::var_os("SHELL").unwrap_or_else(|| OsString::from("/bin/sh"));
    _ = Command::new(shell).current_dir(directory).status()?;
    Ok(())
}
