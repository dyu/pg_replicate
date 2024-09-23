#!/bin/sh

[ "$#" -lt 2 ] && echo '1st arg (stdout|duckdb) and 2nd arg (publication name) are required.' && exit 1

CURRENT_DIR=$PWD
# locate
if [ -z "$BASH_SOURCE" ]; then
    SCRIPT_DIR=`dirname "$(readlink -f $0)"`
elif [ -e '/bin/zsh' ]; then
    F=`/bin/zsh -c "print -lr -- $BASH_SOURCE(:A)"`
    SCRIPT_DIR=`dirname $F`
elif [ -e '/usr/bin/realpath' ]; then
    F=`/usr/bin/realpath $BASH_SOURCE`
    SCRIPT_DIR=`dirname $F`
else
    F=$BASH_SOURCE
    while [ -h "$F" ]; do F="$(readlink $F)"; done
    SCRIPT_DIR=`dirname $F`
fi
# change pwd
cd $SCRIPT_DIR

OPTS_SUFFIX=''
case "$1" in
    stdout)
    ;;
    duckdb)
    mkdir -p target/data
    OPTS_SUFFIX="--duckdb-file=$PWD/target/data/$2.db"
    ;;
    *)
    echo "Run with (stdout|duckdb) as 1st arg."
    exit 1
    ;;
esac

RUST_LOG=info ./target/release/examples/$1 \
--db-host localhost --db-port 5016 \
--db-username postgres --db-password root_pw \
--db-name postgres \
$OPTS_SUFFIX cdc $2 $1_slot
