# md_coverter

App to convert between different markup formats.

## Instalation

Compile the app from source using cargo:

Windows:

`cargo build --release `

## Usage

## Docs

Generate the docs with

```
cargo doc
```

To see documentation of private items add the `--document-private-items` flag

```
cargo doc --document-private-items
```

## Tests

To see all available tests run:

```
cargo test -- --list
```

Tests outside of `md_teader::tests` are regular tests. Tests directly inside this module require pandoc
to be installed. They are not supposed to pass. They are used as a direct comparision
to pandoc.

 