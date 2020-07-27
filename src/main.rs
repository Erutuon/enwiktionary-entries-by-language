use dump_parser::Page;
use dump_parser::{wiktionary_configuration, Node, Positioned};
use std::{
    fs::File,
    io::{BufReader, BufWriter, Write},
    path::Path,
};

mod error;
mod types;

use error::{Error, Result};
use types::{LanguageNameToCode, LanguagesToEntries};

macro_rules! log {
    ($($arg:expr),* $(,)?) => {
        eprintln!($($arg),*)
    };
}

type Namespace = u32; // should be i32
const MAINSPACE: Namespace = 0;
const APPENDIX_NAMESPACE: Namespace = 100;
const RECONSTRUCTION_NAMESPACE: Namespace = 118;

fn make_entry_index(
    pages_articles_path: &Path,
    language_name_to_code: &LanguageNameToCode,
) -> Result<LanguagesToEntries> {
    let dump_file = File::open(pages_articles_path)
        .map_err(|e| Error::from_io(e, "open", pages_articles_path))?;
    let configuration = wiktionary_configuration();
    let dump_file = BufReader::new(dump_file);
    let mut languages_to_entries = LanguagesToEntries::new();
    for parse_result in parse_mediawiki_dump::parse(dump_file) {
        let Page {
            title,
            text,
            namespace,
            ..
        } = parse_result?;
        if namespace == MAINSPACE {
            // This only checks top-level header nodes.
            // We need to recurse if any level-2 headers are at lower levels.
            for node in configuration.parse(&text).nodes {
                if let Node::Heading {
                    nodes, level: 2, ..
                } = node
                {
                    let language_name = nodes.get_text_from(&text);
                    if let Some(language_code) =
                        language_name_to_code.get(language_name)
                    {
                        languages_to_entries.push(*language_code, &title);
                    } else {
                        log!(
                            "language name {} in {} not recognized",
                            language_name, &title
                        );
                    }
                }
            }
        } else if namespace == APPENDIX_NAMESPACE
            || namespace == RECONSTRUCTION_NAMESPACE
        {
            if let Some(Some(Some(language_code))) =
                title.split(':').nth(1).map(|title_after_namespace| {
                    title_after_namespace.split('/').next().map(
                        |language_name| {
                            language_name_to_code.get(language_name)
                        },
                    )
                })
            {
                languages_to_entries.push(*language_code, &title);
            } else if namespace == RECONSTRUCTION_NAMESPACE {
                log!("valid language name not found in title {}", title);
            }
        }
    }
    Ok(languages_to_entries)
}

fn print_entries(
    languages_to_entries: LanguagesToEntries,
    output_dir: &Path,
) -> Result<()> {
    for (language_code, mut entries) in languages_to_entries {
        let mut path = output_dir.join(language_code);
        path.set_extension("txt");
        let output_file = File::create(&path)
            .map_err(|e| Error::from_io(e, "create", &path))?;
        entries.sort();
        let mut output_file = BufWriter::new(output_file);
        for entry in entries {
            writeln!(output_file, "{}", entry)
                .map_err(|e| Error::from_io(e, "write to", &path))?;
        }
    }
    Ok(())
}

fn main_with_result<L: AsRef<Path>, P: AsRef<Path>, O: AsRef<Path>>(
    language_name_to_code_path: L,
    pages_articles_path: P,
    output_dir: O,
) -> Result<()> {
    let output_dir: &Path = output_dir.as_ref();
    std::fs::create_dir_all(output_dir).map_err(|e| {
        Error::from_io(e, "create output directory", output_dir)
    })?;
    let language_name_to_code =
        LanguageNameToCode::from_tsv_file(language_name_to_code_path.as_ref())?;
    let languages_to_entries =
        make_entry_index(pages_articles_path.as_ref(), &language_name_to_code)?;
    print_entries(languages_to_entries, output_dir)?;
    Ok(())
}

fn main() {
    main_with_result("name_to_code.txt", "pages-articles.xml", "entries")
        .unwrap_or_else(|e| {
            eprintln!("{}", e);
            std::process::exit(1);
        });
}
