#![allow(dead_code, unused_variables)]

use clap::{arg, command, value_parser, Arg, Command};
use color_eyre::eyre::{self, eyre};
use colored::{Colorize, ColoredString, control};
use std::cmp::min;
use std::path::{Path, PathBuf};
use std::io::Write;
use std::fs;

// File used to store a list of paths (working directories)
static PATH_FILE: &str = "~/.local/state/workdir";

// Max amount of stored paths (plus one)
const SAVED_PATHS_LIMIT_P1: usize = 20;

// Max amount of printed lines in short list
const SHORT_LIST_LINES_LIMIT: usize = 5;


fn main() -> eyre::Result<()> {
	// Force colored output regardless of TTY
	control::set_override(true);
	// Enable colored error messages (Does not preserve printing order)
	color_eyre::install()?;

	// Parse specified CLI args
	let args = command!() 	// requires 'cargo' feature
		.about("workdir - fast working directory path switcher")
		.author("jake")
		.disable_help_subcommand(true)
		.args_conflicts_with_subcommands(true)

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
					arg!(<path> "Current directory path - provided by wrapper function - do not enter") //.hide(true)
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
	match args.subcommand() {
		Some(("list", subargs))    => list(subargs.get_one::<u8>("length"))?,
		Some(("l", _))                          => l()?,
		Some(("save", subargs))    => save(subargs.get_one::<String>("path"), subargs.get_one::<u8>("pos"))?,
		Some(("restore", subargs)) => restore(subargs.get_one::<u8>("pos"), subargs.get_flag("verbose"))?,
		Some(("delete", subargs))  => delete(subargs.get_one::<u8>("pos"))?,
		Some(("wrapper", subargs)) => dump_wrapper(subargs.get_one::<String>("shell"))?,
		_                          => restore(args.get_one::<u8>("pos"), args.get_flag("verbose"))?
	}

	Ok(())
}


// List paths
fn list(arg_length: Option<&u8>) -> eyre::Result<()> {
	let lines = read_lines()?;
	let n_lines = lines.len();

	// Get number of printed lines from specified length arg, list all lines as default
	let n: usize = arg_length.map_or(n_lines, |&l| l as usize);

	// If length arg is bigger than number of lines => Throw error msg
	if n > n_lines {
		return Err(eyre!("invalid value '{}' for '[length]': only {} paths are saved", n, n_lines));
	}

	print_lines(lines, n)?;
	Ok(())
}

// Short list
fn l() -> eyre::Result<()> {
	let lines = read_lines()?;

	// Get number of printed lines
	let n: usize = min(lines.len(), SHORT_LIST_LINES_LIMIT);

	print_lines(lines, n)?;
	Ok(())
}

// Save or move current PWD path in path list
fn save(arg_path: Option<&String>, arg_pos: Option<&u8>) -> eyre::Result<()> {

	// Get new path id from pos arg, default to 0
	let id = arg_pos.map(pos_to_id()).unwrap_or(0);

	// Unwrap path
	let path: &String = arg_path.expect("current path was not provided. Please use the included wrapper");

	// Check if path is a directory
	if !Path::new(path).is_dir() {
		return Err(eyre!("'{}' is not a directory", path));
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
			return Err(eyre!("path '{}' already exists on that position", lines[id]));
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
			return Err(eyre!("limit of {} saved paths reached", SAVED_PATHS_LIMIT_P1))
		}
	}

	// Save new path in list
	lines.insert(id, path.clone());
	save_lines(lines)?;

	// Print confirmation message
	match existing_id {
		Some(rm_id) => print_ok("moved".cyan(), format!("{} -> {} {}", fmt_id(rm_id), fmt_id(id), path)),
		None => print_ok("saved".green(), format!("{} {}", fmt_id(id), path)),
	}

	Ok(())
}

// Change directory to one of saved paths
fn restore(arg_pos: Option<&u8>, arg_verbose: bool) -> eyre::Result<()> {

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
		// TODO: Remove that path from the list
		return Err(eyre!("'{}' is not an existing directory", line));
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
fn delete(arg_pos: Option<&u8>) -> eyre::Result<()> {

	// Get path id from pos arg, throw error if None
	let id = arg_pos.map(pos_to_id()).expect("'[pos]' was not provided");
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
	print_ok("deleted".purple(), format!("{} {}", fmt_id(id), path));
	Ok(())
}

// Print shell wrapper function
fn dump_wrapper(arg_shell: Option<&String>) -> eyre::Result<()> {
	// TODO: match shell type

	let function = include_str!("wrapper.sh");
	println!("{}", function);
	Ok(())
}


// Print n lines from path list
fn print_lines(lines: Vec<String>, n: usize) -> eyre::Result<()> {

	for i in 0..n {
		// Format each line depending on whether it is a directory
		if Path::new(&lines[i]).is_dir() {
			println!("{} {}", fmt_id(i), lines[i]);
		} else {
			println!("{}", format!("{} {} [*]", fmt_id(i), lines[i]).strikethrough().dimmed());
		};
	}

	Ok(())
}

// Get list of paths from PATH_FILE
fn read_lines() -> eyre::Result<Vec<String>> {

	Ok(
		// Read path file and parse
		fs::read_to_string(get_path_file()?)
			.expect("can't read path file")
			.lines()
			.filter(|l| !l.is_empty())
			.map(|l| l.to_string())
			.collect::<Vec<String>>()
	)
}

// Save list of paths to PATH_FILE
fn save_lines(lines: Vec<String>) -> eyre::Result<()> {

	// Open path file
	let mut file= fs::File::create(get_path_file()?)?;

	// Write lines to file
	for l in lines {
		writeln!(file, "{l}")?;
	}

	Ok(())
}

fn get_path_file() -> eyre::Result<PathBuf> {

	// Expanded path file String
	let path_str = shellexpand::tilde(PATH_FILE).into_owned();

	// Expanded path file Path
	let path = Path::new(&path_str);

	// Check path file existence
	if !path.try_exists().expect("can't check existence of path file") {
		return Err(eyre!("path file doesn't exist"));
	}

	Ok(path.to_owned())
}

// Get closure for maping position to id
fn pos_to_id() -> impl Fn(&u8) -> usize {
	|&pos| (pos - 1) as usize
}

fn get_invalid_pos_err(id: usize, n_lines: usize) -> eyre::Result<()> {
	return Err(eyre!("invalid value '{}' for '[pos]': only {} paths are saved", id + 1, n_lines));
}

// Print confirmation message
fn print_ok(header: ColoredString, body: String) {
	println!("{} {}", header.bold(), body);
}

// Get formated path id
fn fmt_id(id: usize) -> String {
	format!("[{}]", id +1)
}

// // Get formated path
// fn fmt_path(id: usize, path: &String) -> String {
// 	format!("{} {}", fmt_id(id), path)
// }