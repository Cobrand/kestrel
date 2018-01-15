use fnv::FnvHashMap as HashMap;
use std::collections::VecDeque;
use itertools::Itertools;

use fragment::{Fragment, build_data_from_fragments};

#[derive(Debug)]
pub (crate) struct FragmentCombiner {
    pending_fragments: HashMap<u32, HashMap<u8, Fragment<Box<[u8]>>>>,
    out_messages: VecDeque<Box<[u8]>>,
}

impl FragmentCombiner {
    pub fn new() -> Self {
        FragmentCombiner {
            pending_fragments: HashMap::default(),
            out_messages: VecDeque::new(),
        }
    }

    /// Removes the HashMap for key `seq_id`, an tries to create a message out of that.
    ///
    /// Panics if there is no HashMap at `seq_id`
    ///
    /// Returns an Error if all the fragments do not have the same frag_total,
    /// or if "build_message_from_fragments" encountered an error
    fn transform_message(&mut self, seq_id: u32) -> Result<(), ()> {
        let fragments = self.pending_fragments.remove(&seq_id).unwrap();
        if !fragments.values().map(|f| f.frag_total).all_equal() {
            // some fragments don't have the same frag_total
            return Err(());
        }
        // build_data_from_fragments with an IntoIterator with just the values
        let message = build_data_from_fragments(fragments.into_iter().map(|(_k, v)| v))?;
        self.out_messages.push_back(message);
        Ok(())
    }

    pub fn next_out_message(&mut self) -> Option<Box<[u8]>> {
        self.out_messages.pop_front()
    }

    /// Returns all the waiting out messages, and empties the internal queue
    pub fn out_messages(&mut self) -> VecDeque<Box<[u8]>> {
        let empty = VecDeque::default();
        if self.out_messages.len() == 0 {
            empty
        } else {
            ::std::mem::replace(&mut self.out_messages, empty)
        }
    }

    pub fn push(&mut self, fragment: Fragment<Box<[u8]>>) {
        let seq_id = fragment.seq_id;
        let frag_total = fragment.frag_total;

        let try_transform = { 
            let entry = self.pending_fragments.entry(seq_id);
            let seq_hash_map = entry.or_insert_with(|| HashMap::with_capacity_and_hasher(frag_total as usize, Default::default()));
            // if the seq_id/frag_id combo already existed, override it. It can happen when the sender re-sends a packet we've already received
            // because it didn't receive the ack on time.
            seq_hash_map.insert(fragment.frag_id, fragment);
            if seq_hash_map.len() == frag_total as usize + 1 {
                true
                // try to transform fragments into a message, because we have enough of them here
            } else {
                false
            }
        };

        if try_transform {
            let _r = self.transform_message(seq_id);
            // failures to transform messages are ignored. Logging may be an option here TODO
        }
    }
}

#[test]
fn fragment_combiner_success() {
    let fragments: Vec<Fragment<Box<[u8]>>> = vec![
        Fragment { seq_id: 3, frag_id: 1, frag_total: 2, data: Box::new([0, 5]) },
        Fragment { seq_id: 4, frag_id: 1, frag_total: 2, data: Box::new([4, 0]) },
        Fragment { seq_id: 7, frag_id: 0, frag_total: 0, data: Box::new([64, 64]) },
        Fragment { seq_id: 5, frag_id: 1, frag_total: 2, data: Box::new([4, 5]) },
        Fragment { seq_id: 5, frag_id: 0, frag_total: 2, data: Box::new([1, 2, 3]) },
        Fragment { seq_id: 5, frag_id: 2, frag_total: 2, data: Box::new([6, 7, 8, 9]) },
        Fragment { seq_id: 6, frag_id: 1, frag_total: 2, data: Box::new([14, 5]) },
    ];
    let mut fragment_combiner = FragmentCombiner::new();
    for fragment in fragments {
        fragment_combiner.push(fragment);
    }

    let out_message = fragment_combiner.next_out_message().unwrap();
    assert_eq!(out_message.as_ref(), &[64, 64]);
    let out_message = fragment_combiner.next_out_message().unwrap();
    assert_eq!(out_message.as_ref(), &[1, 2, 3, 4, 5, 6, 7, 8, 9]);
}

pub (crate) struct FragmentGenerator<'a, I> where I: Iterator<Item = &'a [u8]> + Clone {
    seq_id: u32,
    frag_total: u8,
    next_frag: u8,
    iterator: I
}

impl<'a, I> FragmentGenerator<'a, I> where I: Iterator<Item = &'a [u8]> + Clone {
    pub fn new(iterator: I, seq_id: u32, frag_total: u8) -> Self {
        FragmentGenerator {
            seq_id,
            frag_total,
            iterator,
            next_frag: 0,
        }
    }
}

impl<'a, I: Iterator<Item = &'a [u8]> + Clone> Iterator for FragmentGenerator<'a, I> {
    type Item = Fragment<&'a [u8]>;
    fn next(&mut self) -> Option<Self::Item> {
        let data = self.iterator.next();
        match data {
            Some(data) => {
                let frag = Fragment {
                    seq_id: self.seq_id,
                    frag_total: self.frag_total,
                    frag_id: self.next_frag,
                    data: data
                };
                self.next_frag += 1;
                Some(frag)
            },
            None => None
        }
    }
}

impl<'a, I: Iterator<Item = &'a [u8]> + Clone> Clone for FragmentGenerator<'a, I> {
    fn clone(&self) -> Self {
        FragmentGenerator {
            seq_id: self.seq_id,
            next_frag: self.next_frag,
            frag_total: self.frag_total,
            iterator: self.iterator.clone(),
        }
    }
}