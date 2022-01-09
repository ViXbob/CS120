# procol-ftp-client
FTP protocol parser (client side)

[![Build Status](https://travis-ci.org/basiliscos/rust-procol-ftp-client.svg?branch=master)](https://travis-ci.org/basiliscos/rust-procol-ftp-client.svg)

The FTP prorocol parser intended to be network transport layer neutral and suitable to use with standard rust
[TcpStream](https://doc.rust-lang.org/std/net/struct.TcpStream.html) 
as well as asynchronous framework such as [mio](https://github.com/carllerche/mio).

# Status

alpha

# Usage

To use `procol_ftp_client`, first add this to your `Cargo.toml`:

```toml
[dependencies]
protocol_ftp_client = "0.1"
```

See [example](https://github.com/basiliscos/rust-procol-ftp-client/blob/master/examples/ftp-get.rs) how to build ftp-get command using `TcpStream` of standard library

# API

[documentation](https://basiliscos.github.io/rust-procol-ftp-client/protocol_ftp_client/index.html)

# Licence

[MIT license](https://github.com/rust-lang/rust/blob/master/LICENSE-MIT) as Rust itself

# Author

Ivan Baidakou (basiliscos)
