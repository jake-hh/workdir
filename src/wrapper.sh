# wrapper function for workdir.rs
# release version
#
# $> workdir wrapper >> ~/.zsh/custom/workdir.sh

function wd {
	if [ -z "$1" ]
	then
		x=`wrapper`
	else
		subcmd="$1"
		shift

		if [ "$subcmd" = "save" ] || [ "$subcmd" = "s" ]
		then
			x=`wrapper "$subcmd" "$PWD" "$@"`
		else
			x=`wrapper "$subcmd" "$@"`
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