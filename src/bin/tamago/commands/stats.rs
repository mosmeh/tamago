use super::Command;
use std::{fs::File, io::BufReader, path::PathBuf};
use structopt::StructOpt;
use tamago::index::Index;

#[derive(StructOpt)]
pub struct StatsCommand {
    #[structopt(short, long)]
    index: PathBuf,
}

impl Command for StatsCommand {
    #[allow(unused_assignments)]
    fn run(self) -> anyhow::Result<()> {
        let _index: Index = {
            let reader = BufReader::new(File::open(&self.index)?);
            bincode::deserialize_from(reader)?
        };

        Ok(())
    }
}
