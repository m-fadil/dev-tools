use crate::tools::fake::rng::Rng;
use std::sync::LazyLock;

const WORDS_TEXT: &str = "lorem ipsum dolor sit amet consectetur adipiscing elit sed do eiusmod tempor incididunt ut labore et dolore magna \
aliqua enim ad minim veniam quis nostrud exercitation ullamco laboris nisi aliquip ex ea commodo consequat duis \
aute irure in reprehenderit voluptate velit esse cillum fugiat nulla pariatur excepteur sint occaecat cupidatat \
non proident sunt culpa qui officia deserunt mollit anim id est laborum at vero eos accusamus iusto odio \
dignissimos ducimus blanditiis praesentium voluptatum deleniti atque corrupti quos dolores quas molestias \
excepturi occaecati cupiditate provident similique mollitia animi perspiciatis unde omnis iste natus error \
voluptatem accusantium doloremque laudantium totam rem aperiam eaque ipsa quae ab illo inventore veritatis \
quasi architecto beatae vitae dicta explicabo nemo ipsam quia voluptas aspernatur aut odit fugit consequuntur \
magni dolores ratione sequi nesciunt neque porro quisquam";

static WORDS: LazyLock<Vec<&'static str>> = LazyLock::new(|| WORDS_TEXT.split(' ').collect());

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Unit {
    Words,
    Sentences,
    Paragraphs,
}

impl Unit {
    pub fn name(self) -> &'static str {
        match self {
            Unit::Words => "words",
            Unit::Sentences => "sentences",
            Unit::Paragraphs => "paragraphs",
        }
    }

    pub fn limit(self) -> i64 {
        match self {
            Unit::Words => LIMIT_WORDS,
            Unit::Sentences => LIMIT_SENTENCES,
            Unit::Paragraphs => LIMIT_PARAGRAPHS,
        }
    }
}

pub const LIMIT_WORDS: i64 = 5000;
pub const LIMIT_SENTENCES: i64 = 500;
pub const LIMIT_PARAGRAPHS: i64 = 100;

const CLASSIC_OPENING: [&str; 8] = [
    "lorem",
    "ipsum",
    "dolor",
    "sit",
    "amet,",
    "consectetur",
    "adipiscing",
    "elit",
];

fn capitalize(word: &str) -> String {
    let mut it = word.chars();
    match it.next() {
        Some(first) => first.to_uppercase().chain(it).collect(),
        None => String::new(),
    }
}

fn random_words(rng: &mut Rng, words: &[&'static str], n: usize) -> Vec<String> {
    let mut out: Vec<String> = Vec::with_capacity(n);
    for _ in 0..n {
        let mut word = *rng.pick(words);
        // avoid immediate repetition
        while out.last().is_some_and(|prev| prev == word) {
            word = *rng.pick(words);
        }
        out.push(word.to_string());
    }
    out
}

/// 5-15 words, capitalised, optional commas in the middle, ending with a period.
fn sentence(rng: &mut Rng, words_pool: &[&'static str], opening: Option<&[&str]>) -> String {
    let length = rng.rand_int(5, 15) as usize;
    let mut words: Vec<String> = opening
        .map(|o| o.iter().map(|s| s.to_string()).collect())
        .unwrap_or_default();
    let more = length.saturating_sub(words.len());
    words.extend(random_words(rng, words_pool, more));
    let mut i = opening.map_or(0, <[&str]>::len);
    while i + 1 < words.len() {
        if i >= 2 && !words[i].ends_with(',') && rng.next() < 0.12 {
            words[i].push(',');
        }
        if words[i].ends_with(',') && i + 1 < words.len() {
            i += 1;
        }
        i += 1;
    }
    format!("{}.", capitalize(&words.join(" ")))
}

fn paragraph(rng: &mut Rng, pool: &[&'static str], opening: Option<&[&str]>) -> String {
    let count = rng.rand_int(3, 7) as usize;
    (0..count)
        .map(|i| sentence(rng, pool, if i == 0 { opening } else { None }))
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn generate(rng: &mut Rng, unit: Unit, count: usize, classic: bool) -> String {
    let pool = &*WORDS;
    let opening: Option<&[&str]> = if classic { Some(&CLASSIC_OPENING) } else { None };
    match unit {
        Unit::Words => {
            let mut words: Vec<String> = opening
                .map(|o| o.iter().map(|s| s.to_string()).collect())
                .unwrap_or_default();
            let more = count.saturating_sub(words.len());
            words.extend(random_words(rng, pool, more));
            words.truncate(count);
            words.join(" ")
        }
        Unit::Sentences => (0..count)
            .map(|i| sentence(rng, pool, if i == 0 { opening } else { None }))
            .collect::<Vec<_>>()
            .join(" "),
        Unit::Paragraphs => (0..count)
            .map(|i| paragraph(rng, pool, if i == 0 { opening } else { None }))
            .collect::<Vec<_>>()
            .join("\n\n"),
    }
}
