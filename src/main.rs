use clap::{self, Parser};
use codespan_reporting::diagnostic::{Diagnostic, Label};
use codespan_reporting::files::SimpleFile;
use codespan_reporting::term;
use codespan_reporting::term::termcolor::{ColorChoice, StandardStream};
use std::{fs, ops::Range, path::Path, process};
use unicode_width::UnicodeWidthStr;

const SECOND: u32 = 1000;
const MINUTE: u32 = SECOND * 60;
const HOUR: u32 = MINUTE * 60;
const ARROW_SEPARATOR: &str = " --> ";

/// Adjust the timestamps in an SRT file
#[derive(Parser)]
#[command(about)]
struct Arguments {
    /// The SRT file to adjust
    file: String,
    /// The change in time
    adjustment: String,
    /// The output file (default: same as input file)
    #[arg(short, long)]
    output: Option<String>,
}

fn main() {
    let args = Arguments::parse();

    let adjustment = match parse_time(&args.adjustment.replace(&['+', '-'], ""), 0) {
        Ok(adjustment) => adjustment,
        Err(err) => {
            eprintln!("{}", err.message);
            process::exit(1);
        }
    };
    let neg = args.adjustment.starts_with('-');

    let path = Path::new(&args.file);
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(_) => {
            eprintln!("Failed to read the input file {}", path.display());
            process::exit(1);
        }
    };
    let text = &text.replace(&['\r', '\u{feff}'], "");

    let mut subtitles = parse_srt(&text).unwrap_or_else(|error| {
        let file = SimpleFile::new(path.file_name().unwrap().to_str().unwrap(), text);

        let diagnostic = Diagnostic::error()
            .with_message(error.message)
            .with_labels(vec![
                Label::primary((), error.range).with_message(error.reason)
            ]);

        let writer = StandardStream::stderr(ColorChoice::Always);
        let config = codespan_reporting::term::Config::default();

        term::emit(&mut writer.lock(), &config, &file, &diagnostic).unwrap();
        process::exit(1);
    });

    for subtitle in subtitles.iter_mut() {
        if neg {
            subtitle.from -= adjustment;
            subtitle.to -= adjustment;
        } else {
            subtitle.from += adjustment;
            subtitle.to += adjustment;
        }
    }

    let output = print_subtitles(&subtitles);

    let write_result = fs::write(args.output.unwrap_or(args.file.clone()), output);
    if write_result.is_err() {
        eprintln!("Failed to write the output file {}", path.display());
        process::exit(1);
    }
}

struct Subtitle<'a> {
    number: u32,
    from: u32, // millis
    to: u32,   // millis
    lines: Vec<&'a str>,
}

struct ParseError {
    message: String,
    reason: String,
    range: Range<usize>,
}

fn parse_srt(text: &str) -> Result<Vec<Subtitle>, ParseError> {
    let mut subtitles: Vec<Subtitle> = vec![];
    let mut lines_iter = text.lines();
    let mut index = 0usize;
    while let Some(number_line) = lines_iter.next() {
        let number = match number_line.parse::<u32>() {
            Ok(number) => number,
            Err(_) => {
                return Err(ParseError {
                    message: format!("Failed to parse {:?} as an integer", number_line),
                    reason: "Invalid subtitle number".into(),
                    range: Range {
                        start: index,
                        end: index + number_line.width(),
                    },
                });
            }
        };
        index += number_line.width() + 1;

        let time_line = match lines_iter.next() {
            Some(time_line) => time_line,
            None => {
                return Err(ParseError {
                    message: format!("Expected to find time line for subtitle {}", number_line),
                    reason: "Missing time line".into(),
                    range: Range {
                        start: index,
                        end: index,
                    },
                })
            }
        };

        let (from_text, to_text) = match time_line.split_once(ARROW_SEPARATOR) {
            Some((from_text, to_text)) => (from_text, to_text),
            None => {
                return Err(ParseError {
                    message: format!(
                        "Expected to find arrow in time line for subtitle {}",
                        number_line
                    ),
                    reason: format!("Missing '{}'", ARROW_SEPARATOR),
                    range: Range {
                        start: index,
                        end: index + time_line.width(),
                    },
                })
            }
        };

        let from = parse_time(from_text, index)?;
        let to = parse_time(to_text, index + from_text.width() + ARROW_SEPARATOR.width())?;

        index += time_line.len() + 1;

        let mut lines: Vec<&str> = vec![];
        while let Some(line) = lines_iter.next() {
            index += line.width() + 1;
            if line.is_empty() {
                break;
            }
            lines.push(line);
        }

        subtitles.push(Subtitle {
            number,
            from,
            to,
            lines,
        })
    }

    Ok(subtitles)
}

fn parse_time(text: &str, index: usize) -> Result<u32, ParseError> {
    let mut number_strs: Vec<&str> = text.splitn(3, ':').collect();
    number_strs.reverse();
    let mut number_strs_iter = number_strs.iter();

    let mut seconds = 0;
    let mut millis = 0;
    if let Some(seconds_str) = number_strs_iter.next() {
        let (seconds_str, millis_str) = seconds_str.split_once(',').unwrap_or((seconds_str, "0"));
        seconds = match seconds_str.parse::<u32>() {
            Ok(seconds) => seconds,
            Err(_) => {
                return Err(ParseError {
                    message: format!("Failed to parse {} as an integer", seconds_str),
                    reason: "Invalid seconds".into(),
                    range: Range {
                        start: index,
                        end: index + text.width(),
                    },
                })
            }
        };
        millis = match millis_str.parse::<u32>() {
            Ok(millis) => millis,
            Err(_) => {
                return Err(ParseError {
                    message: format!("Failed to parse {} as an integer", millis_str),
                    reason: "Invalid millis".into(),
                    range: Range {
                        start: index,
                        end: index + text.width(),
                    },
                })
            }
        }
    }

    let mut minutes = 0;
    if let Some(minutes_str) = number_strs_iter.next() {
        minutes = match minutes_str.parse::<u32>() {
            Ok(minutes) => minutes,
            Err(_) => {
                return Err(ParseError {
                    message: format!("Failed to parse {} as an integer", minutes_str),
                    reason: "Invalid minutes".into(),
                    range: Range {
                        start: index,
                        end: index + text.width(),
                    },
                })
            }
        }
    }

    let mut hours = 0;
    if let Some(hours_str) = number_strs_iter.next() {
        hours = match hours_str.parse::<u32>() {
            Ok(hours) => hours,
            Err(_) => {
                return Err(ParseError {
                    message: format!("Failed to parse {} as an integer", hours_str),
                    reason: "Invalid hours".into(),
                    range: Range {
                        start: index,
                        end: index + text.width(),
                    },
                })
            }
        }
    }

    Ok(hours * HOUR + minutes * MINUTE + seconds * SECOND + millis)
}

fn print_subtitles(subtitles: &[Subtitle]) -> String {
    let mut text = String::new();
    for subtitle in subtitles.iter() {
        let string = print_subtitle(subtitle);
        text.push_str(&string);
        text.push('\n');
    }
    text
}

fn print_subtitle(subtitle: &Subtitle) -> String {
    let mut text = String::new();
    text.push_str(&format!("{}\n", subtitle.number));
    text.push_str(&format!(
        "{} --> {}\n",
        print_time(subtitle.from),
        print_time(subtitle.to)
    ));
    for line in subtitle.lines.iter() {
        text.push_str(&format!("{}\n", line));
    }
    text
}

fn print_time(millis: u32) -> String {
    let hours = millis / HOUR;
    let mut leftover = millis - hours * HOUR;
    let minutes = leftover / MINUTE;
    leftover -= minutes * MINUTE;
    let seconds = leftover / SECOND;
    leftover -= seconds * SECOND;
    format!(
        "{:0>2}:{:0>2}:{:0>2},{:0>3}",
        hours, minutes, seconds, leftover
    )
}
