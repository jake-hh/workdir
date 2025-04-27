# wrapper function for workdir.rs
# development version
#
# $> source src/wrapper.dev.sh

function wd {
	if [ -z "$1" ]
	then
		x=`cargo run --`
	else
		subcmd="$1"
		shift

		if [ "$subcmd" = "save" ] || [ "$subcmd" = "s" ]
		then
			x=`cargo run -- "$subcmd" "$PWD" "$@"`
		else
			x=`cargo run -- "$subcmd" "$@"`
		fi
	fi

	if [ "${x::6}" = "CHDIRV" ]
	then
		echo "changing dir to ${x:7}"
		cd "${x:7}"
	elif [ "${x::5}" = "CHDIR" ]
	then
		cd "${x:6}"
	else
		echo "$x"
	fi
}