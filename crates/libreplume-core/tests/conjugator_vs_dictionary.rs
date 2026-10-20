//! Cross-checks the French conjugator against the Hunspell dictionary.
//! Run with `cargo test --release -- --ignored --nocapture`.

use libreplume_core::conjugation::fr::SimpleTense;
use libreplume_core::resources::french;

#[test]
#[ignore = "slow: validates every dictionary verb"]
fn generated_forms_are_known_words() {
    let t = std::time::Instant::now();
    let fr = french();
    eprintln!(
        "resources loaded in {:?}: {} verbs, {} nominal forms",
        t.elapsed(),
        fr.verbs.verbs.len(),
        fr.lexicon.forms.len()
    );
    let mut total = 0usize;
    let mut bad = Vec::new();
    for id in 0..fr.verbs.verbs.len() as u32 {
        let Some(f) = fr.verbs.simple_forms(id) else { continue };
        for t in SimpleTense::ALL {
            for p in 0..6 {
                if let Some(form) = f.get(t, p) {
                    total += 1;
                    if !fr.speller.check(form) {
                        bad.push(format!("{} {:?}{} {}", f.infinitive, t, p, form));
                    }
                }
            }
        }
    }
    let mut verbs: Vec<&str> = bad.iter().map(|b| b.split(' ').next().unwrap()).collect();
    verbs.dedup();
    eprintln!("{} / {} forms unknown to the dictionary ({} verbs)", bad.len(), total, verbs.len());
    for b in bad.iter().step_by((bad.len() / 60).max(1)) {
        eprintln!("  {b}");
    }
    assert!((bad.len() as f64) / (total as f64) < 0.02);
}
