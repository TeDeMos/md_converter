# md_converter

App to convert between different markup formats.

## Installation

Compile the app from source using cargo

```
cargo build --release
```

Compiled program will end up in `/target/release`

## Usage

Print help:

```
md_converter.exe --help
```

Convert a given file:

```
md_converter.exe --from <INPUT_FORMAT> --to OUTPUT_FORMAT <FILE>
```

Pipe content to convert:

```
<command> | md_converter.exe --from <INPUT_FORMAT> --to OUTPUT_FORMAT <FILE>
```

Convert input until EOF character (ctrl-Z on Windows, ctrl-D on Linux):

```
md_converter.exe --from <INPUT_FORMAT> --to OUTPUT_FORMAT <FILE>
```

## Docs

Generate docs of public modules:

```
cargo doc
```

Additionally, some private modules also contain documentation for explanation. To see them as well:

```
cargo doc --document-private-items
```

## Tests

To see all available tests run:

```
cargo test -- --list
```

Tests outside of `md_teader::tests` are regular tests. Tests directly inside this module require pandoc
to be installed. They are not supposed to pass. They are used as a direct comparison
to pandoc.

 