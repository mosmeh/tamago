pub mod single;

use crate::index::{Index, SequenceId};
use rustc_hash::FxHashMap;

#[derive(Debug, Clone, Copy)]
pub enum LibraryType {
    Unstranded,
    FirstStrand,
    SecondStrand,
}

impl std::str::FromStr for LibraryType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, String> {
        match &s.to_lowercase()[..] {
            "fr-unstranded" => Ok(Self::Unstranded),
            "fr-firststrand" => Ok(Self::FirstStrand),
            "fr-secondstrand" => Ok(Self::SecondStrand),
            _ => Err(format!(
                "Unknown library type {}. \
            Valid values are: fr-unstranded, fr-firststrand, fr-secondstrand",
                s
            )),
        }
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Strand {
    Forward,
    Reverse,
}

impl Strand {
    pub fn is_forward(self) -> bool {
        matches!(self, Strand::Forward)
    }

    pub fn is_reverse(self) -> bool {
        matches!(self, Strand::Reverse)
    }

    pub fn opposite(self) -> Strand {
        match self {
            Self::Forward => Self::Reverse,
            Self::Reverse => Self::Forward,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Anchor {
    query_pos: usize,
    ref_pos: usize,
    len: usize,
}

pub struct Mapper<'a> {
    index: &'a Index,
    library_type: LibraryType,
    seed_min_len: usize,
    seed_max_hits: usize,
    sparsity: usize,
}

impl Mapper<'_> {
    fn search_anchors(
        &self,
        query: &[u8],
        rc_query: &[u8],
        is_read1: bool,
    ) -> FxHashMap<(SequenceId, Strand), Vec<Anchor>> {
        let mut ref_to_anchors: FxHashMap<(SequenceId, Strand), Vec<Anchor>> = FxHashMap::default();

        let mut seed = |query: &[u8], strand| {
            for seed_pos in (0..=(query.len() - self.seed_min_len)).step_by(self.sparsity) {
                let result = self.index.sa.extension_search(
                    &self.index.seq,
                    &query[seed_pos..],
                    self.seed_min_len,
                    self.seed_max_hits,
                );
                if let Some((range, len)) = result {
                    for i in range {
                        let pos = self.index.sa.index_to_pos(i);
                        let id = self.index.seq_id_from_pos(pos);

                        ref_to_anchors
                            .entry((id, strand))
                            .and_modify(|anchors| {
                                anchors.push(Anchor {
                                    query_pos: seed_pos,
                                    ref_pos: pos,
                                    len,
                                })
                            })
                            .or_insert_with(|| {
                                vec![Anchor {
                                    query_pos: seed_pos,
                                    ref_pos: pos,
                                    len,
                                }]
                            });
                    }
                }
            }
        };

        match (self.library_type, is_read1) {
            (LibraryType::Unstranded, _) => {
                seed(&query, Strand::Forward);
                seed(&rc_query, Strand::Reverse);
            }
            (LibraryType::FirstStrand, false) | (LibraryType::SecondStrand, true) => {
                seed(&query, Strand::Forward);
            }
            (LibraryType::SecondStrand, false) | (LibraryType::FirstStrand, true) => {
                seed(&rc_query, Strand::Reverse);
            }
        }

        ref_to_anchors
    }
}

pub struct MapperBuilder<'a> {
    index: &'a Index,
    library_type: LibraryType,
    seed_min_len: usize,
    seed_max_hits: usize,
    sparsity: usize,
}

impl<'a> MapperBuilder<'a> {
    pub fn new(index: &'a Index) -> Self {
        Self {
            index,
            library_type: LibraryType::Unstranded,
            seed_min_len: 31,
            seed_max_hits: 10,
            sparsity: 1,
        }
    }

    pub fn library_type(&mut self, library_type: LibraryType) -> &mut Self {
        self.library_type = library_type;
        self
    }

    pub fn seed_min_len(&mut self, seed_min_len: usize) -> &mut Self {
        self.seed_min_len = seed_min_len;
        self
    }

    pub fn seed_max_hits(&mut self, seed_max_hits: usize) -> &mut Self {
        self.seed_max_hits = seed_max_hits;
        self
    }

    pub fn sparsity(&mut self, sparsity: usize) -> &mut Self {
        self.sparsity = sparsity;
        self
    }

    pub fn build(&self) -> Mapper<'a> {
        Mapper {
            index: self.index,
            library_type: self.library_type,
            seed_min_len: self.seed_min_len,
            seed_max_hits: self.seed_max_hits,
            sparsity: self.sparsity,
        }
    }
}
