use std::path::PathBuf;

use anyhow::Result;
use clap::Args;
use ds_decomp::{analysis::graph::Graph, config::config::Config};
#[derive(Args)]
pub struct EstreyaComGraphArgs {
    #[arg(long, short = 'c')]
    pub config_path: PathBuf,
    #[arg(long, short = 'a')]
    pub asm_file: PathBuf,
    #[arg(long, short = 'd')]
    pub dry_run: bool,
}

impl EstreyaComGraphArgs {
    pub fn run(&self) -> Result<()> {
        let _config = Config::from_file(&self.config_path)?;
        let _config_path = self.config_path.parent().unwrap();
        let asm_file = &self.asm_file;

        Graph::from_file(asm_file);
        Ok(())
    }
}
