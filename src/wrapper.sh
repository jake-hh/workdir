
# wrapper function for workdir.rs 
function wd {
    if [ -z "$1" ]
    then
        x=`cargo run --`
    else
        subcmd="$1"
        shift

        if [ "$subcmd" = "save" ]
        then
            x=`cargo run -- "$subcmd" "$PWD" "$@"`
        else
            x=`cargo run -- "$subcmd" "$@"`
        fi
    fi

    if [ "${x::5}" = "CHDIR" ]
    then
        echo "changing dir to ${x:6}"
        cd "${x:6}"
    else
        echo "$x"
    fi
}