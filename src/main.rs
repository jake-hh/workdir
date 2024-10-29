#![allow(dead_code, unused_variables)]

use clap::{arg, command, value_parser, Command};
use color_eyre::eyre;
use std::path::Path;
use std::io::Write;
use std::fs;

static PATH_FILE: &str = "/tmp/myfile";

fn main() -> eyre::Result<()> {
	color_eyre::install()?;

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
		.subcommand(
			Command::new("delete")
				.about("Delete selected path")
				.arg(
					arg!([num] "Path number")
					.required(true)
					.value_parser(value_parser![u8].range(1..10))))
		.subcommand(
			Command::new("wrapper")
				.about("Dump wrapper function")
				.arg(
					arg!([shell] "Shell type")
					.value_parser(["sh", "bash", "zsh"])))
        .get_matches();

    match args.subcommand() {
        Some(("list", _))          => list()?,
        Some(("push", _))          => push(),
        Some(("save", subargs))    => save(subargs.get_one::<String>("path"), subargs.get_one::<u8>("num"))?,
        Some(("restore", subargs)) => restore(subargs.get_one::<u8>("num"))?,
		Some(("delete", subargs))  => delete(subargs.get_one::<u8>("num"))?,
		Some(("wrapper", subargs)) => dump_wrapper(subargs.get_one::<String>("shell"))?,
        _                          => restore(None)?,
	}

	Ok(())
}

fn list() -> eyre::Result<()> {
	println!();

	let lines = read_lines()?;

	println!("PATHS");
	for i in 0..lines.len() {
		println!("[{}] {}", i +1, lines[i]);
	}
	Ok(())
}

fn push() {
	println!();
	println!("'wd push' was used")
}

fn save (path: Option<&String>, num: Option<&u8>) -> eyre::Result<()> {
	println!();

	let id: usize = match num {
		Some(npos) => (npos -1) as usize,
		None => 0
	};

	let path_str = path.expect("current path was not provided");

	if !Path::new(path_str).is_dir() {
		return Err(eyre::eyre!("'{}' is not a directory", path_str));
	}

	let mut lines = read_lines()?;

	if id > lines.len() {
		return Err(eyre::eyre!("invalid value '{}' for '[num]': only {} paths are saved", id + 1, lines.len()));
	}
	
	lines.insert(id, path_str.clone());
	save_lines(lines)?;

	println!("saved: [{}] {}", id +1, path_str);
	Ok(())
}

fn restore (num: Option<&u8>) -> eyre::Result<()> {
	
	let id: usize = match num {
		Some(npos) => (npos -1) as usize,
		None => 0
	};

	let lines = read_lines()?;

	if id >= lines.len() {
		println!();
		return Err(eyre::eyre!("invalid value '{}' for '[num]': only {} paths are saved", id + 1, lines.len()));
	}

	let line = &lines[id];

	if !Path::new(line).is_dir() {
		// remove that path from the list
		println!();
		return Err(eyre::eyre!("'{}' is not an existing directory", line));
	}

	// cd to that path
	println!("CHDIR {}", line);

	Ok(())
}

fn delete (num: Option<&u8>) -> eyre::Result<()> {
	println!();

	let id = *num.expect("'num' was not provided") as usize -1;

	let mut lines = read_lines()?;

	if id > lines.len() {
		return Err(eyre::eyre!("invalid value '{}' for '[num]': only {} paths are saved", id + 1, lines.len()));
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