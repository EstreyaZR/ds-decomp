use std::path::PathBuf;

use anyhow::Result;
use clap::Args;

#[derive(Args)]
pub struct EstreyaComSizeArgs {
    #[arg(long, short = 'c')]
    pub config: PathBuf,
    #[arg(long, short = 'n')]
    pub name: String,
    #[arg(long, short = 's')]
    pub new_size: u32,
}

impl EstreyaComSizeArgs {
    pub fn run(&self) -> Result<()> {
        todo!();
    }
}
