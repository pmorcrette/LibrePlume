# Thesauri

Compact copies of two MyThes thesauri, generated with `scripts/build-thesaurus.py`
(one line per meaning: headword, part of speech, synonyms, antonyms; synonyms ranked
by centrality). Regenerate them with that script when updating the sources.

| File | Source | Licence |
| --- | --- | --- |
| `fr.tsv` | LibreOffice French thesaurus `thes_fr.dat` (Debian/Ubuntu package `mythes-fr` 26.8.0.3), by Kevin Hendricks, Frédéric Labbé and Laurent Godard | LGPL 2.1 or later ([`LGPL-2.1.txt`](LGPL-2.1.txt)) |
| `en.tsv` | LibreOffice/OpenOffice English thesaurus `th_en_US_v2.dat`, derived from WordNet 2.1 (Princeton University) | WordNet licence ([`WORDNET-LICENSE.txt`](WORDNET-LICENSE.txt)) |

WordNet 2.1 Copyright 2005 by Princeton University. All rights reserved.

The conversion keeps the content of the sources; it drops WordNet « generic term »
entries (broader words rather than synonyms), separates antonyms and reorders synonyms.
