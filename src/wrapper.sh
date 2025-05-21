# wrapper function for workdir.rs
# release version
#
# $> workdir wrapper >> ~/.zsh/custom/workdir.sh

function wd {
	# check arguments & run workdir
	if [ -z "$1" ]
	then
		# when no args found
		x=`workdir`
	else
		subcmd="$1"
		shift

		if [ "$subcmd" = "save" ] || [ "$subcmd" = "s" ]
		then
			# when save subcommand found
			x=`workdir "$subcmd" "$PWD" "$@"`
		else
			# when other args found
			x=`workdir "$subcmd" "$@"`
		fi
	fi

	# check workdir output
	if [ "${x::6}" = "CHDIRV" ]
	then
		# when output contains 'change dir verbose' flag
		echo "changing dir to ${x:7}"
		cd "${x:7}"
	elif [ "${x::5}" = "CHDIR" ]
	then
		# when output contains 'change dir' flag
		cd "${x:6}"
	else
		# when output contains no flags
		echo "$x"
	fi
}