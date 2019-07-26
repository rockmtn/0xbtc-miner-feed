# 0xbtc-miner-feed

Low-latency mining parameters server for 0xBitcoin miners (written in Rust).

## The problem

0xBitcoin solo mining software always needs the latest `miningTarget` and `challengeNumber` from the [0xBitcoin smart contract](https://etherscan.io/address/0xb6ed7644c69416d67b522e20bc294a9a9b405b31#code) in order to mine. These parameters can be obtained from an Ethereum EVM (parity or geth) or from a service (Infura or Cloudflare). However, there are two problems:

- If many clients are polling the same service frequently, they might collectively hit rate limits (especially with Infura).
  - Side note: Polling is actually not ideal because it's high-latency and therefore increases the chance of missing blocks or mining stale blocks.
- If clients want to stream `Mint()` events from an EVM or Infura in order to achieve low-latency, then they have to use websockets. Using websockets complicates solo mining software.

## The solution

The 0xbtc-miner-feed service is a low-latency, low-overhead way to get 0xBitcoin mining parameters.

It's low latency because:

- The latest `challengeNumber` is gathered instananeously from new `Mint()` events.
- The latest `miningTarget` is updated once every 10 seconds. (This is plenty fast since `miningTarget` usually changes about once a week.)

When either of these values changes, all clients are notified immediately. Latency inside the server is approximately zero seconds (from receiving the `Mint()` event to broadcasting the latest mining parameters to all connected clients).

It's low overhead because:

- All that's required is TCP sockets and simple string processing.

## Download

- Linux static binary: [0xbtc-miner-feed_v0.1.0.zip](https://github.com/rockmtn/0xbtc-miner-feed/releases/download/v0.1.0/0xbtc-miner-feed_v0.1.0.zip)

## Configure

After downloading, copy `config.example.toml` to `config.toml` and edit the values in the latter file.

```
cp config.example.toml config.toml
# then edit config.toml
```

- Example config: [config.example.toml](config.example.toml)

## Building from source

Install Rust: <https://www.rust-lang.org/tools/install>

Build for development:
```
cargo build
```

Build for release:
```
cargo build --release
```

Build for distribution (Linux only):

```
make release
```

## Running

```
./0xbtc-miner-feed
```

Or if built from source you can also run with `cargo`:

```
cargo run
```

### Running as a system service

Modify the file [systemd/0xbtc-miner-feed.service](systemd/0xbtc-miner-feed.service) so it has the right username and paths for your system.

```
sudo cp systemd/0xbtc-miner-feed.service /etc/systemd/system/
sudo systemctl enable 0xbtc-miner-feed
sudo systemctl start 0xbtc-miner-feed
```

## Protocol

The service protocol is JSON Lines over TCP. This should keep the client-side implementation reasonably simple. (Websockets are a pain to use in many languages, like C++, but plain TCP sockets are easy in all languages.)

Just connect to the server with a normal TCP connection. The server will immediately respond with the most recent mining parameters (JSON followed by a newline). You can hang up at that point if you only want the current parameters, or, if you stay connected, you will get another line of JSON every time the 0xBitcoin mining parameters change. The server also sends pings once every 30s to keep the connection alive.

The JSON sent by the server (except for pings which you can ignore, see example below) will always have exactly the following format, the length of the JSON will always be exactly 172 bytes, the keys will always be in the same order (`miningTarget` first, `challengeNumber` second), and the hex values will always be at the fixed byte positions demonstrated here:

```
{"miningTarget":"0x0000000000000d3cbfef57a209d15ffe4d8fbaeab4e36e5054953f8e38b0a644","challengeNumber":"0xd2d92bb38f9f08940ee420718e46518fd21bd9e05dc9b32c73b8c0f94e762c38"}
```

That means you can extract the hex values of `miningTarget` and `challengeNumber` from their character positions in the string; you don't have to parse the JSON if that would be difficult in your environment.

Here's an example of using the service, demonstrated via netcat (`nc`):

```
$ nc 127.0.0.1 3333
{"miningTarget":"0x0000000000000d3cbfef57a209d15ffe4d8fbaeab4e36e5054953f8e38b0a644","challengeNumber":"0xd2d92bb38f9f08940ee420718e46518fd21bd9e05dc9b32c73b8c0f94e762c38"}
{"ping":"ping"}
{"ping":"ping"}
{"ping":"ping"}
{"ping":"ping"}
{"ping":"ping"}
{"ping":"ping"}
{"ping":"ping"}
{"ping":"ping"}
{"miningTarget":"0x0000000000000d3cbfef57a209d15ffe4d8fbaeab4e36e5054953f8e38b0a644","challengeNumber":"0x7e2835247d31c315a8131b08d3ac312d403fa1b9116acc5983934c8fb05d4e8b"}
{"ping":"ping"}
{"ping":"ping"}
...
```
