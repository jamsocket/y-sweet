#!/bin/sh

set -e

if ! [ -x "$(command -v worker-build)" ]
then
    echo "Installing worker-build..."
    cargo install -q worker-build --version 0.0.10 --force
    echo "worker-build installed."
fi

if [ "$1" = "--dev" ]
then
    worker-build --dev
else
    worker-build --release
fi
