mod create_file;
mod graph;
mod rename;
mod set_size;
use anyhow::Result;
use clap::{Args, Subcommand};
pub use create_file::*;
pub use graph::*;
pub use rename::*;
pub use set_size::*;

#[derive(Args)]
pub struct EstreyaArgs {
    #[command(subcommand)]
    command: EstreyaCommand,
}

#[derive(Subcommand)]
pub enum EstreyaCommand {
    Rename(EstreyaComRenameArgs),
    Graph(EstreyaComGraphArgs),
    //EstreyaComSize(EstreyaComSizeArgs),
    //EstreyaComCreateFile(EstreyaComCreateFileArgs),
}

impl EstreyaArgs {
    pub fn run(&self) -> Result<()> {
        match &self.command {
            EstreyaCommand::Rename(rename) => rename.run(),
            EstreyaCommand::Graph(graph) => graph.run(),
            //EstreyaCommand::EstreyaComSize(size) => todo!(), //size.run(),
            //EstreyaCommand::EstreyaComCreateFile(create_file) => todo!(), //create_file.run(),
        }
    }
}
