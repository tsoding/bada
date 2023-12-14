# Bada

Simple Programming Language that compiles to BEAM bytecode.

## Quick Start

Install dependencies:
- [Rust](https://www.rust-lang.org/)
- [Erlang](https://www.erlang.org/)

Compile the Compiler (we don't use Cargo because we don't have any thirdparty dependencies yet):

```console
$ rustc ./src/bada.rs
```

Compile an example using the Compiler:

```console
$ ./bada ./examples/bada.boom
```

Load the example into Erlang environment:

```console
$ erl
> code:add_path("./examples/").
> code:load_file(bada).
> bada:hello().
> bada:world().
```

To reload the Example module:

```console
> code:purge(bada), code:load_file(bada).
```
