use std::collections::HashMap;
use tracing::{debug, info};

pub trait NgramIndex {
    fn insert(&mut self, ngram: &str, freq: u32);
    fn lookup(&self, prefix: &str) -> Vec<(String, u32)>;
}

#[derive(Default)]
pub struct TrieNode {
    children: HashMap<String, TrieNode>,
    freq: u32,
}

impl TrieNode {
    fn collect_all(&self, prefix: &str, results: &mut Vec<(String, u32)>) {
        results.push((prefix.to_string(), self.freq));
        for (word, child) in &self.children {
            debug!("prefix: '{}'\tchild: '{}'", prefix, word);
            let new_prefix = if prefix.is_empty() {
                word.clone()
            } else {
                format!("{} {}", prefix, word)
            };
            child.collect_all(&new_prefix, results);
        }
    }
    pub fn from_it<I: Iterator<Item = String>>(it: I, freq_min: u32) -> TrieNode {
        info!("Load ngrams");
        let mut i = 0;
        let mut trie = TrieNode::default();
        for line in it {
            let (ngram, _freq) = line
                .split_once('\t')
                .expect("ngram line missing tab separator");
            let freq: u32 = _freq.parse().expect("Bad freq");
            if freq < freq_min {
                // ngrams are sorted by frequency, so we're done here.
                break;
            }
            trie.insert(ngram, freq);
            i += 1;
        }
        info!("Loaded {} grams", i);
        trie
    }
}

impl NgramIndex for TrieNode {
    fn insert(&mut self, ngram: &str, freq: u32) {
        let mut node = self;
        for word in ngram.split_whitespace() {
            node = node.children.entry(word.to_string()).or_default();
        }
        // The leaf node gets the frequency
        node.freq = freq;
    }

    fn lookup(&self, prefix: &str) -> Vec<(String, u32)> {
        let mut node = self;
        for word in prefix.split_whitespace() {
            match node.children.get(word) {
                Some(next) => node = next,
                None => return vec![],
            }
        }
        let mut results = vec![];
        node.collect_all(&prefix, &mut results);
        results[1..].to_vec()
    }
}
