#![allow(dead_code, unused_variables)]

use clap::{arg, command, value_parser, Command};
use color_eyre::eyre;
use std::path::Path;
use std::io::Write;
use std::fs;

static PATH_FILE: &str = "/tmp/myfile";

fn main() -> eyre::Result<()> {
	color_eyre::install()?;
	println!();

	let args = command!() 	// requires 'cargo' feature
		.about("workdir - fast working directory path switcher")
		.author("Jake")
		.disable_help_subcommand(true)

		.subcommand(Command::new("list").about("List recent paths"))
		.subcommand(Command::new("push").about("Push path on top of list"))
		.subcommand(
			Command::new("restore")
				.about("Switch to selected path")
                .arg(
					arg!([num] "Optional path number")
					.value_parser(value_parser!(u8).range(1..10))))
		.subcommand(
			Command::new("save")
				.about("Save path")
                .arg(
					arg!([path] "Current directory path")
					.required(true)
					.value_parser(value_parser!(String)))
                .arg(
					arg!([num] "Optional path number")
					.value_parser(value_parser!(u8).range(1..10))))
        .get_matches();

    match args.subcommand() {
        Some(("list", _))          => list()?,
        Some(("push", _))          => push(),
        Some(("save", subargs))    => save(subargs.get_one::<String>("path"), subargs.get_one::<u8>("num"))?,
        Some(("restore", subargs)) => restore(subargs.get_one::<u8>("num"))?,
        _                          => restore(None)?,
	}

	Ok(())
}

fn list() -> eyre::Result<()> {
	let lines = read_lines()?;

	println!("PATHS");
	for i in 1..lines.len()+1 {
		println!("[{}] {}", i, lines[i-1]);
	}
	Ok(())
}

fn push() {
	println!("'wd push' was used")
}

fn save (path: Option<&String>, num: Option<&u8>) -> eyre::Result<()> {

	let id: usize = match num {
		Some(npos) => (npos -1) as usize,
		None => 0
	};

	let path_str = path.expect("current path was not provided");

	if !Path::new(path_str).is_dir() {
		return Err(eyre::eyre!("'{}' is not a directory", path_str));
	}


	println!("'wd save' was used, id is: {:?}, path is {:?}", id, path_str);

	let mut lines = read_lines()?;

	if id > lines.len() {
		return Err(eyre::eyre!("invalid value '{}' for '[num]': only {} paths are saved", id + 1, lines.len()));
	}
	
	lines.insert(id, path_str.clone());
	
	save_lines(lines)?;
	
	Ok(())
}

fn restore (num: Option<&u8>) -> eyre::Result<()> {
	
	let id: usize = match num {
		Some(npos) => (npos -1) as usize,
		None => 0
	};

	let lines = read_lines()?;

	if id >= lines.len() {
		return Err(eyre::eyre!("invalid value '{}' for '[num]': only {} paths are saved", id + 1, lines.len()));
	}

	let line = &lines[id];

	if !Path::new(line).is_dir() {
		// remove that path from the list
		return Err(eyre::eyre!("'{}' is not an existing directory", line));
	}

	// cd to that path
	println!("CHDIR {}", line);

	Ok(())
}

fn read_lines() -> eyre::Result<Vec<String>> {
	let mut result = Vec::new();
	let file = Path::new(PATH_FILE);

	if !file.try_exists().expect("Can't check existence of path file") {
		return Err(eyre::eyre!("path file doesn't exist"));
	}

	let text = fs::read_to_string(file).expect("Can't read path file");

	for line in text.lines() {
		result.push(line.to_string());
	}
	Ok(result)
}

fn save_lines (lines: Vec<String>) -> eyre::Result<()> {
	let file = Path::new(PATH_FILE);

	if !file.try_exists().expect("Can't check existence of path file") {
		return Err(eyre::eyre!("path file doesn't exist"));
	}

	let mut buffer= fs::File::create(file)?;
	
	for l in lines {
		buffer.write(l.as_bytes())?;
		buffer.write("\n".as_bytes())?;
	}

	Ok(())
}