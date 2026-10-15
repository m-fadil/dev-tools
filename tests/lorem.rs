mod common;
use common::dt;
use serde_json::Value;

#[test]
fn default_is_three_paragraphs() {
    let r = dt(&["lorem", "--seed", "1"]);
    assert_eq!(r.code, 0);
    let paragraphs: Vec<&str> = r.out.trim_end().split("\n\n").collect();
    assert_eq!(paragraphs.len(), 3);
    for p in paragraphs {
        let n = p.matches('.').count();
        assert!((3..=7).contains(&n), "{p}");
    }
}

#[test]
fn seed_is_reproducible() {
    let a = dt(&["lorem", "-p", "2", "--seed", "42"]).out;
    assert_eq!(dt(&["lorem", "-p", "2", "--seed", "42"]).out, a);
    assert_ne!(dt(&["lorem", "-p", "2", "--seed", "43"]).out, a);
}

#[test]
fn words_and_sentences() {
    assert_eq!(
        dt(&["lorem", "-w", "37", "--seed", "3"]).out.trim().split(' ').count(),
        37
    );
    let out = dt(&["lorem", "-s", "4", "--seed", "3"]).out;
    let s = out.trim();
    let sentences: Vec<&str> = s.split_inclusive('.').map(str::trim).collect();
    assert_eq!(sentences.len(), 4);
    for sentence in sentences {
        assert!(sentence.chars().next().unwrap().is_uppercase(), "{sentence}");
        assert!(sentence.ends_with('.'));
        let n = sentence.split(' ').count();
        assert!((5..=15).contains(&n), "{sentence}");
    }
}

#[test]
fn classic_opening() {
    let open = "Lorem ipsum dolor sit amet, consectetur adipiscing elit";
    assert!(dt(&["lorem", "-s", "2", "--classic"]).out.starts_with(open));
    assert!(dt(&["lorem", "-p", "1", "--classic"]).out.starts_with(open));
    assert_eq!(dt(&["lorem", "-w", "3", "--classic"]).out.trim(), "lorem ipsum dolor");
}

#[test]
fn maximum_word_count() {
    for extra in [vec![], vec!["--classic"]] {
        let mut args = vec!["lorem", "-w", "5000", "--seed", "1"];
        args.extend(extra);
        let r = dt(&args);
        assert_eq!(r.code, 0, "{}", r.err);
        assert_eq!(r.out.trim_end().split(' ').count(), 5000);
    }
}

#[test]
fn limits_enforced() {
    for args in [
        vec!["-w", "0"],
        vec!["-w", "5001"],
        vec!["-s", "501"],
        vec!["-p", "101"],
        vec!["-p", "x"],
        vec!["-w", "1", "-p", "1"],
        vec!["extra"],
        vec!["--seed", "-1"],
    ] {
        let mut a = vec!["lorem"];
        a.extend(&args);
        let r = dt(&a);
        assert_eq!(r.code, 2, "{args:?}");
        assert_eq!(r.out, "");
    }
}

#[test]
fn json_shape() {
    let v: Value = serde_json::from_str(&dt(&["lorem", "-s", "2", "--seed", "5", "--json"]).out).unwrap();
    let keys: Vec<&String> = v.as_object().unwrap().keys().collect();
    assert_eq!(keys, ["unit", "count", "text"]);
    assert_eq!(v["unit"], "sentences");
    assert_eq!(v["count"], 2);
    assert!(v["text"].is_string());
    let d: Value = serde_json::from_str(&dt(&["lorem", "--json"]).out).unwrap();
    assert_eq!(d["unit"], "paragraphs");
}

#[test]
fn seeded_output_matches_legacy_constants() {
    // Captured from legacy generate(mulberry32(seed), unit, count, classic).
    for (seed, words, sentences, paragraph) in [
        (
            "0",
            "duis lorem aliquip enim est blanditiis molestias similique id corrupti commodo incididunt",
            "Lorem aliquip enim est blanditiis molestias, similique. Culpa voluptatem mollitia illo occaecati, atque et ea inventore cillum tempor quae aut praesentium.",
            "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Enim est blanditiis molestias, similique id corrupti. Voluptatem mollitia illo, occaecati atque et ea inventore cillum. Incididunt totam dignissimos ullamco dolore cillum commodo pariatur.",
        ),
        (
            "1",
            "occaecati lorem dignissimos neque sequi irure molestias voluptatem officia quisquam id vero",
            "Lorem dignissimos neque sequi irure molestias voluptatem officia, quisquam id vero. In nostrud consectetur officia quos ab reprehenderit ullamco mollitia blanditiis rem exercitation voluptatem.",
            "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Neque sequi irure molestias voluptatem officia quisquam, id vero aliqua. In nostrud consectetur officia quos ab reprehenderit ullamco mollitia blanditiis rem exercitation voluptatem. Atque dolore duis adipiscing excepturi explicabo ea porro voluptatem, sunt voluptate, unde. Reprehenderit ipsam iusto consequuntur natus minim perspiciatis, dolores magna neque minim non et sunt. In aliqua mollitia praesentium cillum et aliqua sed voluptate, perspiciatis similique et, iusto dignissimos.",
        ),
        (
            "42",
            "quas anim vitae perspiciatis veniam dignissimos aute occaecati explicabo laborum commodo ipsam",
            "Anim vitae perspiciatis veniam dignissimos aute occaecati explicabo laborum, commodo ipsam. Architecto adipiscing dolores amet duis sed nostrud ipsa dignissimos, sit.",
            "Lorem ipsum dolor sit amet, consectetur adipiscing elit vitae. Veniam dignissimos aute occaecati explicabo, laborum commodo ipsam laudantium, voluptate exercitation accusamus. Duis sed nostrud, ipsa dignissimos. Beatae vero inventore, esse anim amet. Ea provident laboris voluptate laudantium dicta iusto, ullamco irure reprehenderit do. Magna labore dolor excepteur at molestias voluptas eiusmod voluptatem consequuntur irure natus quis odio veritatis.",
        ),
        (
            "4294967295",
            "voluptas nostrud voluptatem magni beatae blanditiis unde laborum magna porro nemo nostrud",
            "Nostrud voluptatem magni beatae blanditiis, unde laborum magna porro nemo nostrud nulla laudantium, laborum. Iusto aute voluptate dolor id commodo.",
            "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Magni beatae blanditiis unde laborum, magna porro nemo nostrud nulla laudantium laborum. Labore tempor iusto aute voluptate dolor id, commodo iusto deserunt laboris proident blanditiis quas. Ratione nemo porro excepturi officia non. Sunt voluptas minim ipsam nisi amet perspiciatis deleniti accusantium ex anim. Voluptas qui similique id at laboris ex consequuntur duis ad nemo aliquip eiusmod quas. Quasi culpa odio, doloremque ipsa eos dolores accusantium sint.",
        ),
    ] {
        for (args, expected) in [
            (vec!["lorem", "-w", "12", "--seed", seed], words),
            (vec!["lorem", "-s", "2", "--seed", seed], sentences),
            (vec!["lorem", "-p", "1", "--classic", "--seed", seed], paragraph),
        ] {
            let result = dt(&args);
            assert_eq!(result.code, 0, "{}", result.err);
            assert_eq!(result.out, format!("{expected}\n"), "{args:?}");
        }
    }
}
