use std::collections::HashMap;
use regex::Regex;
use std::fs::{self};
use std::io::{BufWriter, Write};
use std::process;
use rayon::prelude::*;
use std::time::Instant;
use tracing::{info, debug};
type FrequencyNgram = HashMap<String, usize>;
const NMIN: usize = 1;
const NMAX: usize = 4;

pub fn run(input_file: &str, output_dir: &str) {
    let file_data = fs::read(input_file).unwrap_or_else(|err| {
        eprintln!("Error reading file '{}': {}", input_file, err);
        process::exit(1);
    });

    info!("Read {} bytes from {}", file_data.len(), input_file);
    fs::create_dir_all(output_dir).unwrap_or_else(|err| {
        eprintln!("Error creating output directory: {}", err);
        process::exit(1);
    });

    let num_threads = rayon::current_num_threads();
    let file_size = file_data.len();
    let chunk_size = file_size / num_threads;

    let ngram_chunks: Vec<Vec<FrequencyNgram>> = (0..num_threads)
        .into_par_iter()
        .map(|thread_id| {
            let file_offset = thread_id * chunk_size;
            const MARGIN_BYTES: usize = 50;
            let total_to_read = chunk_size + MARGIN_BYTES;
            let end_offset = std::cmp::min(file_offset + total_to_read, file_data.len());
            let chunk = &file_data[file_offset..end_offset];
            let text = String::from_utf8_lossy(chunk);

            // clean up opensubtitle tok files
            let re = Regex::new(r"\{\s*[^}]*\}").unwrap(); // Remove any {content}
            let re2 = Regex::new(r"<[^>]{1,3}>").unwrap(); // Remove any {content}
            // let re3 = Regex::new(r"^\d\d\d ").unwrap(); // Remove any {content}
            let re3 = Regex::new(r"\d\d\d\w? (\d\d : )+\d\d, \d\d\d\d? -- > (\d\d : )+\d\d, \d\d\d\d? ").unwrap();

            let mut t = Instant::now();
            let tokens: Vec<String> = re3.replace_all(&re2.replace_all(&re.replace_all(&text, ""), ""), "")
                .to_lowercase()
                .replace("\n- ", " ")
                .replace("\n... ", " ")
                .replace(" ...\n", " . ")
                .replace(" ,,,\n", " . ")
                .replace(" -\n", " . ")
                .replace("'\n", " ")
                .replace("\n' ", " ")

                // we could make this just " ". but this fixes impossible cases
                .replace('\n', " . ")
                .replace(" .. ", " ")
                .replace(" ...", "")
                .replace(" ' ", " ")
                .replace(", ", " ")
                .replace(": ", " ")
                .replace(" - ", " ")
                .replace("- ", " ")
                .replace(" \" ", " ")
                .replace("' s", "s") // joe' s -> joes
                .split_whitespace()
                .map(str::to_string)
                .collect();
            debug!("[{:02}] Tokenize {:.2}s", thread_id, t.elapsed().as_secs_f64());
            // long tokens, for example "affärsuppgörelse" could be split into two
            // tokens: "affärs" "uppgörelse"

            // maybe we dont really need to create all these different ngrams.........
            // maybe just a 3ngram is enough... -> 2words -> next.
            // or 4ngram -> 3 words -> next
            // also maybe we should make reverse ngrams too..!
            let mut ngrams: Vec<FrequencyNgram> = vec![FrequencyNgram::new(); 6];
            t = Instant::now();
            for i in NMIN..=NMAX {
                for window in tokens.windows(i) {
                    // Skip n-grams that contain sentence-ending punctuation
                    if window.iter().any(|token| [".", "?", "!"].contains(&token.as_str())){
                        continue;
                    }

                    let ngram = window.join(" ");
                    *ngrams[i].entry(ngram).or_insert(0) += 1;
                }
            }
            debug!("[{:02}] ngrams: {:.2}s", thread_id, t.elapsed().as_secs_f64());
            ngrams
        })
        .collect();

    let ngrams: Vec<Vec<(String, usize)>> = (NMIN..=NMAX).into_par_iter().map(|n| {
        let mut results = FrequencyNgram::new();

        let mut t = Instant::now();
        for chunk_ngrams in &ngram_chunks {
            for (ngram, count) in &chunk_ngrams[n] {
                *results.entry(ngram.clone()).or_insert(0) += count;
            }
        }
        debug!("Merge {}grams: {:.2}s", n, t.elapsed().as_secs_f64());

        t = Instant::now();

        let mut sorted_ngrams: Vec<(String, usize)> = results.into_iter().collect();
        sorted_ngrams.sort_by(|a, b| b.1.cmp(&a.1));
        debug!("Sort {}grams: {:.2}s", n, t.elapsed().as_secs_f64());
        return sorted_ngrams;
    }).collect();

    for (i, sorted_ngrams) in ngrams.iter().enumerate() {
        let n = i + NMIN;
        let t = Instant::now();
        let output_file = format!("{}/{}grams.txt", output_dir, n);
        let file = fs::File::create(&output_file).unwrap_or_else(|err| {
            eprintln!("Error creating file {}: {}", output_file, err);
            process::exit(1);
        });

        let mut writer = BufWriter::new(file);
        for (ngram, count) in sorted_ngrams {
            writeln!(writer, "{}\t{}", ngram, count).unwrap_or_else(|err| {
                eprintln!("Error writing to file: {}", err);
                process::exit(1);
            });
        }

        writer.flush().unwrap_or_else(|err| {
            eprintln!("Error flushing file: {}", err);
            process::exit(1);
        });

        debug!("Write {}: {:.2}s", output_file, t.elapsed().as_secs_f64());
        println!("Wrote {}", output_file);
    }
}
