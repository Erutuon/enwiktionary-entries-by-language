use dump_parser::Page;
use dump_parser::{wiktionary_configuration, Node, Positioned};
use smartstring::alias::String;
use std::{
    collections::HashMap,
    fs::File,
    hint::unreachable_unchecked,
    io::{BufReader, BufWriter, Write},
    path::Path,
    rc::Rc,
};
use unicase::UniCase;

mod error;
use error::{Error, Result};

macro_rules! exit_with_error {
    ($($tt:tt)*) => ({
        eprintln!($($tt)*);
        ::std::process::exit(-1)
    })
}

fn check_output_directory(path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();
    std::fs::create_dir_all(&path)
        .map_err(|e| Error::io_error("create output directory", path, e))?;
    Ok(())
}

fn get_language_name_to_code(
    path: impl AsRef<Path>,
) -> Result<HashMap<String, Rc<String>>> {
    let path = path.as_ref();
    let language_name_to_code = std::fs::read_to_string(path)
        .map_err(|e| Error::io_error("read from", path, e))?;
    language_name_to_code
        .lines()
        .enumerate()
        .filter(|(_, line)| *line != "")
        .map(|(i, line)| {
            let mut splitter = line.split('\t');
            match (splitter.next(), splitter.next()) {
                (Some(name), Some(code)) => {
                    if matches!(splitter.next(), None) {
                        Ok((name.into(), Rc::new(code.into())))
                    } else {
                        Err(Error::NotTwoTabs {
                            path: path.into(),
                            line_number: i + 1,
                            line: line.into(),
                            tabs: splitter.count() + 3,
                        })
                    }
                }
                (Some(_), None) => Err(Error::NotTwoTabs {
                    path: path.into(),
                    line: line.into(),
                    line_number: i + 1,
                    tabs: 1,
                }),
                (None, None) => Err(Error::NotTwoTabs {
                    path: path.into(),
                    line: line.into(),
                    line_number: i + 1,
                    tabs: 0,
                }),
                // `std::slice::Split` is a `std::iter::FusedIterator`,
                // which will not yield `Some(_)` after yielding `None`
                _ => unsafe { unreachable_unchecked() },
            }
        })
        .collect()
}

fn make_entry_index(
    output_directory: impl AsRef<Path>,
    pages_articles_path: impl AsRef<Path>,
    language_name_to_code_path: impl AsRef<Path>,
) -> Result<HashMap<Rc<String>, Vec<Rc<UniCase<String>>>>> {
    let output_directory = output_directory.as_ref();
    check_output_directory(output_directory)?;
    let pages_articles_path = pages_articles_path.as_ref();
    let dump_file = File::open(pages_articles_path)
        .map_err(|e| Error::io_error("open", pages_articles_path, e))?;
    let language_name_to_code =
        get_language_name_to_code(language_name_to_code_path)?;
    let configuration = wiktionary_configuration();
    let dump_file = BufReader::new(dump_file);
    let mut languages_to_entries =
        HashMap::<Rc<String>, Vec<Rc<UniCase<String>>>>::new();
    for parse_result in parse_mediawiki_dump::parse(dump_file) {
        if let Page {
            title,
            text,
            namespace: 0,
            ..
        } = parse_result?
        {
            let title = Rc::new(UniCase::new(title.into()));
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
                        languages_to_entries
                            .entry(Rc::clone(language_code))
                            .or_insert_with(Vec::new)
                            .push(Rc::clone(&title));
                    } else {
                        eprintln!(
                            "language name {} in {} not recognized",
                            language_name, &title
                        );
                    }
                }
            }
        }
    }
    Ok(languages_to_entries)
}

fn print_entries(
    languages_to_entries: HashMap<Rc<String>, Vec<Rc<UniCase<String>>>>,
    output_directory: impl AsRef<Path>,
) -> Result<()> {
    let output_directory = output_directory.as_ref();
    for (language_code, mut entries) in languages_to_entries {
        let mut path = output_directory.join(language_code.as_str());
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

fn main() {
    let output_directory: &Path = "entries".as_ref();
    let languages_to_entries = make_entry_index(
        output_directory,
        "pages-articles.xml",
        "name_to_code.txt",
    )
    .unwrap_or_else(|e| exit_with_error!("{}", e));
    print_entries(languages_to_entries, output_directory)
        .unwrap_or_else(|e| exit_with_error!("{}", e));
}
