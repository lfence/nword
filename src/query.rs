use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use tracing::{debug, info};

#[derive(Debug)]
pub struct Options {
    pub prefix_mode: bool,
    pub suffix_mode: bool,
    // skip infrequent ngrams. (faster trie building, fewer results)
    pub freq_min: u32,
    // how many recursions we make
    pub max_depth: u32,
}

pub trait NgramIndex {
    fn insert(&mut self, ngram: &str, freq: u32);
    fn lookup(&self, prefix: &str) -> Vec<(String, u32)>;
}

pub struct TrieNode {
    children: HashMap<String, TrieNode>,
    freq: u32,
}

impl Default for TrieNode {
    fn default() -> Self {
        TrieNode {
            children: HashMap::default(),
            freq: 0,
        }
    }
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
    fn from_it<I: Iterator<Item = String>>(it: I, freq_min: u32) -> TrieNode {
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

pub struct NgramStream<'a> {
    trie: &'a TrieNode,
    queue: Vec<(String, u32)>,
    pending: Vec<(String, u32)>,
    max_depth: u32,
}

impl<'a> NgramStream<'a> {
    fn new(trie: &'a TrieNode, seed: String, max_depth: u32) -> Self {
        NgramStream {
            trie,
            queue: vec![(seed, 0)],
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
        Box::new(NgramStream::new(trie, seed.to_string(), max_depth))
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
            seeds
                .into_iter()
                .flat_map(move |good_seed: String| NgramStream::new(trie, good_seed, max_depth)),
        )
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
        let suffix_mode_transform = |x: String| {
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
        for ngram in stream_ngrams(&ngram3_suffix, &query, opts.max_depth) {
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
