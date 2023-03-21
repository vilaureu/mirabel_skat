# Skat for Mirabel/Surena

This is an implementation of
[_Skat_](<https://en.wikipedia.org/wiki/Skat_(card_game)>) for the
[_surena_](https://github.com/RememberOfLife/surena) game engine and the
[_mirabel_](https://github.com/RememberOfLife/mirabel) game GUI.

# Surena Usage Example

```
$ surena repl
> /load_plugin ./target/debug/libmirabel_skat.so
> /create def
> /resolve_random
```

## Libraries

This project uses the following libraries:

- [_nom_](https://github.com/rust-bakery/nom) under the
  [_MIT License_](https://github.com/rust-bakery/nom/blob/main/LICENSE)
- [_mirabel_rs_](https://github.com/vilaureu/mirabel_rs) under the
  [_MIT License_](https://github.com/vilaureu/mirabel_rs/blob/main/LICENSE)

## License

See the `LICENSE` file.
