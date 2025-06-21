# nword

N-gram processor for tokenized text files.

## Usage

Build ngram database:
```bash
# get some dataset. for example opensubtitles
curl -LO https://object.pouta.csc.fi/OPUS-OpenSubtitles/v1/mono/sv.tok.gz && gunzip sv.tok.gz

# make the ngrams.
nword build sv.tok sv_ngrams/
```

Query n-grams by providing "seed" grams to stdin:
```bash
echo 'jag heter\nvem är' | ./target/release/nword --verbose=info query ./sv_ngrams >/dev/null
#>  0.002940626s  INFO nword: Options { prefix_mode: true, suffix_mode: false, freq_min: 4, max_depth: 2 }
#>  0.002985019s  INFO nword::query: Load ngrams
#>  0.103173488s  INFO nword::query: Loaded 126374 grams
#>  0.118134393s  INFO nword::query: exhausted after 17571 ngrams
#>  0.143877094s  INFO nword: Finished in 0.14s

# include less common ngrams at the expense of reading more into memory
echo 'jag heter\nvem är' | ./target/release/nword --verbose=info query ./sv_ngrams --freq-min=2 >/dev/null
#>  0.003292668s  INFO nword: Options { prefix_mode: true, suffix_mode: false, freq_min: 2, max_depth: 2 }
#>  0.003347151s  INFO nword::query: Load ngrams
#>  0.447670105s  INFO nword::query: Loaded 492406 grams
#>  0.504129311s  INFO nword::query: exhausted after 64896 ngrams
#>  0.620174039s  INFO nword: Finished in 0.62s

# note: per-seed order is arbitrary, because HashMap entry order.
echo 'jag heter\nvem är' | ./target/release/nword query ./sv_ngrams --freq-min=2 | head -5
#> jag heter walter
#> jag heter lee
#> jag heter sherman
#> jag heter hank
#> jag heter allie

echo 'jag heter\nvem är' | ./target/release/nword query ./sv_ngrams --freq-min=2 | tail -5
#> vem är först på min
#> vem är först på plats
#> vem är först om den
#> vem är först om du
#> vem är först om en

echo 'karen' | ./target/release/nword  query ./sv_ngrams --suffix-mode | head -5
#> namn är karen
#> hans namn är karen
#> mammas namn är karen
#> riktiga namn är karen
#> mellan namn är karen
```

## Build

```bash
cargo build --release
```

Input: tokenized text files, one sentence per line.
Output: frequency-sorted tab-separated files (1grams.txt, 2grams.txt, etc.).
