use super::Command;
use std::{
    fs::File,
    io::{BufWriter, Write},
    path::PathBuf,
};
use structopt::StructOpt;
use tamago::index::{
    suffix_array::{HashFunc, SuffixArrayConfig},
    IndexBuilder,
};

#[derive(StructOpt, Debug)]
pub struct IndexCommand {
    #[structopt(short, long)]
    reference: PathBuf,
    #[structopt(short, long)]
    index: PathBuf,
    #[structopt(long)]
    header_sep: Option<String>,
    #[structopt(subcommand)]
    sa_opt: SuffixArrayOpt,
}

impl Command for IndexCommand {
    fn run(self) -> anyhow::Result<()> {
        eprintln!("{:#?}", self);

        let mut builder = IndexBuilder::from_file(self.reference)?.sa_config(self.sa_opt.into());
        if let Some(value) = self.header_sep {
            builder = builder.header_sep(value);
        }

        eprintln!("Indexing");
        let index = builder.build()?;

        eprintln!("Writing");
        let mut writer = BufWriter::new(File::create(&self.index)?);
        bincode::serialize_into(&mut writer, &index)?;
        writer.flush()?;

        Ok(())
    }
}

#[derive(StructOpt, Debug)]
enum SuffixArrayOpt {
    FixedLengthBuckets {
        #[structopt(short, long)]
        len: usize,
    },
    VariableLengthBuckets {
        #[structopt(short)]
        k: usize,
        #[structopt(short, default_value = "1")]
        f: f64,
    },
    Hashing {
        #[structopt(short)]
        k: usize,
        #[structopt(short, long)]
        bits: usize,
        #[structopt(short, long, default_value = "xxhash")]
        hash: HashFunc,
    },
    Fringed {
        #[structopt(short)]
        l: usize,
    },
}

impl From<SuffixArrayOpt> for SuffixArrayConfig {
    fn from(opt: SuffixArrayOpt) -> Self {
        match opt {
            SuffixArrayOpt::FixedLengthBuckets { len } => Self::FixedLengthBuckets { len },
            SuffixArrayOpt::VariableLengthBuckets { k, f } => Self::VariableLengthBuckets { k, f },
            SuffixArrayOpt::Hashing { k, bits, hash } => Self::Hashing {
                k,
                bits,
                hash_func: hash,
            },
            SuffixArrayOpt::Fringed { l } => Self::Fringed { l },
        }
    }
}
