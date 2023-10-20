#!/bin/sh

set -e

if ! which worker-build > /dev/null 2>&1;
then
    echo "Installing worker-build"
    cargo install worker-build --version 0.0.10 --force
fi

if [ "$1" = "--dev" ]
then
    worker-build --dev
else
    worker-build --release
fi
