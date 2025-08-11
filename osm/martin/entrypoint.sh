#!/bin/sh

mkdir -p /app/config

cat /data/martin/config.yaml | sed "s/\$USER/$USER/" > /app/config/config.yaml

martin --config=/app/config/config.yaml


