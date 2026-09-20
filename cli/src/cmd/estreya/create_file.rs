use std::path::PathBuf;

use anyhow::Result;
use clap::Args;

#[derive(Args)]
pub struct EstreyaComCreateFileArgs {
    #[arg(long, short = 'c')]
    pub config: PathBuf,
    #[arg(long, short = 's')]
    pub symbol: String,
}

impl EstreyaComCreateFileArgs {
    pub fn run(&self) -> Result<()> {
        todo!();
    }
}
