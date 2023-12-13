# Bada

Simple Programming Language that compiles to BEAM bytecode.

## Quick Start

```console
$ rustc bada.rs
$ cat > bada.boom <<END
hello = 69
world = 420
END
$ ./bada ./bada.boom
$ erl
1> code:load_file(bada).
2> bada:hello().
3> bada:world().
```
