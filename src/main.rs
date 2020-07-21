use dump_parser::Page;
use dump_parser::{wiktionary_configuration, Node, Positioned};
use smartstring::alias::String;
use std::{
    collections::HashMap,
    fs::File,
    io::{BufReader, BufWriter, Write},
    path::Path,
};
use unicase::UniCase;

mod error;
use error::{Error, Result};

// type LanguagesToEntries<'a> = HashMap<&'a String, Vec<UniCase<String>>>;

struct LanguagesToEntries<'a>(HashMap<&'a str, Vec<UniCase<String>>>);

impl<'a> LanguagesToEntries<'a> {
    #[inline(always)]
    fn new() -> Self {
        Self(HashMap::new())
    }

    #[inline(always)]
    fn push(&mut self, language_code: &'a str, title: &UniCase<String>) {
        self.0
            .entry(language_code)
            .or_insert_with(Vec::new)
            .push(title.clone());
    }
}

impl<'a> IntoIterator for LanguagesToEntries<'a> {
    type Item = (&'a str, Vec<UniCase<String>>);
    type IntoIter =
        <HashMap<&'a str, Vec<UniCase<String>>> as IntoIterator>::IntoIter;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

fn check_output_dir(path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();
    std::fs::create_dir_all(&path)
        .map_err(|e| Error::io_error("create output directory", path, e))?;
    Ok(())
}

fn get_language_name_to_code(
    path: impl AsRef<Path>,
) -> Result<HashMap<String, String>> {
    let path = path.as_ref();
    let language_name_to_code = std::fs::read_to_string(path)
        .map_err(|e| Error::io_error("read from", path, e))?;
    language_name_to_code
        .lines()
        .enumerate()
        .filter(|(_, line)| *line != "")
        .map(|(i, line)| {
            let mut splitter = line.splitn(2, '\t');
            let (first, second) = (splitter.next(), splitter.next());
            if let (Some(name), Some(code)) = (first, second) {
                Ok((name.into(), code.into()))
            } else {
                Err(Error::InvalidNameToCodeFormat {
                    path: path.into(),
                    line: line.into(),
                    line_number: i + 1,
                })
            }
        })
        .collect()
}

type Namespace = u32; // should be i32
const MAINSPACE: Namespace = 0;
const APPENDIX_NAMESPACE: Namespace = 100;
const RECONSTRUCTION_NAMESPACE: Namespace = 118;

fn make_entry_index<'a>(
    output_dir: impl AsRef<Path>,
    pages_articles_path: impl AsRef<Path>,
    language_name_to_code: &'a HashMap<String, String>,
) -> Result<LanguagesToEntries<'a>> {
    let output_dir = output_dir.as_ref();
    check_output_dir(output_dir)?;
    let pages_articles_path = pages_articles_path.as_ref();
    let dump_file = File::open(pages_articles_path)
        .map_err(|e| Error::io_error("open", pages_articles_path, e))?;
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
            let title = UniCase::new(String::from(title));
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
                        languages_to_entries.push(language_code, &title);
                    } else {
                        eprintln!(
                            "language name {} in {} not recognized",
                            language_name, &title
                        );
                    }
                }
            }
        } else if namespace == APPENDIX_NAMESPACE
            || namespace == RECONSTRUCTION_NAMESPACE
        {
            let title = UniCase::new(String::from(title));
            if let Some(Some(Some(language_code))) =
                title.split(':').nth(1).map(|title_after_namespace| {
                    title_after_namespace.split('/').next().map(
                        |language_name| {
                            language_name_to_code.get(language_name)
                        },
                    )
                })
            {
                languages_to_entries.push(language_code, &title);
            } else if namespace == RECONSTRUCTION_NAMESPACE {
                eprintln!(
                    "valid language name not found in reconstruction title {}",
                    title
                );
            }
        }
    }
    Ok(languages_to_entries)
}

fn print_entries(
    languages_to_entries: LanguagesToEntries,
    output_dir: impl AsRef<Path>,
) -> Result<()> {
    let output_dir = output_dir.as_ref();
    for (language_code, mut entries) in languages_to_entries {
        let mut path = output_dir.join(language_code);
        path.set_extension("txt");
        let output_file = File::create(&path)
            .map_err(|e| Error::io_error("create", &path, e))?;
        entries.sort();
        let mut output_file = BufWriter::new(output_file);
        for entry in entries {
            writeln!(output_file, "{}", entry)
                .map_err(|e| Error::io_error("write to", &path, e))?;
        }
    }
    Ok(())
}

fn main_with_result(
    output_dir: impl AsRef<Path>,
    pages_articles_path: impl AsRef<Path>,
    language_name_to_code_path: impl AsRef<Path>,
) -> Result<()> {
    let output_dir: &Path = output_dir.as_ref();
    let language_name_to_code =
        get_language_name_to_code(language_name_to_code_path)?;
    let languages_to_entries = make_entry_index(
        output_dir,
        pages_articles_path,
        &language_name_to_code,
    )?;
    print_entries(languages_to_entries, output_dir)?;
    Ok(())
}

fn main() {
    main_with_result("entries", "pages-articles.xml", "name_to_code.txt")
        .unwrap_or_else(|e| {
            eprintln!("{}", e);
            std::process::exit(1);
        });
}
