use std::path::PathBuf;

use anyhow::Result;
use clap::Args;
use ds_decomp::{
    analysis::graph::{Graph, GraphNodes, GraphParser},
    config::config::Config,
};

use crate::util::io::read_dir;
#[derive(Args)]
pub struct EstreyaComGraphArgs {
    #[arg(long, short = 'c')]
    pub config_path: PathBuf,
    #[arg(long, short = 'a')]
    pub asm_dir: PathBuf,
    #[arg(long, short = 'd')]
    pub dry_run: bool,
}

impl EstreyaComGraphArgs {
    pub fn run(&self) -> Result<()> {
        let _config = Config::from_file(&self.config_path)?;
        let _config_path = self.config_path.parent().unwrap();
        if let Ok(nodes) = Self::dir_crawler(&self.asm_dir) {
            Graph::with_graph_nodes(nodes);
        }
        Ok(())
    }

    pub fn dir_crawler(path: &PathBuf) -> Result<GraphNodes> {
        let mut vec = GraphNodes::default();

        let dir = read_dir(path).unwrap();
        for file in dir {
            let file = file.unwrap();
            let path = file.path();
            if path.is_dir() {
                if let Ok(nodes) = Self::dir_crawler(&path) {
                    vec.0.extend(nodes);
                }
            } else {
                let entries = GraphParser::parse(path).unwrap();
                let nodes = entries.to_nodes();
                let nodes = nodes.resolve_labels();
                vec.0.extend(nodes.0);
            }
        }
        Ok(vec)
    }
}
