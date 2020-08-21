use dump_parser::{
    wiktionary_configuration, Namespace, Node, Page, Positioned,
};
use std::{
    fmt::Display,
    fs::File,
    io::{BufReader, BufWriter, Write},
    path::Path,
    result::Result as StdResult,
};

mod error;
mod types;

use error::{Error, Result};
use getopts::{Fail, Options};
use types::{LanguageNameToCode, LanguagesToEntries};

macro_rules! log {
    ($($arg:expr),* $(,)?) => {
        eprintln!($($arg),*)
    };
}

fn make_entry_index(
    pages_articles_path: &Path,
    language_name_to_code: &LanguageNameToCode,
) -> Result<LanguagesToEntries> {
    let dump_file = File::open(pages_articles_path)
        .map_err(|e| Error::from_io(e, "open", pages_articles_path))?;
    let configuration = wiktionary_configuration();
    let dump_file = BufReader::new(dump_file);
    let mut languages_to_entries = LanguagesToEntries::new();
    for parse_result in dump_parser::parse(dump_file) {
        let Page {
            title,
            text,
            namespace,
            ..
        } = parse_result?;
        if namespace == Namespace::Main {
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
                            language_name,
                            &title
                        );
                    }
                }
            }
        } else if namespace == Namespace::Appendix
            || namespace == Namespace::Reconstruction
        {
            if let Some(language_code) = title
                .split(':')
                .nth(1)
                .and_then(|title_after_namespace| {
                    title_after_namespace.split('/').next()
                })
                .and_then(|language_name| {
                    language_name_to_code.get(language_name)
                })
            {
                languages_to_entries.push(*language_code, &title);
            } else if namespace == Namespace::Reconstruction {
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

enum Args {
    Help {
        program: String,
        options: Options,
    },
    Parse {
        language_name_to_code_path: String,
        pages_articles_path: String,
        entries_dir: String,
    },
}

enum ArgParseError {
    Fail(Fail),
    MissingArgs(Vec<&'static str>),
    MissingProgram,
}

impl Display for ArgParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArgParseError::Fail(e) => e.fmt(f),
            ArgParseError::MissingArgs(args) => {
                write!(f, "missing arguments: {}", args.join(", "))
            }
            ArgParseError::MissingProgram => write!(f, "missing program name"),
        }
    }
}

impl From<Fail> for ArgParseError {
    fn from(e: Fail) -> Self {
        ArgParseError::Fail(e)
    }
}

macro_rules! get_multiple_opts {
    (
        $options:ident
        $enum:ident :: $variant:ident {
            $(
                $opt:literal => $name:ident
            ),+
            $(,)?
        }
    ) => {
        let mut missing_options = Vec::new();
        $(
            let $name = $options.opt_str($opt);
            if $name.is_none() {
                missing_options.push($opt);
            };
        )+
        if let ($(Some($name)),+) = ($($name),+) {
            Ok($enum :: $variant {
                $($name),+
            })
        } else {
            Err(ArgParseError::MissingArgs(missing_options))
        }
    };
}

fn parse_args(
    args: impl IntoIterator<Item = String>,
) -> StdResult<Args, ArgParseError> {
    let mut options = Options::new();
    options.optopt(
        "l",
        "language-name-to-code",
        "file containing lines of language name, tab, language code",
        "FILE",
    );
    options.optopt("p", "pages-xml", "XML page dump file", "FILE");
    options.optopt("o", "output-dir", "output directory", "DIR");
    options.optflag("h", "help", "display this message");

    let mut args = args.into_iter();
    let program = args.next().ok_or(ArgParseError::MissingProgram)?;
    let args: Vec<_> = args.collect();

    let matches = options.parse(&args)?;
    if matches.opt_present("help") {
        Ok(Args::Help { program, options })
    } else {
        get_multiple_opts! {
            matches
            Args::Parse {
                "language-name-to-code" => language_name_to_code_path,
                "pages-xml" => pages_articles_path,
                "output-dir" => entries_dir,
            }
        }
    }
}

fn main() {
    let args = parse_args(std::env::args()).unwrap_or_else(|e| {
        eprintln!("{}", e);
        std::process::exit(1);
    });
    match args {
        Args::Help { program, options } => print!(
            "{}",
            options.usage(&format!("Usage: {} [options]", program))
        ),
        Args::Parse {
            language_name_to_code_path,
            pages_articles_path,
            entries_dir,
        } => {
            main_with_result(
                language_name_to_code_path,
                pages_articles_path,
                entries_dir,
            )
            .unwrap_or_else(|e| {
                eprintln!("{}", e);
                std::process::exit(1);
            });
        }
    }
}
