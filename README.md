# SampleGraph API

This web server provides a simple REST API for SampleGraph: a web application for visualizing relationships between musical samples.

## Development 🏗️

Before beginning development, you must acquire an API client key from [Genius](https://docs.genius.com).

### Local 💻

Install the following:

- [Rust](https://www.rust-lang.org/tools/install)
- [Redis](https://redis.io/docs/getting-started/installation)

Then run the following:

```console
foo@bar$ redis-server
foo@bar$ cargo run
```

### Docker 🐳

Install the following:

- [Docker](https://docs.docker.com/engine/install)

The run the following:

```console
foo@bar$ docker compose build
foo@bar$ docker compose up
```
