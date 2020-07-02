# *Multi-Architecture Docker image for AudioServe*

## Overview

![Publish on docker](https://github.com/greencashew/audioserve/workflows/Publish%20on%20docker/badge.svg?branch=master)

Supported architectures:

- **amd64**
- **armv7**
- **armv8**

## Brief

Simple personal server to serve audio files from directories. Intended primarily for audio books, but anything with decent directories structure will do. Focus here is on simplicity and minimalistic design.

Server is in Rust,  default web client (HTML5 + Javascript) is intended for modern browsers (latest Firefox or Chrome) and is integrated with the server. There is also [Android client](https://github.com/izderadicka/audioserve-android) and API for custom clients.

For some background and video demo check this article [Audioserve Audiobooks Server - Stupidly Simple or Simply Stupid?](http://zderadicka.eu/audioserve-audiobooks-server-stupidly-simple-or-simply-stupid)

[===== READ MORE =====](https://github.com/izderadicka/audioserve)

-----

### Docker Image

Simple example:

```bash
docker run -d --name audioserve -p 3000:3000 -v /path/to/your/audiobooks:/audiobooks  greencashew/audioserve --no-authentication /audiobooks
```

Then open <http://localhost:3000> - and browse your collection.  This is indeed the very minimal configuration of audioserve. For real deployment you'd like provide more command line parameters (or environment variables or your custom config file) - see more complex example below.

A more detailed example:

```bash
    docker run -d --name audioserve -p 3000:3000 \
        -v /path/to/your/audiobooks1:/collection1 \
        -v /path/to/your/audiobooks2:/collection2 \
        -v /path/for/audioserve-data:/home/audioserve/.audioserve \
        -e AUDIOSERVE_SHARED_SECRET=mypass \
        greencashew/audioserve \
        --ssl-key /audioserve/ssl/audioserve.p12 --ssl-key-password mypass \
        --search-cache \
        /collection1 /collection2
```

In the above example, we are adding two different collections of audiobooks (collection1 and collection2).
Both are made available to the container via `-v` option and then passed to audioserve on command line.
Also we have maped with `-v` some folder to `/home/audioserve/.audioserve`, where runtime data of audioserve are stored (server secret, caches ...)

We set the shared secret via `AUDIOSERVE_SHARED_SECRET` env.variable and specify use of TLS via `--ssl-key` and `ssl-key-password` (the tests only self-signed key is already prebundled in the image, for real use you'll need to generate your own key, or use reverse proxy that terminates TLS). We also enable search cache with `--search-cache` argument.

-----

## Docker Compose

```bash
version: '3.3'
services:
    audioserve:
        container_name: audioserve
        ports:
            - '3000:3000'
        environment:
            - AUDIOSERVE_SHARED_SECRET=pass
        volumes:
            - '/mnt/audiobooks/:/audiobooks'
        image: greencashew/audioserve
        restart: always
        command:
            --search-cache /audiobooks
```

## Parameters

```bash
➜  ~ docker run -it --rm  greencashew/audioserve --help
audioserve 0.12.2
Ivan <ivan.zderadicka@gmail.com>

USAGE:
    audioserve [FLAGS] [OPTIONS] [BASE_DIR]...

FLAGS:
        --allow-symlinks             Will follow symbolic/soft links in collections directories
        --cors                       Enable CORS - enabled any origin of requests
    -d, --debug                      Enable debug logging (detailed logging config can be done via RUST_LOG env.
                                     variable)
        --disable-folder-download    Disables API point for downloading whole folder
    -h, --help                       Prints help information
        --no-authentication          no authentication required - mainly for testing purposes
        --print-config               Will print current config, with all other options to stdout, usefull for creating
                                     config file
        --search-cache               Caches collections directory structure for quick search, monitors directories for
                                     changes
        --thread-pool-large          Use larger thread pool (usually will not be needed)
    -V, --version                    Prints version information

OPTIONS:
        --chapters-duration <chapters-duration>
            If long files is presented as chapters, one chapter has x mins [default: 30] [env:
            AUDIOSERVE_CHAPTERS_FROM_DURATION=]
        --chapters-from-duration <chapters-from-duration>
            forces split of audio file larger then x mins into chapters (not physically, but it'll be just visible as
            folder with chapters)[default:0 e.g. disabled] [env: AUDIOSERVE_CHAPTERS_FROM_DURATION=]
    -c, --client-dir <client-dir>
            Directory with client files - index.html and bundle.js [env: AUDIOSERVE_CLIENT_DIR=]

    -g, --config <config>
            Configuration file in YAML format [env: AUDIOSERVE_CONFIG=]

        --data-dir <data-dir>
            Base directory for data created by audioserve (caches, state, ...) [default is $HOME/.audioserve] [env:
            AUDIOSERVE_DATA_DIR=]
    -l, --listen <listen>
            Address and port server is listening on as address:port (by default listen on port 3000 on all interfaces)
            [env: AUDIOSERVE_LISTEN=]
        --positions-file <positions-file>
            File to save last listened positions [env: AUDIOSERVE_POSITIONS_FILE=]

        --secret-file <secret-file>
            Path to file where server secret is kept - it's generated if it does not exists [default: is
            $HOME/.audioserve.secret] [env: AUDIOSERVE_SECRET_FILE=]
    -s, --shared-secret <shared-secret>
            Shared secret for client authentication [env: AUDIOSERVE_SHARED_SECRET=]

        --shared-secret-file <shared-secret-file>
            File containing shared secret, it's slightly safer to read it from file, then provide as command argument
            [env: AUDIOSERVE_SHARED_SECRET_FILE=]
        --ssl-key <ssl-key>
            TLS/SSL private key and certificate in form of PKCS#12 key file, if provided, https is used [env:
            AUDIOSERVE_SSL_KEY=]
        --ssl-key-password <ssl-key-password>
            Password for TLS/SSL private key [env: AUDIOSERVE_SSL_KEY_PASSWORD=]

        --thread-pool-keep-alive-secs <thread-pool-keep-alive-secs>
            Threads in pool will shutdown after given seconds, if there is no work. Default is to keep threads forever.
            [env: AUDIOSERVE_THREAD_POOL_KEEP_ALIVE=]
        --token-validity-days <token-validity-days>
            Validity of authentication token issued by this server in days[default 365, min 10] [env:
            AUDIOSERVE_TOKEN_VALIDITY_DAYS=]
    -x, --transcoding-max-parallel-processes <transcoding-max-parallel-processes>
            Maximum number of concurrent transcoding processes [default: 2 * number of cores] [env:
            AUDIOSERVE_MAX_PARALLEL_PROCESSES=]
        --transcoding-max-runtime <transcoding-max-runtime>
            Max duration of transcoding process in hours. If takes longer process is killed. Default is 24h [env:
            AUDIOSERVE_TRANSCODING_MAX_RUNTIME=]

ARGS:
    <BASE_DIR>...    Root directories for audio books, also refered as collections [env: AUDIOSERVE_BASE_DIRS=]
```

## Author

Project has been created by [Ivan Zderadicka](https://github.com/izderadicka)

## Sources

- https://github.com/greencashew/audioserve/blob/master/.github/workflows/publish-on-docker.yml
- https://github.com/greencashew/audioserve
- https://github.com/izderadicka/audioserve

