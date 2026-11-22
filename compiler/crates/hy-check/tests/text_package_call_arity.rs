//! `strings.map` is excluded: its keyword spelling is already rejected by the parser.
//! Fixed-arity strings calls must reject at check time, not Go build time.
use hy_check::{check_file, Verdict};
use hy_syntax::parse_file;

const CALLS: &[(&str, &str, &str)] = &[
    ("strings", "fields", "\"abc\""),
    ("strings", "title", "\"abc\""),
    ("strings", "to_lower", "\"abc\""),
    ("strings", "to_title", "\"abc\""),
    ("strings", "to_upper", "\"abc\""),
    ("strings", "trim_space", "\"abc\""),
    ("strings", "compare", "\"abc\", \"abc\""),
    ("strings", "contains", "\"abc\", \"abc\""),
    ("strings", "contains_any", "\"abc\", \"a\""),
    ("strings", "contains_rune", "\"abc\", 1"),
    ("strings", "count", "\"abc\", \"abc\""),
    ("strings", "cut", "\"abc\", \"abc\""),
    ("strings", "equal_fold", "\"abc\", \"abc\""),
    ("strings", "has_prefix", "\"abc\", \"abc\""),
    ("strings", "has_suffix", "\"abc\", \"abc\""),
    ("strings", "index", "\"abc\", \"abc\""),
    ("strings", "index_any", "\"abc\", \"a\""),
    ("strings", "index_byte", "\"abc\", 1"),
    ("strings", "index_rune", "\"abc\", 1"),
    ("strings", "join", "nil, \"abc\""),
    ("strings", "last_index", "\"abc\", \"abc\""),
    ("strings", "last_index_any", "\"abc\", \"a\""),
    ("strings", "last_index_byte", "\"abc\", 1"),
    ("strings", "repeat", "\"abc\", 1"),
    ("strings", "replace_all", "\"abc\", \"abc\", \"abc\""),
    ("strings", "split", "\"abc\", \"abc\""),
    ("strings", "split_after", "\"abc\", \"abc\""),
    ("strings", "trim", "\"abc\", \"a\""),
    ("strings", "trim_left", "\"abc\", \"a\""),
    ("strings", "trim_prefix", "\"abc\", \"abc\""),
    ("strings", "trim_right", "\"abc\", \"a\""),
    ("strings", "trim_suffix", "\"abc\", \"abc\""),
    ("strings", "split_n", "\"abc\", \"abc\", 2"),
    ("strings", "split_after_n", "\"abc\", \"abc\", 2"),
    ("strings", "replace", "\"abc\", \"abc\", \"abc\", 2"),
    ("strings", "clone", "\"a\""),
    ("strings", "new_reader", "\"a\""),
    ("strings", "fields_func", "\"a\", nil"),
    ("strings", "contains_func", "\"a\", nil"),
    ("strings", "index_func", "\"a\", nil"),
    ("strings", "last_index_func", "\"a\", nil"),
    ("strings", "to_lower_special", "nil, \"a\""),
    ("strings", "to_title_special", "nil, \"a\""),
    ("strings", "to_upper_special", "nil, \"a\""),
    ("strings", "to_valid_utf8", "\"a\", \"b\""),
    ("strings", "trim_func", "\"a\", nil"),
    ("strings", "trim_left_func", "\"a\", nil"),
    ("strings", "trim_right_func", "\"a\", nil"),
    ("strings", "cut_prefix", "\"a\", \"b\""),
    ("strings", "cut_suffix", "\"a\", \"b\""),
];

fn check(pkg: &str, body: &str) -> hy_check::CheckResult {
    let src = format!("import \"{pkg}\"\nfn main() {{\n{body}\n}}\n");
    let ast = parse_file("text-calls.hy", &src).unwrap_or_else(|d| panic!("{src}: {d:?}"));
    check_file(&ast)
}

#[test]
fn every_curated_fixed_arity_call_accepts_its_go_signature() {
    for (pkg, member, args) in CALLS {
        let r = check(pkg, &format!("{pkg}.{member}({args})"));
        assert_eq!(
            r.verdict,
            Verdict::Accept,
            "{pkg}.{member}: {:?}",
            r.diagnostics
        );
    }
}

#[test]
fn every_curated_fixed_arity_call_rejects_missing_and_extra_arguments() {
    let mut missed = Vec::new();
    for (pkg, member, args) in CALLS {
        let short = args
            .rsplit_once(", ")
            .map(|(prefix, _)| prefix)
            .unwrap_or("");
        for args in [String::new(), short.to_string(), format!("{args}, 0")] {
            let r = check(pkg, &format!("{pkg}.{member}({args})"));
            if r.verdict != Verdict::Reject || !r.diagnostics.iter().any(|d| d.code == "HYB-E1002")
            {
                missed.push(format!("{pkg}.{member}({args}): {:?}", r.diagnostics));
            }
        }
    }
    assert!(missed.is_empty(), "missing E1002:\n{}", missed.join("\n"));
}

#[test]
fn every_curated_fixed_arity_call_rejects_variadic_spread() {
    let mut missed = Vec::new();
    for (pkg, member, args) in CALLS {
        let r = check(pkg, &format!("{pkg}.{member}({args}...)"));
        if r.verdict != Verdict::Reject || !r.diagnostics.iter().any(|d| d.code == "HYB-E1002") {
            missed.push(format!("{pkg}.{member}: {:?}", r.diagnostics));
        }
    }
    assert!(missed.is_empty(), "missing E1002:\n{}", missed.join("\n"));
}

#[test]
fn new_replacer_remains_variadic() {
    for args in ["", "\"a\", \"b\"", "[]string{\"a\", \"b\"}..."] {
        let r = check("strings", &format!("strings.new_replacer({args})"));
        assert_eq!(r.verdict, Verdict::Accept, "{args}: {:?}", r.diagnostics);
    }
}

#[test]
fn tuple_forwarding_uses_the_expanded_result_count() {
    let src = "import \"strings\"\nfn pair() (string, string) { return \"a\", \"b\" }\nfn main() { strings.compare(pair()) }";
    let ast = parse_file("tuple.hy", src).unwrap();
    let r = check_file(&ast);
    assert_eq!(r.verdict, Verdict::Accept, "{:?}", r.diagnostics);
    let src = src.replace("strings.compare", "strings.clone");
    let r = check_file(&parse_file("tuple.hy", &src).unwrap());
    assert_eq!(r.verdict, Verdict::Reject, "{:?}", r.diagnostics);
    assert!(r.diagnostics.iter().any(|d| d.code == "HYB-E1002"));
}

#[test]
fn local_receiver_named_strings_is_not_treated_as_the_package() {
    let src = "struct local {}\nfn (x local) compare() int { return 1 }\nfn main() { strings := local{}\n _ = strings.compare() }";
    let r = check_file(&parse_file("shadow.hy", src).unwrap());
    assert_eq!(r.verdict, Verdict::Accept, "{:?}", r.diagnostics);
}

#[test]
fn foreign_tuple_forwarding_keeps_valid_calls_and_rejects_wrong_counts() {
    let src =
        "import \"strings\"\nimport \"path\"\nfn main() { strings.compare(path.split(\"a/b\")) }";
    let r = check_file(&parse_file("foreign-tuple.hy", src).unwrap());
    assert_eq!(r.verdict, Verdict::Accept, "{:?}", r.diagnostics);
    let r = check("strings", "strings.compare(strings.cut(\"a\", \"b\"))");
    assert_eq!(r.verdict, Verdict::Reject, "{:?}", r.diagnostics);
    assert!(r.diagnostics.iter().any(|d| d.code == "HYB-E1002"));
}
