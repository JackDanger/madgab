#!/bin/zsh
# https://developer.apple.com/metal/tensorflow-plugin/
#
source $(which virtualenvwrapper.sh)
workon madgab

source ~/miniforge3/bin/activate

conda install -c apple tensorflow-deps

pip install -r requirements.txt

exec zsh
