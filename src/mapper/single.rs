use super::{Mapper, Strand};
use crate::{index::SequenceId, sequence};

pub struct SingleMapping {
    pub seq_id: SequenceId,
    pub pos: usize,
    pub strand: Strand,
    pub score: i32,
}

impl Mapper<'_> {
    pub fn map_single(&self, query: &[u8]) -> Vec<SingleMapping> {
        if query.len() < self.seed_min_len {
            // TODO
            return Vec::new();
        }

        let rc_query = sequence::reverse_complement(query);

        // seeding
        let ref_to_anchors = self.search_anchors(query, &rc_query, true);
        if ref_to_anchors.is_empty() {
            return Vec::new();
        }

        let mut mappings = Vec::new();
        for ((seq_id, strand), anchors) in ref_to_anchors {
            mappings.push(SingleMapping {
                seq_id,
                pos: anchors[0].ref_pos,
                strand,
                score: 0,
            });
        }

        mappings
    }
}
