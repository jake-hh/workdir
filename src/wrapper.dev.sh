# wrapper function for workdir.rs
# development version
#
# $> source src/wrapper.dev.sh

function wd {
	# run workdir
	x=`cargo run -- "$@"`

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