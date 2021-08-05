#![allow(clippy::write_with_newline)]

use crate::{
    index::{Index, SequenceId},
    mapper::single::SingleMapping,
    sequence::COMPLEMENT_TABLE,
};
use std::io;

pub fn write_header<W: io::Write>(mut out: W, pg: &str, index: &Index) -> io::Result<()> {
    out.write_all(b"@HD\tVN:1.0\tSO:unknown\n")?;

    for i in 0..index.num_seqs() {
        let id = SequenceId(i);
        out.write_all(b"@SQ\tSN:")?;
        out.write_all(index.seq_name(id))?;
        let range = index.seq_range(id);
        write!(out, "\tLN:{}\tDS:T\n", range.end - range.start)?;
    }

    write!(
        out,
        "@PG\tID:{}\tPN:{}\tVN:{}\n",
        pg,
        pg,
        env!("CARGO_PKG_VERSION")
    )
}

pub fn write_mapping_single<W: io::Write>(
    mut out: W,
    index: &Index,
    qname: &[u8],
    seq: &[u8],
    mapping: &SingleMapping,
    secondary: bool,
    rc_seq_cache: &mut Option<Vec<u8>>,
) -> io::Result<()> {
    let rname = index.seq_name(mapping.seq_id);

    let mut flag = 0;
    if mapping.strand.is_reverse() {
        flag |= 0x10;
    }
    if secondary {
        flag |= 0x100;
    }

    let seq: &[u8] = if mapping.strand.is_forward() {
        seq
    } else if let Some(rc) = rc_seq_cache {
        rc
    } else {
        *rc_seq_cache = Some(reverse_complement(seq));
        rc_seq_cache.as_ref().unwrap()
    };

    out.write_all(qname)?;
    write!(out, "\t{}\t", flag)?;
    out.write_all(rname)?;
    write!(out, "\t{}\t255\t{}M\t*\t0\t0\t", mapping.pos + 1, seq.len())?;
    out.write_all(seq)?;
    write!(out, "\t*\tAS:i:{}\n", mapping.score)
}

pub fn write_unmapped_single<W: io::Write>(mut out: W, qname: &[u8], seq: &[u8]) -> io::Result<()> {
    out.write_all(qname)?;
    out.write_all(b"\t4\t*\t0\t255\t*\t*\t0\t0\t")?;
    out.write_all(seq)?;
    out.write_all(b"\t*\tAS:i:0\n")
}

fn reverse_complement(seq: &[u8]) -> Vec<u8> {
    seq.iter()
        .rev()
        .map(|x| COMPLEMENT_TABLE[*x as usize])
        .collect()
}
