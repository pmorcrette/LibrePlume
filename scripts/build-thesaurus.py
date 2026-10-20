#!/usr/bin/env python3
"""Converts a MyThes thesaurus (LibreOffice .dat file) into LibrePlume's compact format.

    scripts/build-thesaurus.py fr thes_fr.dat crates/libreplume-core/thesaurus/fr.tsv
    scripts/build-thesaurus.py en th_en_US_v2.dat crates/libreplume-core/thesaurus/en.tsv

Output: one line per meaning, sorted by headword:

    headword <TAB> part of speech <TAB> synonym|synonym|… <TAB> antonym|antonym|…

English (WordNet) annotations are interpreted: « generic term » entries are broader
words, not synonyms, and are dropped; « similar term » and « related term » are kept
after the plain synonyms; « antonym » goes to the last column.

Synonyms are ranked by centrality: a synonym that shares many synonyms with the
headword comes first (« difficile » → malaisé, pénible… before abrupt, escarpé), which
puts the most common sense first when the source lists senses in arbitrary order.
"""

import re
import sys

POS = {
    "noun": "noun", "nom": "noun", "adj": "adjective", "adjectif": "adjective",
    "verb": "verb", "verbe": "verb", "adv": "adverb", "adverbe": "adverb",
    "interjection": "interjection", "pronom": "pronoun", "preposition": "preposition",
    "préposition": "preposition", "conjonction": "conjunction", "locution": "phrase",
}
TAG = re.compile(r"\s*\((generic term|similar term|related term|antonym)\)\s*$")


def pos_label(raw):
    words = [POS.get(w.lower(), "") for w in raw.strip("()").split()]
    return " ".join(dict.fromkeys(w for w in words if w))


def clean(field):
    field = field.replace("\t", " ").replace("|", " ").strip()
    return re.sub(r"\s+", " ", field)


def convert(src, dst):
    with open(src, encoding="utf-8", errors="replace") as f:
        lines = f.read().split("\n")
    out = []
    i = 1  # first line is the encoding
    while i < len(lines):
        head = lines[i]
        i += 1
        if "|" not in head:
            continue
        word, count = head.rsplit("|", 1)
        try:
            count = int(count)
        except ValueError:
            continue
        word = clean(word)
        for line in lines[i:i + count]:
            parts = line.split("|")
            plain, similar, antonyms = [], [], []
            for p in parts[1:]:
                m = TAG.search(p)
                term = clean(TAG.sub("", p))
                if not term or term.lower() == word.lower():
                    continue
                kind = m.group(1) if m else None
                if kind == "generic term":
                    continue
                if kind == "antonym":
                    antonyms.append(term)
                elif kind in ("similar term", "related term"):
                    similar.append(term)
                else:
                    plain.append(term)
            plain = list(dict.fromkeys(plain))
            similar = [t for t in dict.fromkeys(similar) if t not in plain]
            antonyms = list(dict.fromkeys(antonyms))
            if plain or similar or antonyms:
                out.append([word, pos_label(parts[0]), plain, similar, antonyms])
        i += count

    neighbours = {}
    for word, _, plain, similar, _ in out:
        neighbours.setdefault(word, set()).update(plain, similar)

    def ranked(word, terms):
        mine = neighbours.get(word, set())
        # sorted() is stable: ties keep the source order.
        return sorted(terms, key=lambda t: -len(mine & neighbours.get(t, set())))

    out = [
        (word, pos, "|".join(ranked(word, plain) + ranked(word, similar)), "|".join(antonyms))
        for word, pos, plain, similar, antonyms in out
    ]
    out.sort(key=lambda r: (r[0].lower(), r[0]))
    with open(dst, "w", encoding="utf-8") as f:
        for row in out:
            f.write("\t".join(row) + "\n")
    print(f"{dst}: {len(out)} meanings, {len({r[0] for r in out})} headwords")


if __name__ == "__main__":
    if len(sys.argv) != 4:
        sys.exit(__doc__)
    convert(sys.argv[2], sys.argv[3])
