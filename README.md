# MTProto-rs

[MTProto](https://core.telegram.org/mtproto) protocol and schema
implementation in Rust.

Intended to provide low-level features to create a robust foundation for
higher-level libraries such as `telegram-rs`.

Supports Rust 1.19 or newer.
Older versions may work, but not guaranteed to.


## Features

Currently implemented and planned features include:

- [x] Code autogeneration for TL-schema
      (implemented in [`tl_codegen`][tl_codegen_code])
- [x] MTProto binary [de]serialization
      (handled by [`serde_mtproto`][serde_mtproto_repo])
- [ ] Encryption facilities which enforce
      [security guidelines][mtproto_security_guidelines]
- [ ] Key exchange
- [ ] Seamless RPC:
    * Schema functions are modeled as structs
    * Sending requests and receiving responses are automatically
      provided by associated methods
- [ ] Handling sessions and messages

[tl_codegen_code]: https://github.com/Connicpu/mtproto-rs/tree/master/tl_codegen
[serde_mtproto_repo]: https://github.com/hcpl/serde_mtproto
[mtproto_security_guidelines]: https://core.telegram.org/mtproto/security_guidelines


## Examples

There are 3 examples which you can build and run:

### `tcp_auth`

Fetches authorization key over TCP. Supports 3 modes: abridged,
intermediate and full (this example uses all three).

Based on [tokio](https://tokio.rs).

```sh
$ cargo run --example tcp_auth
```

### `http_auth`

Same as `tcp_auth` but over HTTP which only has 1 mode.

Based on [tokio](https://tokio.rs) and [hyper](https://hyper.rs).

```sh
$ cargo run --example http_auth
```

### `dynamic`

Dynamic typing using `TLObject` in action.

```sh
$ cargo run --example dynamic
```

You can also look at [tests](./tests/) for more use cases which are automatically tested.


## License

MTProto-rs is licensed under either of

 * Apache License, Version 2.0, ([LICENSE_APACHE](LICENSE_APACHE) or
   http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE_MIT](LICENSE_MIT) or
   http://opensource.org/licenses/MIT)

at your option.
