use crate::sequence;
use serde::{Deserialize, Serialize};
use std::ops::Range;
use sufsort_rs::sufsort::SA;

#[derive(Serialize, Deserialize)]
pub struct Fringed {
    array: Vec<u32>,
    offsets: Vec<u32>,
    k: usize,
    l: usize,
}

impl Fringed {
    pub fn new(text: &[u8], l: usize) -> Self {
        assert!(text.len() <= u32::MAX as usize + 1);

        let k = l + 16;

        let sa = SA::<i32>::new(text);
        let array: Vec<_> = sa.sarray.into_iter().map(|x| x as u32).collect();

        let offsets_len = 1 << (2 * l);
        let mut left_to_indices = vec![sorted_list::SortedList::new(); offsets_len];
        for s in array {
            if s as usize + k > text.len() {
                continue;
            }
            let seq = &text[s as usize..][..k];
            if seq
                .iter()
                .any(|x| *x == 0 || *x == crate::sequence::DUMMY_CODE)
            {
                continue;
            }

            let mut left = 0;
            let mut right = 0;
            for (j, x) in seq[..l].iter().enumerate() {
                left |= (sequence::code_to_two_bit(*x) as u32) << (2 * j);
            }
            for (j, x) in seq[l..k].iter().enumerate() {
                right |= (sequence::code_to_two_bit(*x) as u32) << (2 * j);
            }
            left_to_indices[left as usize].insert(right, s);
        }

        let mut left_to_right_counts = Vec::new();
        for indices in &left_to_indices {
            let mut prev = None;
            let mut count = 0;
            for (i, _) in indices.iter() {
                if prev.is_none() || prev.unwrap() != i {
                    count += 1;
                    prev = Some(i);
                }
            }
            left_to_right_counts.push(count);
        }

        let mut offsets = Vec::new();
        let mut offset_start = 0;
        for i in 0..offsets_len {
            offsets.push(offset_start);
            offset_start += left_to_indices[i].len() as u32 + left_to_right_counts[i] * 2;
        }
        offsets.push(offset_start);

        let mut ssa = vec![0u32; offset_start as usize];
        for i in 0..offsets_len {
            let right_count = left_to_right_counts[i] as usize;
            if right_count == 0 {
                continue;
            }

            let mut prev = *left_to_indices[i].iter().next().unwrap().0;
            let mut z = offsets[i] as usize;
            let mut pos = z + 2 * right_count;

            ssa[z] = pos as u32;
            ssa[z + right_count] = prev;
            z += 1;

            for (right, s) in left_to_indices[i].iter() {
                if prev != *right {
                    ssa[z] = pos as u32;
                    ssa[z + right_count] = *right;
                    z += 1;
                    prev = *right;
                }
                ssa[pos] = *s;
                pos += 1;
            }
        }

        Self {
            array: ssa,
            offsets,
            k,
            l,
        }
    }
}

impl super::SuffixArrayVariant for Fringed {
    fn index_to_pos(&self, index: usize) -> usize {
        self.array[index] as usize
    }

    fn extension_search(
        &self,
        text: &[u8],
        query: &[u8],
        min_len: usize,
        max_hits: usize,
    ) -> Option<(Range<usize>, usize)> {
        let mut left = 0;
        for (j, x) in query[..self.l].iter().enumerate() {
            left |= (sequence::code_to_two_bit(*x) as u32) << (2 * j);
        }
        let section_begin = self.offsets[left as usize];
        let section_end = self.offsets[left as usize + 1];
        if section_begin == section_end {
            return None;
        }

        let head_begin = section_begin;
        let head_end = self.array[section_begin as usize];
        let num_rights = (head_end - head_begin) / 2;
        let right_begin = (head_begin + num_rights) as usize;
        let right_end = head_end as usize;

        let mut right = 0;
        for (j, x) in query[self.l..self.k].iter().enumerate() {
            right |= (sequence::code_to_two_bit(*x) as u32) << (2 * j);
        }
        let idx = if let Ok(i) = self.array[right_begin..right_end].binary_search(&right) {
            i
        } else {
            return None;
        };

        let mut begin = self.array[head_begin as usize + idx] as usize;
        let mut end = if head_begin as usize + idx + 1 == right_begin {
            section_end
        } else {
            self.array[head_begin as usize + idx + 1]
        } as usize;

        unsafe {
            super::equal_range(
                &self.array,
                text.as_ptr().add(self.k),
                query.as_ptr().add(self.k),
                query.as_ptr().add(min_len),
                &mut begin,
                &mut end,
            );
        }
        if begin == end {
            return None;
        }

        let mut depth = min_len;
        let query_len = query.len();
        while depth < query_len && end - begin > max_hits {
            unsafe {
                super::equal_range(
                    &self.array,
                    text.as_ptr().add(depth),
                    query.as_ptr().add(depth),
                    query.as_ptr().add(depth + 1),
                    &mut begin,
                    &mut end,
                );
            }

            if begin == end {
                return None;
            }
            depth += 1;
        }

        if depth == query_len && end - begin > max_hits {
            None
        } else {
            Some((begin..end, depth))
        }
    }

    fn bucket_size_distribution(&self) -> Option<std::collections::BTreeMap<usize, usize>> {
        None
    }
}
