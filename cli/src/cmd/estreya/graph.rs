use std::path::PathBuf;

use anyhow::Result;
use clap::Args;
use ds_decomp::{
    analysis::graph::{Graph, GraphOptions},
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
        if let Ok(files) = Self::dir_crawler(&self.asm_dir) {
            //log::info!("{:#?}", nodes);
            let mut graph = Graph::from_files(GraphOptions { debug: false }, files);
            graph.apply_tags();
            log::info!("Finished ApplyTags");
            graph.init_edges();
            log::info!("Finished InitEdges");
            graph.init_called_by();
            log::info!("Finished InitCalledBy");

            let mut counter = 10;
            while counter > 0 {
                graph.init_tree_status();
                // log::info!("Finished InitTreeStatus");
                // log::info!("Finished InitTreeStatus");
                graph.find_trees();
                counter -= 1;
            }
        }
        Ok(())
    }

    pub fn dir_crawler(path: &PathBuf) -> Result<Vec<PathBuf>> {
        let mut vec = Vec::<PathBuf>::default();

        let dir = read_dir(path).unwrap();
        for file in dir {
            let file = file.unwrap();
            let path = file.path();
            if path.is_dir() {
                if let Ok(paths) = Self::dir_crawler(&path) {
                    vec.extend(paths);
                }
            } else {
                vec.push(path);
            }
        }
        Ok(vec)
    }
}
