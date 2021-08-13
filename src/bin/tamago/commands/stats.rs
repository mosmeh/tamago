use super::Command;
use std::{fs::File, io::BufReader, path::PathBuf};
use structopt::StructOpt;
use tamago::index::Index;

#[derive(StructOpt)]
enum Info {
    BucketSizeDistribution,
    IndexSize,
}

#[derive(StructOpt)]
pub struct StatsCommand {
    #[structopt(short, long)]
    index: PathBuf,

    #[structopt(subcommand)]
    info: Info,
}

impl Command for StatsCommand {
    #[allow(unused_assignments)]
    fn run(self) -> anyhow::Result<()> {
        let index: Index = {
            let reader = BufReader::new(File::open(&self.index)?);
            bincode::deserialize_from(reader)?
        };

        match self.info {
            Info::BucketSizeDistribution => {
                for (k, v) in index.sa.bucket_size_distribution().into_iter() {
                    println!("{}\t{}", k, v);
                }
            }
            Info::IndexSize => {
                println!("{}", index.size_bytes());
            }
        }

        Ok(())
    }
}
