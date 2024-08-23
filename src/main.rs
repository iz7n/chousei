use clap::{self, Parser};
use std::{fs, path::Path, process};

const SECOND: u32 = 1000;
const MINUTE: u32 = SECOND * 60;
const HOUR: u32 = MINUTE * 60;

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

fn main() -> Result<(), String> {
    let args = Arguments::parse();

    let adjustment = parse_time(&args.adjustment.replace(&['+', '-'], ""))?;
    let neg = args.adjustment.starts_with('-');

    let path = Path::new(&args.file);
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(_) => {
            eprintln!("Failed to read the input file {}", path.display());
            process::exit(1);
        }
    };

    let mut subtitles = parse_srt(&text)?;

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

    Ok(())
}

struct Subtitle<'a> {
    number: &'a str,
    from: u32, // millis
    to: u32,   // millis
    lines: Vec<&'a str>,
}

fn parse_srt(text: &str) -> Result<Vec<Subtitle>, String> {
    let mut subtitles: Vec<Subtitle> = vec![];
    let mut lines_iter = text.lines();
    while let Some(number_line) = lines_iter.next() {
        let time_line = match lines_iter.next() {
            Some(time_line) => time_line,
            None => return Err(format!("Failed to find time line for line {}", number_line)),
        };
        let (from_text, to_text) = match time_line.split_once(" --> ") {
            Some((from_text, to_text)) => (from_text, to_text),
            None => {
                return Err(format!(
                    "Time line for line {} did not contain ' --> '",
                    number_line
                ))
            }
        };
        let from = parse_time(from_text)?;
        let to = parse_time(to_text)?;

        let mut lines: Vec<&str> = vec![];
        while let Some(line) = lines_iter.next() {
            if line.is_empty() {
                break;
            }
            lines.push(line);
        }

        subtitles.push(Subtitle {
            number: number_line,
            from,
            to,
            lines,
        })
    }

    Ok(subtitles)
}

fn parse_time(text: &str) -> Result<u32, String> {
    let mut number_strs: Vec<&str> = text.splitn(3, ':').collect();
    number_strs.reverse();
    let mut number_strs_iter = number_strs.iter();

    let mut seconds = 0;
    let mut millis = 0;
    if let Some(seconds_str) = number_strs_iter.next() {
        let (seconds_str, millis_str) = seconds_str.split_once(',').unwrap_or((seconds_str, "0"));
        seconds = match seconds_str.parse::<u32>() {
            Ok(seconds) => seconds,
            Err(_) => return Err(format!("Failed to parse {} as an integer", seconds_str)),
        };
        millis = match millis_str.parse::<u32>() {
            Ok(millis) => millis,
            Err(_) => return Err(format!("Failed to parse {} as an integer", millis_str)),
        }
    }

    let mut minutes = 0;
    if let Some(minutes_str) = number_strs_iter.next() {
        minutes = match minutes_str.parse::<u32>() {
            Ok(minutes) => minutes,
            Err(_) => return Err(format!("Failed to parse {} as an integer", minutes_str)),
        }
    }

    let mut hours = 0;
    if let Some(hours_str) = number_strs_iter.next() {
        hours = match hours_str.parse::<u32>() {
            Ok(hours) => hours,
            Err(_) => return Err(format!("Failed to parse {} as an integer", hours_str)),
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
