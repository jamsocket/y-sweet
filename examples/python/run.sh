#!/bin/sh

python3 -m venv env

pip install requirements.txt

env/bin/python3 main.py

