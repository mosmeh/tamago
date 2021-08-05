use super::MapCommand;
use anyhow::{anyhow, Result};
use bio::io::fasta::{self, FastaRead};
use std::io::{self, BufWriter, Write};
use tamago::{index::Index, mapper::Mapper, utils};

pub fn main(config: MapCommand, index: &Index, mapper: &Mapper) -> Result<()> {
    let out = io::stdout();
    let mut out = BufWriter::new(out.lock());

    let mut num_processed = 0;
    let mut num_mapped = 0;

    eprintln!("Starting single-end mapping");

    let mut reader = fasta::Reader::from_file(&config.reads)?;
    let mut record = fasta::Record::new();
    reader.read(&mut record)?;

    while !record.is_empty() {
        record.check().map_err(|e| anyhow!(e.to_owned()))?;

        let qname = utils::extract_name_bytes(record.id(), &config.header_sep);
        let mapped = super::map_single(&mut out, index, mapper, qname, record.seq())?;

        if mapped {
            num_mapped += 1
        };
        num_processed += 1;

        reader.read(&mut record)?;
    }

    out.flush()?;

    eprintln!(
        "Mapped {} / {} reads ({:.2}%)",
        num_mapped,
        num_processed,
        num_mapped as f64 * 100.0 / num_processed as f64
    );

    Ok(())
}
