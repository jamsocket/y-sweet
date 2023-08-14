#!/bin/sh

set -eux
cd "$(dirname -- "$0")"

VERSION=

while getopts v: opts
do
    case $opts in
        v) VERSION="$OPTARG";;
        ?) printf "Usage: %s: [-v version] \n" "$0"
        exit 2;;
    esac
done


printf "Updating y-sweet and y-sweet-core crate version!\n"
(cd crates && cargo ws version custom "${VERSION}" --no-git-commit --force "\*" --all --yes) 

printf "Updating js-pkg version!\n"
(cd js-pkg/server && npm version "${VERSION}" --no-git-tag-version)

printf "please check versions and commit!\n"
