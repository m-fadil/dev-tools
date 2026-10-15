use crate::errors::{CliError, Result};

pub fn bytes(n: usize) -> Result<Vec<u8>> {
    let mut buf = vec![0u8; n];
    getrandom::fill(&mut buf).map_err(|_| CliError::Failed("Sumber acak OS tidak tersedia.".into()))?;
    Ok(buf)
}
