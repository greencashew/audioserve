#!/bin/bash

docker build --tag audioserve-builder -f Dockerfile.static .
# if repeated build are done it can be made faster by mapping volumes to /.cargo and /.npm
docker run -it --rm -v /home/ivan/workspace/audioserve/:/src -u $(id -u) audioserve-builder