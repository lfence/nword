use std::collections::HashSet;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use tracing::{debug, info};

use crate::trie::{TrieNode,NgramIndex};

#[derive(Debug)]
pub struct Options {
    pub prefix_mode: bool,
    pub suffix_mode: bool,
    // skip infrequent ngrams. (faster trie building, fewer results)
    pub freq_min: u32,
    // how many recursions we make
    pub max_depth: u32,
}

pub struct NgramStream<'a> {
    trie: &'a TrieNode,
    queue: Vec<(String, u32)>,
    pending: Vec<(String, u32)>,
    max_depth: u32,
}
impl<'a> NgramStream<'a> {
    fn new(trie: &'a TrieNode, max_depth: u32) -> Self {
        NgramStream {
            trie,
            queue: vec![],
            pending: vec![],
            max_depth,
        }
    }
}

impl Iterator for NgramStream<'_> {
    type Item = String;
    fn next(&mut self) -> Option<Self::Item> {
        while self.pending.is_empty() {
            if self.queue.is_empty() {
                return None;
            }
            // new search
            let (cur, depth) = self.queue.remove(0);
            let words: Vec<&str> = cur.split_whitespace().collect();

            // 2-gram to lookup a 3-gram candidates
            let prefix = words[words.len() - 2..].join(" ");
            debug!("lookup: '{}'", prefix);
            for (ngram, freq) in self.trie.lookup(&prefix) {
                let new = ngram.split_whitespace().last().expect("broken ngram");
                let next_prefix = format!("{} {}", cur, new);
                self.pending.push((next_prefix.clone(), freq));
                if depth < self.max_depth {
                    self.queue.push((next_prefix, depth + 1));
                }
            }
        }
        let (ngram, _freq) = self.pending.pop().expect("cant happen");
        Some(ngram)
    }
}

fn stream_ngrams<'a>(
    trie: &'a TrieNode,
    seed: &str,
    max_depth: u32,
) -> Box<dyn Iterator<Item = String> + 'a> {
    if seed.split(" ").count() >= 2 {
        let mut grams = NgramStream::new(trie, max_depth);
        grams.queue.push((seed.to_string(), 0));
        Box::new(grams)
    } else {
        // let prefix = format!("{} ", seed);
        let seeds: Vec<String> = trie
            .lookup(seed)
            .iter()
            .map(|(word, _)| {
                word.split_whitespace()
                    .take(2)
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .collect::<HashSet<_>>() // deduplicate here
            .into_iter()
            .collect();

        Box::new(
            // return all 2grams too
            seeds.clone().into_iter().chain(
            // then the 3grams for those
            seeds
                .into_iter()
                .flat_map(move |good_seed: String| {
                    // we're already one step down.
                    let mut grams = NgramStream::new(trie, max_depth);
                    grams.queue.push((good_seed, 1));
                    grams
                }),
        ))
    }
}

fn stream_file_lines(path: &str) -> impl Iterator<Item = String> {
    BufReader::new(File::open(path).expect("file open fail"))
        .lines()
        .map(|l| l.expect("file read fail"))
}

pub fn run(data_dir: &str, opts: Options) -> io::Result<()> {
    // normal
    let ngram3_path = format!("{}/3grams.txt", data_dir);
    let ngram3 = if opts.prefix_mode {
        // normal mode:
        TrieNode::from_it(stream_file_lines(&ngram3_path), opts.freq_min)
    } else {
        TrieNode::default()
    };
    let ngram3_suffix = if opts.suffix_mode {
        // we can give a suffix and it finds possible prefixes for it instead.
        let suffix_mode_transform = |x: String| -> String {
            let mut words: Vec<&str> = x.split_whitespace().collect();
            let len = words.len();
            words[..len - 1].reverse();
            words[..len - 1].join(" ") + "\t" + words[len - 1]
        };
        TrieNode::from_it(
            stream_file_lines(&ngram3_path).map(suffix_mode_transform),
            opts.freq_min,
        )
    } else {
        TrieNode::default()
    };

    let input = io::stdin().lock();
    let mut total: u64 = 0;
    for seed in input.lines().flatten() {
        let query = seed.trim().to_lowercase();
        if query.is_empty() {
            continue;
        }
        for ngram in stream_ngrams(&ngram3, &query, opts.max_depth) {
            total += 1;
            writeln!(io::stdout(), "{}", ngram)?;
        }

        let reverse_query = query.split_whitespace().rev().collect::<Vec<&str>>().join(" ");
        for ngram in stream_ngrams(&ngram3_suffix, &reverse_query, opts.max_depth) {
            total += 1;
            writeln!(
                io::stdout(),
                "{}",
                ngram
                    .split_whitespace()
                    .rev()
                    .collect::<Vec<&str>>()
                    .join(" ")
            )?;
        }
    }
    info!("exhausted after {} ngrams", total);
    Ok(())
}
