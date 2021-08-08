use super::MapCommand;
use anyhow::{anyhow, Result};
use bio::io::fasta::{self, FastaRead};
use rayon::prelude::*;
use std::{
    io::{self, BufWriter, Write},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    thread,
};
use tamago::{index::Index, mapper::Mapper, utils};

struct Task {
    qname: Vec<u8>,
    seq: Vec<u8>,
}

pub fn main(config: MapCommand, index: &Index, mapper: &Mapper) -> Result<()> {
    rayon::ThreadPoolBuilder::new()
        .num_threads(config.threads)
        .build_global()?;

    let chunk_size = config.chunk * 1024 * 1024;

    let (writer_tx, writer_rx): (crossbeam_channel::Sender<Vec<u8>>, _) =
        crossbeam_channel::unbounded();
    let writer_thread = thread::spawn(move || -> Result<()> {
        let out = io::stdout();
        let mut writer = BufWriter::new(out.lock());
        for x in writer_rx.into_iter() {
            writer.write_all(&x)?;
        }
        Ok(())
    });

    let mut num_processed = 0;
    let num_mapped = Arc::new(AtomicUsize::new(0));

    eprintln!("Starting mapping");

    let mut reader = fasta::Reader::from_file(config.reads)?;
    let mut record = fasta::Record::new();
    reader.read(&mut record)?;

    while !record.is_empty() {
        let mut chunk = Vec::new();

        while !record.is_empty() && chunk.len() < chunk_size {
            record.check().map_err(|e| anyhow!(e.to_owned()))?;
            chunk.push(Task {
                qname: utils::extract_name_bytes(record.id(), &config.header_sep).to_owned(),
                seq: record.seq().to_owned(),
            });
            reader.read(&mut record)?;
        }

        chunk
            .par_iter()
            .try_for_each_with::<_, _, Result<()>>(writer_tx.clone(), |tx, task| {
                let mut buf = Vec::new();
                let mapped = super::map(&mut buf, index, mapper, &task.qname, &task.seq)?;
                if mapped {
                    num_mapped.fetch_add(1, Ordering::Relaxed);
                }
                tx.send(buf)?;
                Ok(())
            })?;

        num_processed += chunk.len();
    }

    eprintln!("Finishing output");
    drop(writer_tx);
    writer_thread.join().unwrap()?;

    let num_mapped = num_mapped.load(std::sync::atomic::Ordering::Relaxed);
    eprintln!(
        "Mapped {} / {} reads ({:.2}%)",
        num_mapped,
        num_processed,
        num_mapped as f64 * 100.0 / num_processed as f64
    );

    Ok(())
}
