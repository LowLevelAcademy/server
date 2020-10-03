# Low-Level Academy backend

This repository contains the code for the [Low-Level Academy](https://lowlvl.org) backend.

Related repositories:

- [Front-end code](https://github.com/LowLevelAcademy/LowLevelAcademy)
- [WebAssembly modules](https://github.com/LowLevelAcademy/wasm-modules)

It handles users' compilation requests and returns resulting WebAssembly files.

## Build instructions

Before starting the server, you might need to pull the Docker image used for compilation:

```
docker pull lowlvl/playground
```

You can also build it locally from [sources](./deploy):

```
cd deploy && bash build.sh
```

You can start the backend using the following command:

```
cargo run --release
```

If you want to run it in the development mode, use

```
ROCKET_ENV=development cargo run --release
```

## License

This code is partially based on the [Rust playground backend](https://github.com/integer32llc/rust-playground/) which is authored by Jake Goulding.

Code in this repository is licensed under either of

- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
