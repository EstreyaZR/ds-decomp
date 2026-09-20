use std::path::PathBuf;

use anyhow::Result;
use clap::Args;
use ds_decomp::config::{config::Config, symbol::SymbolMaps};

#[derive(Args)]
pub struct EstreyaComRenameArgs {
    #[arg(long, short = 'c')]
    pub config: PathBuf,
    #[arg(long, short = 'o')]
    pub old_name: String,
    #[arg(long, short = 'n')]
    pub new_name: String,
}

impl EstreyaComRenameArgs {
    pub fn run(&self) -> Result<()> {
        let config = Config::from_file(&self.config)?;
        let config_path = self.config.parent().unwrap();

        let mut symbol_maps = SymbolMaps::from_config(config_path, &config)?;
        let old_name = &self.old_name;

        for (module, symbol_map) in symbol_maps.iter_mut() {
            let Some(full) = symbol_map.by_name(old_name)? else {
                log::info!("{old_name} not found in {module}");
                continue;
            };
            log::info!("{old_name} found in {module} at {:#?}!", full.1.addr);
            let symbol = full.1;
            let rename = symbol_map.rename_by_address(symbol.addr, &*self.new_name)?;
            assert!(rename == true);
        }
        symbol_maps.to_files(&config, config_path)?;
        Ok(())
    }
}
