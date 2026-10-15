mod common;
use common::dt;

#[test]
fn ts_alias_dispatches_to_time_help_group() {
    // Every registered command is listed with its aliases in help.
    let r = dt(&["--help"]);
    assert!(r.out.contains("(alias: ts)"));
    assert!(r.out.contains("(alias: sv)"));
}

#[test]
fn names_and_aliases_unique() {
    let r = dt(&["--help"]);
    let mut names = std::collections::HashSet::new();
    let mut count = 0;
    for line in r.out.lines().filter(|l| l.starts_with("  ")) {
        let name = line.split_whitespace().next().unwrap();
        assert!(names.insert(name.to_string()), "duplicate {name}");
        count += 1;
        if let Some((_, rest)) = line.rsplit_once("(alias: ") {
            for a in rest.trim_end_matches(')').split(", ") {
                assert!(names.insert(a.to_string()), "duplicate {a}");
            }
        }
    }
    assert_eq!(count, 38);
}
