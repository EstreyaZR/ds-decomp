use std::path::PathBuf;

use anyhow::Result;
use clap::{Args, Subcommand};
use ds_decomp::analysis::graph::{Graph, GraphOptions};

use crate::util::io::read_dir;

#[derive(Args)]
pub struct GraphArgs {
    #[command(subcommand)]
    command: GraphCommand,
}

#[derive(Subcommand)]
pub enum GraphCommand {
    Graph(GraphComArgs),
}

impl GraphArgs {
    pub fn run(&self) -> Result<()> {
        match &self.command {
            GraphCommand::Graph(graph) => graph.run(),
        }
    }
}

#[derive(Args)]
pub struct GraphComArgs {
    #[arg(long, short = 'c')]
    pub config_path: PathBuf,
    #[arg(long, short = 'a')]
    pub asm_dir: PathBuf,
    #[arg(long, short = 'd')]
    pub dry_run: bool,

    /// Number of iterations to scan for trees
    #[arg(long, short = 'n', default_value_t = 10)]
    pub iterations: u8,

    /// Print more Infos
    #[arg(long)]
    pub debug: bool,
}

impl GraphComArgs {
    pub fn run(&self) -> Result<()> {
        let config_path = self.config_path.parent().unwrap();
        if let Ok(files) = Self::dir_crawler(&self.asm_dir) {
            //log::info!("{:#?}", nodes);
            let mut graph = Graph::from_files(GraphOptions { debug: self.debug }, files);
            graph.init_nodes();

            for _ in 0..self.iterations {
                graph.find_trees();
            }

            if self.dry_run {
                log::info!("Dry Run Complete");
                return Ok(());
            } else {
                graph.to_file(config_path).expect("Error writing the tree files");
                log::info!("Wrote Files");
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
