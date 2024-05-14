#![allow(dead_code, unused_variables)]

use color_eyre::eyre::Result;

use clap::{arg, command, value_parser, Command};

fn main() -> Result<()> {
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
					arg!([num] "Optional path number")
					.value_parser(value_parser!(u8).range(1..10))))
        .get_matches();

    match args.subcommand() {
        Some(("list", _))          => list(),
        Some(("push", _))          => push(),
        Some(("save", subargs))    => save(subargs.get_one::<u8>("num")),
        Some(("restore", subargs)) => restore(subargs.get_one::<u8>("num")),
        _                          => restore(None),
	}

	Ok(())
}

fn list() {
	println!("'wd list' was used")
}

fn push() {
	println!("'wd push' was used")
}

fn save (num: Option<&u8>) {
	println!("'wd save' was used, num is: {:?}", num);
}

fn restore (num: Option<&u8>) {
	println!("'wd restore' was used, num is: {:?}", num);
}
