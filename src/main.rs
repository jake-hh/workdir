#![allow(dead_code, unused_variables)]

use clap::{arg, command, value_parser, Arg, Command, ColorChoice};
use clap::builder::styling::{Styles, Color, AnsiColor};
use colored::{Colorize, ColoredString, control};
use std::cmp::min;
use std::path::{Path, PathBuf};
use std::io::{self, Write};
use std::fs;
use std::fmt;

// File used to store a list of paths (working directories)
static PATH_FILE: &str = "~/.local/state/workdir";

// Max amount of stored paths (plus one)
const SAVED_PATHS_LIMIT_P1: usize = 20;

// Max amount of printed lines in short list
const SHORT_LIST_LINES_LIMIT: usize = 5;


pub enum Warn {
	InvalidLengthValue(usize, usize),
	LineWriteFailed(String, std::io::Error),
	CannotFlush(std::io::Error),
}

impl fmt::Display for Warn {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Warn::InvalidLengthValue(len, len_limit) => write!(f, "invalid value '{}' for '{}': only {} paths are saved", len.to_string().yellow(), "[length]".bold(), len_limit),
			Warn::LineWriteFailed(line, err) => write!(f, "failed writing line '{}' to path file {}", line.yellow(), attach_nested(err)),
			Warn::CannotFlush(err)   => write!(f, "cannot flush stderr stream {}", attach_nested(err)),
		}
	}
}

pub enum Error {
	InvalidPosValue(usize, usize),
	PathIsNotDir(String),
	IdenticalPathPos(String),
	PathLimitReached(),
	NoPathFile(),
	NoPathArg(),
	CannotCheckFile(std::io::Error),
	CannotOpenFile(std::io::Error),
	CannotReadFile(std::io::Error),
	CannotReadInput(String, std::io::Error),
}

impl fmt::Display for Error {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Error::InvalidPosValue(id, length) => write!(f, "invalid value '{}' for '{}': only {} paths are saved", id_to_pos(*id).to_string().yellow(), "[pos]".bold(), length),
			Error::PathIsNotDir(path)     => write!(f, "'{}' is not a directory", path.yellow()),
			Error::IdenticalPathPos(path) => write!(f, "path '{}' already exists on that position", path.yellow()),
			Error::PathLimitReached()     => write!(f, "limit of {} saved paths reached", SAVED_PATHS_LIMIT_P1.to_string().red()),
			Error::NoPathFile()           => write!(f, "path file doesn't exist: '{}'", PATH_FILE),
			Error::NoPathArg()            => write!(f, "current working path was not provided. Please use the included wrapper"),
			Error::CannotCheckFile(err)   => write!(f, "cannot check existence of path file {}", attach_nested(err)),
			Error::CannotOpenFile(err)    => write!(f, "cannot open path file {}", attach_nested(err)),
			Error::CannotReadFile(err)    => write!(f, "cannot read path file {}", attach_nested(err)),
			Error::CannotReadInput(input, err)   => write!(f, "cannot read line from stdin stream, failed at: '{}' {}", input, attach_nested(err)),
		}
	}
}

fn attach_nested(err: &std::io::Error) -> String {
	format!("\n\n{}", err.to_string().red())
}


fn main() {
	// Force colored output regardless of TTY
	control::set_override(true);

	let styles: Styles = Styles::styled()
		.header(AnsiColor::BrightGreen.on_default().bold().underline().underline_color(Some(Color::Ansi(AnsiColor::Black))))
		.usage(AnsiColor::BrightGreen.on_default().bold().underline().underline_color(Some(Color::Ansi(AnsiColor::Black))))
		.placeholder(AnsiColor::BrightBlack.on_default());

	// Parse specified CLI args
	let args = command!() 	// requires 'cargo' feature
		.about("workdir - fast 'working directory' switcher")
		.author("jake")
		.disable_version_flag(false)
		.disable_help_subcommand(false)
		.args_conflicts_with_subcommands(true)
		.color(ColorChoice::Always)
		.styles(styles)

		.subcommand(Command::new("list")
				.about("List recent paths")
				.aliases(["ls"])
				.arg(
					arg!([length] "Optional max list length")
					.value_parser(value_parser!(u8).range(1..SAVED_PATHS_LIMIT_P1 as i64))))

		.subcommand(Command::new("l")
				.about(format!("List {} recent paths", SHORT_LIST_LINES_LIMIT)))

		.subcommand(Command::new("restore")
				.about("Switch to selected path")
				.aliases(["r", "res"])
				.arg(
					arg!([pos] "Optional path position")
					.value_parser(value_parser!(u8).range(1..SAVED_PATHS_LIMIT_P1 as i64)))
				.arg(arg!(-v --verbose "Show verbose info")))

		.subcommand(Command::new("save")
				.about("Save path")
				.aliases(["s"])
				.arg(
					arg!(<path> "Current directory path (provided by wrapper function - do not enter)") //.hide(true)
					.value_parser(value_parser!(String)))
				.arg(
					arg!([pos] "Optional path position")
					.value_parser(value_parser!(u8).range(1..SAVED_PATHS_LIMIT_P1 as i64))))

		.subcommand(Command::new("delete")
				.about("Delete selected path")
				.aliases(["d", "del"])
				.arg(
					arg!(<pos> "Path position")
					.value_parser(value_parser![u8].range(1..SAVED_PATHS_LIMIT_P1 as i64))))

		.subcommand(Command::new("wrapper")
				.about("Dump wrapper function")
				.arg(
					arg!([shell] "Shell type")
					.value_parser(["sh", "bash", "zsh"])))

		.arg(Arg::new("pos")
			.help("Optional path position to restore")
			.value_parser(value_parser![u8].range(1..SAVED_PATHS_LIMIT_P1 as i64)))
		.arg(arg!(-v --verbose "Show verbose info"))

		.get_matches();

	// Match CLI args
	let result = match args.subcommand() {
		Some(("list", subargs))    => list(subargs.get_one::<u8>("length")),
		Some(("l", _))                          => l(),
		Some(("save", subargs))    => save(subargs.get_one::<String>("path"), subargs.get_one::<u8>("pos")),
		Some(("restore", subargs)) => restore(subargs.get_one::<u8>("pos"), subargs.get_flag("verbose")),
		Some(("delete", subargs))  => delete(subargs.get_one::<u8>("pos")),
		Some(("wrapper", subargs)) => dump_wrapper(subargs.get_one::<String>("shell")),
		_                          => restore(args.get_one::<u8>("pos"), args.get_flag("verbose"))
	};

	// if let Some(e) = result.err() {
	result.unwrap_or_else(|e| print_error(e));
}


// List paths
fn list(arg_length: Option<&u8>) -> Result<(), Error> {
	let lines = read_lines()?;
	let n_lines = lines.len();

	// Get number of printed lines from specified length arg, list all lines as default
	let mut n: usize = arg_length.map_or(n_lines, |&l| l as usize);

	// If length arg is bigger than number of lines => Throw error msg
	if n > n_lines {
		print_warning(Warn::InvalidLengthValue(n, n_lines));
		n = n_lines;
	}

	print_lines(lines, n);
	Ok(())
}

// Short list
fn l() -> Result<(), Error> {
	let lines = read_lines()?;

	// Get number of printed lines
	let n: usize = min(lines.len(), SHORT_LIST_LINES_LIMIT);

	print_lines(lines, n);
	Ok(())
}

// Save or move current PWD path in path list
fn save(arg_path: Option<&String>, arg_pos: Option<&u8>) -> Result<(), Error> {

	// Get new path id from pos arg, default to 0
	let id = arg_pos.map(pos_to_id()).unwrap_or(0);

	// Unwrap path
	let path: &String = arg_path.ok_or(Error::NoPathArg())?;

	// Check if path is a directory
	if !Path::new(path).is_dir() {
		return Err(Error::PathIsNotDir(path.clone()));
	}

	let mut lines = read_lines()?;
	let n_lines = lines.len();

	// Find existing id of path in lines list, if any
	let existing_id = lines.iter().position(|l| l == path);

	if let Some(ex_id) = existing_id {
		// Will be moving the path to new position

		// Check id
		if id >= n_lines {
			return get_invalid_pos_err(id, n_lines);
		}

		// If line already exists at that position => Throw error msg
		if ex_id == id {
			return Err(Error::IdenticalPathPos(path.clone()));
		}

		// Line already exists at a different position => Remove old path from list
		lines.remove(ex_id);
	}
	else {
		// Will be adding path to list

		// Check id
		if id > n_lines {
			return get_invalid_pos_err(id, n_lines);
		}

		// Check n_lines
		if n_lines >= SAVED_PATHS_LIMIT_P1 {
			return Err(Error::PathLimitReached());
		}
	}

	// Save new path in list
	lines.insert(id, path.clone());
	save_lines(lines)?;

	// Print confirmation message
	match existing_id {
		Some(rm_id) => print_ok("moved".cyan(), format!("{} -> {}", fmt_id(rm_id), fmt_path(id, path))),
		None => print_ok("saved".green(), fmt_path(id, path)),
	}

	Ok(())
}

// Change directory to one of saved paths
fn restore(arg_pos: Option<&u8>, arg_verbose: bool) -> Result<(), Error> {

	// Get path id from pos arg, default to 0
	let id = arg_pos.map(pos_to_id()).unwrap_or(0);

	let lines = read_lines()?;

	// Check id
	if id >= lines.len() {
		return get_invalid_pos_err(id, lines.len());
	}

	// Get selected line
	let line = &lines[id];

	// Check if path is a directory
	if !Path::new(line).is_dir() {
		print_error(Error::PathIsNotDir(line.to_string()));
		ask_to_remove(id, lines)?;
		return Ok(());
	}

	// Tell the wrapper to cd to selected path
	if arg_verbose {
		println!("CHDIRV {}", line);  // Use 'change dir verbose' flag
	} else {
		println!("CHDIR {}", line);   // Use 'change dir' flag
	}

	Ok(())
}

// Delete one of saved paths
fn delete(arg_pos: Option<&u8>) -> Result<(), Error> {

	// Get path id from pos arg, throw error if None
	let id = arg_pos.map(pos_to_id()).expect("FATAL: '[pos]' was not provided");
	let mut lines = read_lines()?;

	// Check id
	if id >= lines.len() {
		return get_invalid_pos_err(id, lines.len());
	}

	// Get copy of path
	let path: String = lines[id].clone();

	// Remove path from list
	lines.remove(id);
	save_lines(lines)?;

	// Print confirmation message
	print_ok("deleted".purple(), fmt_path(id, &path));
	Ok(())
}

// Print shell wrapper function
fn dump_wrapper(arg_shell: Option<&String>) -> Result<(), Error> {
	// TODO: match shell type

	let function = include_str!("wrapper.sh");
	println!("{}", function);
	Ok(())
}


fn ask_to_remove(id: usize, mut lines: Vec<String>) -> Result<(), Error> {
	eprintln!();

	// Path to remove
	let line = &lines[id];
	// User response buffor
	let mut input = String::new();

	// Ask repeatedly, quit when user responds correctly
	loop {
		eprint!("Remove from list? [y/n]: ");

		// Ensure prompt prints immediately
		io::stderr().flush().map_err(|e| print_warning(Warn::CannotFlush(e))).ok();

		// Save whole input to buffor
		match io::stdin().read_line(&mut input) {

			Ok(nbytes) => {
				// Enter should take one byte
				assert!(nbytes > 0);

				// Get first char from input
				if let Some(first_char) = input.trim().chars().next() {
					if first_char == 'Y' || first_char == 'y' {

						// Get copy of path
						let path: String = lines[id].clone();

						// Remove path from list
						lines.remove(id);
						save_lines(lines)?;

						// Print confirmation message and quit
						println!();
						print_ok("deleted".purple(), fmt_path(id, &path));
						return Ok(());
					}
					else if first_char == 'N' || first_char == 'n' {
						// Quit
						return Ok(());
					}
				}
			}
			Err(e) => return Err(Error::CannotReadInput(input, e))
		}

		// Clear buffor
		input.clear();
	}
}

// Print n lines from path list
fn print_lines(lines: Vec<String>, n: usize) {

	for i in 0..n {
		// Format each line depending on whether it is a directory
		if Path::new(&lines[i]).is_dir() {
			println!("{}", fmt_path(i, &lines[i]));
		} else {
			println!("{}", format!("{} [*]", fmt_path(i, &lines[i])).strikethrough().dimmed());
		};
	}
}

// Get list of paths from PATH_FILE
fn read_lines() -> Result<Vec<String>, Error> {

	Ok(
		// Read path file and parse
		fs::read_to_string(get_path_file()?)
			.map_err(|e| Error::CannotReadFile(e))?
			.lines()
			.filter(|l| !l.is_empty())
			.map(|l| l.to_string())
			.collect::<Vec<String>>()
	)
}

// Save list of paths to PATH_FILE
fn save_lines(lines: Vec<String>) -> Result<(), Error> {

	// Open path file
	let mut file= fs::File::create(get_path_file()?).map_err(|e| Error::CannotOpenFile(e))?;

	// Write lines to file
	for l in lines {
		writeln!(file, "{l}").unwrap_or_else(|e| print_warning(Warn::LineWriteFailed(l, e)));
	}

	Ok(())
}

// Check path file & return its path
fn get_path_file() -> Result<PathBuf, Error> {

	// Expanded path file String
	let path_str = shellexpand::tilde(PATH_FILE).into_owned();

	// Expanded path file Path
	let path = Path::new(&path_str);

	// Check path file existence
	if !path.try_exists().map_err(|e| Error::CannotCheckFile(e))? {
		return Err(Error::NoPathFile());
	}

	Ok(path.to_owned())
}

// Get InvalidPosValue Error
fn get_invalid_pos_err(id: usize, n_lines: usize) -> Result<(), Error> {
	return Err(Error::InvalidPosValue(id, n_lines));
}

// Print errors
fn print_error(err: Error) {
	eprintln!("{} {}", "error:".bold().red(), err);
}
// Print warning
fn print_warning(warn: Warn) {
	eprintln!("{} {}\n", "warning:".bold().yellow(), warn);
}

// Print confirmation message
fn print_ok(header: ColoredString, body: String) {
	println!("{} {}", header.bold(), body);
}

// Get closure for maping position to id
fn pos_to_id() -> impl Fn(&u8) -> usize {
	|&pos| (pos - 1) as usize
}

// Get pos from id
fn id_to_pos(id: usize) -> usize {
	id + 1
}

// Get formated path id
fn fmt_id(id: usize) -> String {
	format!("[{}]", id_to_pos(id))
}

// Get formated path
fn fmt_path(id: usize, path: &String) -> String {
	format!("{} {}", fmt_id(id), path)
}