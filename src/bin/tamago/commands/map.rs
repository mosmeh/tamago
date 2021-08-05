mod parallel;
mod serial;

use super::Command;
use anyhow::Result;
use std::{
    fs::File,
    io::{BufReader, Write},
    path::PathBuf,
    time::Instant,
};
use structopt::StructOpt;
use tamago::{
    index::Index,
    mapper::{LibraryType, Mapper, MapperBuilder},
    sequence,
};

#[derive(StructOpt, Debug)]
pub struct MapCommand {
    #[structopt(short, long)]
    index: PathBuf,

    #[structopt(short, long)]
    reads: PathBuf,

    #[structopt(short, long, default_value = "fr-unstranded")]
    library_type: LibraryType,

    #[structopt(short = "k", long, default_value = "31")]
    seed_min_len: usize,
    #[structopt(short, default_value = "1000")]
    multiplicity: usize,
    #[structopt(short, default_value = "1")]
    sparsity: usize,

    #[structopt(long)]
    header_sep: Option<String>,

    #[structopt(long, default_value = env!("CARGO_PKG_NAME"))]
    sam_pg: String,

    #[structopt(short, long, default_value = "1")]
    threads: usize,
    #[structopt(short, long, default_value = "1")]
    chunk: usize,
}

impl Command for MapCommand {
    fn run(self) -> Result<()> {
        eprintln!("{:#?}", self);

        eprintln!("Loading index");
        let index: Index = {
            let reader = BufReader::new(File::open(&self.index)?);
            bincode::deserialize_from(reader)?
        };

        let mapper = MapperBuilder::new(&index)
            .library_type(self.library_type)
            .seed_min_len(self.seed_min_len)
            .seed_max_hits(self.multiplicity)
            .sparsity(self.sparsity)
            .build();

        let start_time = Instant::now();

        if self.threads > 1 {
            parallel::main(self, &index, &mapper)?;
        } else {
            serial::main(self, &index, &mapper)?;
        }

        eprintln!("Elapsed(ms):{}", start_time.elapsed().as_millis());
        eprintln!("Finished");

        Ok(())
    }
}

fn map_single<'a, W: Write>(
    _out: W,
    _index: &Index,
    mapper: &Mapper<'a>,
    _qname: &[u8],
    seq: &[u8],
) -> Result<bool> {
    let encoded_seq = sequence::encode(&seq);

    let mappings = mapper.map_single(&encoded_seq);
    Ok(!mappings.is_empty())
}
