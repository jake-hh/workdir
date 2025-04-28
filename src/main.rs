#![allow(dead_code, unused_variables)]

use clap::{arg, command, value_parser, Arg, Command};
use color_eyre::eyre;	// note: does not preserve printing order
use std::path::Path;
use std::io::Write;
use std::fs;

static PATH_FILE: &str = "~/.local/state/workdir";

// Max amount of stored paths
const LIMIT: usize = 20;

fn main() -> eyre::Result<()> {
	color_eyre::install()?;

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

fn list(size: Option<&u8>) -> eyre::Result<()> {
	let lines = read_lines()?;

	let n: usize = match size {
		Some(num) => {
			if num.clone() as usize > lines.len() {
				return Err(eyre::eyre!("invalid value '{}' for '[size]': only {} paths are saved", num, lines.len()));
			}
			num.clone() as usize
		},
		None => lines.len()
	};

	list_lines(lines, n)?;

	Ok(())
}

fn l() -> eyre::Result<()> {
	let lines = read_lines()?;

	let n: usize = if lines.len() < 5 { lines.len() } else { 5 };

	list_lines(lines, n)?;

	Ok(())
}

fn save (path: Option<&String>, pos: Option<&u8>) -> eyre::Result<()> {

	let id: usize = match pos {
		Some(num) => (num -1) as usize,
		None => 0
	};

	let path_str = path.expect("current path was not provided");

	if !Path::new(path_str).is_dir() {
		return Err(eyre::eyre!("'{}' is not a directory", path_str));
	}

	let mut lines = read_lines()?;
	let mut remove: Option<usize> = None;
	let size = lines.len();

	for i in 0..size {
		if &lines[i] == path_str {
			if i == id {
				return Err(eyre::eyre!("path '{}' already exists on thath position", lines[i]));
			}
			remove = Some(i);
		}
	}

	if let Some(rm_id) = remove {
		if id >= size {
			return Err(eyre::eyre!("invalid value '{}' for '[pos]': only {} paths are saved", id + 1, size));
		}
		lines.remove(rm_id);
	}
	else {
		if id > size {
			return Err(eyre::eyre!("invalid value '{}' for '[pos]': only {} paths are saved", id + 1, size));
		}
		else if size >= LIMIT {
			return Err(eyre::eyre!("limit of {} saved paths reached", LIMIT))
		}
	}

	lines.insert(id, path_str.clone());
	save_lines(lines)?;

	match remove {
		Some(rm_id) => println!("moved: [{}] -> [{}] {}", rm_id +1, id +1, path_str),
		None => println!("saved: [{}] {}", id +1, path_str),
	}

	Ok(())
}

fn restore (pos: Option<&u8>, verbose: bool) -> eyre::Result<()> {

	let id: usize = match pos {
		Some(num) => (num -1) as usize,
		None => 0
	};

	let lines = read_lines()?;

	if id >= lines.len() {
		return Err(eyre::eyre!("invalid value '{}' for '[pos]': only {} paths are saved", id + 1, lines.len()));
	}

	let line = &lines[id];

	if !Path::new(line).is_dir() {
		// remove that path from the list
		return Err(eyre::eyre!("'{}' is not an existing directory", line));
	}

	// cd to that path
	if verbose {
		println!("CHDIRV {}", line);
	} else {
		println!("CHDIR {}", line);
	}

	Ok(())
}

fn delete (pos: Option<&u8>) -> eyre::Result<()> {

	let id = *pos.expect("'[pos]' was not provided") as usize -1;

	let mut lines = read_lines()?;

	if id >= lines.len() {
		return Err(eyre::eyre!("invalid value '{}' for '[pos]': only {} paths are saved", id + 1, lines.len()));
	}

	let path_str: String = lines[id].clone();

	lines.remove(id);
	save_lines(lines)?;

	println!("deleted: [{}] {}", id +1, path_str);

	Ok(())
}

fn dump_wrapper (shell: Option<&String>) -> eyre::Result<()> {
	let function = include_str!("wrapper.sh");
	println!("{}", function);

	Ok(())
}


fn list_lines(lines: Vec<String>, n: usize) -> eyre::Result<()> {
	for i in 0..n {
		let missing_star = if !Path::new(&lines[i]).is_dir() {" [*]"} else {""};
		println!("[{}] {}{}", i +1, lines[i], missing_star);
	}

	Ok(())
}

fn read_lines() -> eyre::Result<Vec<String>> {
	let mut result = Vec::new();
	let fstr = shellexpand::tilde(PATH_FILE).into_owned();
	let file = Path::new(&fstr);

	if !file.try_exists().expect("can't check existence of path file") {
		return Err(eyre::eyre!("path file doesn't exist"));
	}

	let text = fs::read_to_string(file).expect("can't read path file");

	for line in text.lines() {
		if !line.is_empty() {
			result.push(line.to_string());
		}
	}

	Ok(result)
}

fn save_lines (lines: Vec<String>) -> eyre::Result<()> {
	let fstr = shellexpand::tilde(PATH_FILE).into_owned();
	let file = Path::new(&fstr);

	if !file.try_exists().expect("can't check existence of path file") {
		return Err(eyre::eyre!("path file doesn't exist"));
	}

	let mut buffer= fs::File::create(file)?;

	for l in lines {
		buffer.write(l.as_bytes())?;
		buffer.write("\n".as_bytes())?;
	}

	Ok(())
}