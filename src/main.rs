#![allow(dead_code, unused_variables)]

use clap::{arg, command, value_parser, Arg, Command};
use color_eyre::eyre;
use colored::{Colorize, ColoredString, control};
use std::path::Path;
use std::io::Write;
use std::fs;

// File used to store a list of paths (working directories)
static PATH_FILE: &str = "~/.local/state/workdir";

// Max amount of stored paths
const LIMIT: usize = 20;


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
					arg!([size] "Optional max list size")
					.value_parser(value_parser!(u8).range(1..LIMIT as i64))))

		.subcommand(Command::new("l")
				.about("List 5 recent paths"))

		.subcommand(Command::new("restore")
				.about("Switch to selected path")
				.aliases(["r", "res"])
				.arg(
					arg!([pos] "Optional path position")
					.value_parser(value_parser!(u8).range(1..LIMIT as i64)))
				.arg(arg!(-v --verbose "Show verbose info")))

		.subcommand(Command::new("save")
				.about("Save path")
				.aliases(["s"])
				.arg(
					arg!(<path> "Current directory path - provided by wrapper function - do not enter") //.hide(true)
					.value_parser(value_parser!(String)))
				.arg(
					arg!([pos] "Optional path position")
					.value_parser(value_parser!(u8).range(1..LIMIT as i64))))

		.subcommand(Command::new("delete")
				.about("Delete selected path")
				.aliases(["d", "del"])
				.arg(
					arg!(<pos> "Path position")
					.value_parser(value_parser![u8].range(1..LIMIT as i64))))

		.subcommand(Command::new("wrapper")
				.about("Dump wrapper function")
				.arg(
					arg!([shell] "Shell type")
					.value_parser(["sh", "bash", "zsh"])))

		.arg(Arg::new("pos")
			.help("Optional path position to restore")
			.value_parser(value_parser![u8].range(1..LIMIT as i64)))
		.arg(arg!(-v --verbose "Show verbose info"))

		.get_matches();

	// Match CLI args
	match args.subcommand() {
		Some(("list", subargs))    => list(subargs.get_one::<u8>("size"))?,
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
fn list(size: Option<&u8>) -> eyre::Result<()> {
	let lines = read_lines()?;

	// Get number of printed lines...
	let n: usize = match size {
		// ...from specified size arg
		Some(num) => {
			if num.clone() as usize > lines.len() {
				// Size arg is bigger than lines => Show error msg
				return Err(eyre::eyre!("invalid value '{}' for '[size]': only {} paths are saved", num, lines.len()));
			}
			num.clone() as usize
		},
		// ...list all lines as default
		None => lines.len()
	};

	list_lines(lines, n)?;

	Ok(())
}

// Short list
fn l() -> eyre::Result<()> {
	let lines = read_lines()?;

	// Get number of printed lines
	let n: usize = if lines.len() < 5 { lines.len() } else { 5 };

	list_lines(lines, n)?;

	Ok(())
}

// Save or move current PWD path in path list
fn save (path: Option<&String>, pos: Option<&u8>) -> eyre::Result<()> {

	// Get new path id...
	let id: usize = match pos {
		Some(num) => (num -1) as usize,  // ...from pos arg
		None => 0                            //  ...default is 0
	};

	// Unwrap path
	let path_str = path.expect("current path was not provided. Please use the included wrapper");

	// Check if path is a directory
	if !Path::new(path_str).is_dir() {
		return Err(eyre::eyre!("'{}' is not a directory", path_str));
	}

	// Optional id of a line to remove
	let mut remove: Option<usize> = None;
	let mut lines = read_lines()?;
	let size = lines.len();

	// Check lines list
	for i in 0..size {
		if &lines[i] == path_str {
			if i == id {
				// Line already exists at that position => Show error msg
				return Err(eyre::eyre!("path '{}' already exists on thath position", lines[i]));
			}
			// Line already exists at a different position => Mark old id for removal
			remove = Some(i);
		}
	}

	if let Some(rm_id) = remove {
		// Check id
		if id >= size {
			return Err(eyre::eyre!("invalid value '{}' for '[pos]': only {} paths are saved", id + 1, size));
		}
		// Remove old path from list
		lines.remove(rm_id);
	}
	else {
		// Check id
		if id > size {
			return Err(eyre::eyre!("invalid value '{}' for '[pos]': only {} paths are saved", id + 1, size));
		}
		// Check size
		else if size >= LIMIT {
			return Err(eyre::eyre!("limit of {} saved paths reached", LIMIT))
		}
	}

	// Save new path in list
	lines.insert(id, path_str.clone());
	save_lines(lines)?;

	// Print confirmation message
	match remove {
		Some(rm_id) => print_ok("moved".cyan(), format!("{} -> {} {}", fmt_number(rm_id), fmt_number(id), path_str)),
		None => print_ok("saved".green(), format!("{} {}", fmt_number(id), path_str)),
	}

	Ok(())
}

// Change directory to one of saved paths
fn restore (pos: Option<&u8>, verbose: bool) -> eyre::Result<()> {

	// Get path id...
	let id: usize = match pos {
		Some(num) => (num -1) as usize,  // ...from pos arg
		None => 0                            //  ...default is 0
	};

	let lines = read_lines()?;

	// Check id
	if id >= lines.len() {
		return Err(eyre::eyre!("invalid value '{}' for '[pos]': only {} paths are saved", id + 1, lines.len()));
	}

	// Get selected line
	let line = &lines[id];

	// Check if path is a directory
	if !Path::new(line).is_dir() {
		// TODO: Remove that path from the list
		return Err(eyre::eyre!("'{}' is not an existing directory", line));
	}

	// Tell the wrapper to cd to selected path
	if verbose {
		println!("CHDIRV {}", line);  // Use 'change dir verbose' flag
	} else {
		println!("CHDIR {}", line);   // Use 'change dir' flag
	}

	Ok(())
}

// Delete one of saved paths
fn delete (pos: Option<&u8>) -> eyre::Result<()> {

	// Unwrap pos
	let id = *pos.expect("'[pos]' was not provided") as usize -1;

	let mut lines = read_lines()?;

	// Check id
	if id >= lines.len() {
		return Err(eyre::eyre!("invalid value '{}' for '[pos]': only {} paths are saved", id + 1, lines.len()));
	}

	// Get copy of path
	let path_str: String = lines[id].clone();

	// Remove path from list
	lines.remove(id);
	save_lines(lines)?;

	// Print confirmation message
	print_ok("deleted".purple(), format!("{} {}", fmt_number(id), path_str));

	Ok(())
}

// Print shell wrapper function
fn dump_wrapper (shell: Option<&String>) -> eyre::Result<()> {
	let function = include_str!("wrapper.sh");
	println!("{}", function);

	Ok(())
}


// Print n lines from path list
fn list_lines(lines: Vec<String>, n: usize) -> eyre::Result<()> {

	for i in 0..n {
		// Format each line depending on whether it is a directory
		if Path::new(&lines[i]).is_dir() {
			println!("{} {}", fmt_number(i), lines[i]);
		} else {
			println!("{}", format!("{} {} [*]", fmt_number(i), lines[i]).strikethrough().dimmed());
		};
	}

	Ok(())
}

// Get list of paths from PATH_FILE
fn read_lines() -> eyre::Result<Vec<String>> {
	let mut result = Vec::new();

	// Expanded path file String
	let fstr = shellexpand::tilde(PATH_FILE).into_owned();
	// Expanded path file Path
	let file = Path::new(&fstr);

	// Check path file existence
	if !file.try_exists().expect("can't check existence of path file") {
		return Err(eyre::eyre!("path file doesn't exist"));
	}

	// Read path file
	let text = fs::read_to_string(file).expect("can't read path file");

	// Parse text to lines
	for line in text.lines() {
		if !line.is_empty() {
			result.push(line.to_string());
		}
	}

	// Return lines
	Ok(result)
}

// Save list of paths to PATH_FILE
fn save_lines (lines: Vec<String>) -> eyre::Result<()> {

	// Expanded path file String
	let fstr = shellexpand::tilde(PATH_FILE).into_owned();
	// Expanded path file Path
	let file = Path::new(&fstr);

	// Check path file existence
	if !file.try_exists().expect("can't check existence of path file") {
		return Err(eyre::eyre!("path file doesn't exist"));
	}

	// Open path file
	let mut buffer= fs::File::create(file)?;

	// Write lines to file
	for l in lines {
		buffer.write(l.as_bytes())?;
		buffer.write("\n".as_bytes())?;
	}

	Ok(())
}

// Print confirmation message
fn print_ok(header: ColoredString, body: String) {
	println!("{} {}", header.bold(), body);
}

// Get formated path id
fn fmt_number(id: usize) -> String {
	format!("[{}]", id +1)
}

// // Get formated path
// fn fmt_path(id: usize, path: &String) -> String {
// 	format!("{} {}", fmt_number(id), path)
// }