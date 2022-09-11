#!/bin/sh
rsync -rv --exclude=.git --exclude=target . protohackers:/root/code
