//! Prints the issues found in each line of stdin: `cargo run --example check -- fr < text.txt`

use libreplume_core::{check, CheckOptions, Language};
use std::io::BufRead;

fn main() {
    let lang = std::env::args().nth(1).and_then(|l| l.parse::<Language>().ok());
    let opts = CheckOptions { language: lang, ..Default::default() };
    for line in std::io::stdin().lock().lines() {
        let line = line.expect("utf-8 input");
        if line.trim().is_empty() {
            continue;
        }
        let r = check(&line, &opts);
        println!("» {line}");
        for m in r.matches {
            println!(
                "    {:<26} {:?} {:?} → {:?}",
                m.rule_id,
                m.severity,
                m.text,
                m.replacements.iter().take(3).collect::<Vec<_>>()
            );
        }
    }
}
