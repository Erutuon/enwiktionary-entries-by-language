# entries-by-language

Makes an index of English Wiktionary entry names for each language. Requires the decompressed `pages-articles.xml` dump file and a TSV file (`name_to_code.txt`) in the format `(<name> '\t' <code> '\n')+` where `<name>` is the Wiktionary language name (used in the language header) and `<code>` is the Wiktionary language code. Creates an `entries` directory in the current working directory with a `<code>.txt` file for each language listing all the entry names for that language, sorted case-insensitively.
