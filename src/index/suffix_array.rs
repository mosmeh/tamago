mod fixed_length_buckets;
mod fringed;
mod hashing;
mod variable_length_buckets;

use fixed_length_buckets::FixedLengthBuckets;
use fringed::Fringed;
use hashing::Hashing;
use variable_length_buckets::VariableLengthBuckets;

use serde::{Deserialize, Serialize};
use std::ops::Range;

pub enum SuffixArrayConfig {
    FixedLengthBuckets { len: usize },
    VariableLengthBuckets { k: usize, f: f64 },
    Hashing { k: usize, bits: usize },
    Fringed { l: usize },
}

trait SuffixArrayImpl {
    fn index_to_pos(&self, index: usize) -> usize;

    fn extension_search(
        &self,
        text: &[u8],
        query: &[u8],
        min_len: usize,
        max_hits: usize,
    ) -> Option<(Range<usize>, usize)>;
}

#[derive(Serialize, Deserialize)]
pub enum SuffixArray {
    FixedLengthBuckets(FixedLengthBuckets),
    VariableLengthBuckets(VariableLengthBuckets),
    Hashing(Hashing),
    Fringed(Fringed),
}

impl SuffixArray {
    pub fn new(text: &[u8], config: &SuffixArrayConfig) -> Self {
        match config {
            SuffixArrayConfig::FixedLengthBuckets { len } => {
                Self::FixedLengthBuckets(FixedLengthBuckets::new(text, *len))
            }
            SuffixArrayConfig::VariableLengthBuckets { k, f } => {
                Self::VariableLengthBuckets(VariableLengthBuckets::new(text, *k, *f))
            }
            SuffixArrayConfig::Hashing { k, bits } => Self::Hashing(Hashing::new(text, *k, *bits)),
            SuffixArrayConfig::Fringed { l } => Self::Fringed(Fringed::new(text, *l)),
        }
    }

    pub fn index_to_pos(&self, index: usize) -> usize {
        match self {
            Self::FixedLengthBuckets(sa) => sa.index_to_pos(index),
            Self::VariableLengthBuckets(sa) => sa.index_to_pos(index),
            Self::Hashing(sa) => sa.index_to_pos(index),
            Self::Fringed(sa) => sa.index_to_pos(index),
        }
    }

    pub fn extension_search(
        &self,
        text: &[u8],
        query: &[u8],
        min_len: usize,
        max_hits: usize,
    ) -> Option<(Range<usize>, usize)> {
        match self {
            Self::FixedLengthBuckets(sa) => sa.extension_search(text, query, min_len, max_hits),
            Self::VariableLengthBuckets(sa) => sa.extension_search(text, query, min_len, max_hits),
            Self::Hashing(sa) => sa.extension_search(text, query, min_len, max_hits),
            Self::Fringed(sa) => sa.extension_search(text, query, min_len, max_hits),
        }
    }
}

unsafe fn equal_range(
    sa: &[u32],
    text_base: *const u8,
    query_begin: *const u8,
    query_end: *const u8,
    begin: &mut usize,
    end: &mut usize,
) {
    let mut q_begin = query_begin;
    let mut q_end = q_begin;
    let mut t_begin = text_base;
    let mut t_end = t_begin;

    while begin < end {
        let mid = *begin + (*end as isize - *begin as isize) as usize / 2;
        let offset = sa[mid] as usize;
        let mut q;
        let mut t;
        if q_begin < q_end {
            q = q_begin;
            t = t_begin.add(offset);
        } else {
            q = q_end;
            t = t_end.add(offset);
        }
        let mut x;
        let mut y;
        loop {
            x = *t;
            y = *q;
            if x != y {
                break;
            };
            q = q.add(1);
            if q == query_end {
                *begin = lower_bound(sa, t_begin, q_begin, query_end, *begin, mid);
                *end = upper_bound(sa, t_end, q_end, query_end, mid + 1, *end);
                return;
            }
            t = t.add(1);
        }
        if x < y {
            *begin = mid + 1;
            q_begin = q;
            t_begin = t.sub(offset);
        } else {
            *end = mid;
            q_end = q;
            t_end = t.sub(offset);
        }
    }
}

unsafe fn lower_bound(
    sa: &[u32],
    mut text_base: *const u8,
    mut query_begin: *const u8,
    query_end: *const u8,
    mut begin: usize,
    mut end: usize,
) -> usize {
    while begin < end {
        let mid = begin + (end - begin) / 2;
        let offset = sa[mid];
        let mut t = text_base.offset(offset as isize);
        let mut q = query_begin;
        loop {
            if *t < *q {
                begin = mid + 1;
                query_begin = q;
                text_base = t.sub(offset as usize);
                break;
            }
            q = q.add(1);
            if q == query_end {
                end = mid;
                break;
            }
            t = t.add(1);
        }
    }
    begin
}

unsafe fn upper_bound(
    sa: &[u32],
    mut text_base: *const u8,
    mut query_begin: *const u8,
    query_end: *const u8,
    mut begin: usize,
    mut end: usize,
) -> usize {
    while begin < end {
        let mid = begin + (end - begin) / 2;
        let offset = sa[mid];
        let mut t = text_base.offset(offset as isize);
        let mut q = query_begin;
        loop {
            if *t > *q {
                end = mid;
                query_begin = q;
                text_base = t.sub(offset as usize);
                break;
            }
            q = q.add(1);
            if q == query_end {
                begin = mid + 1;
                break;
            }
            t = t.add(1);
        }
    }
    end
}
