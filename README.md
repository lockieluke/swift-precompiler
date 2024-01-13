# Swift Precompiler

⚡ A fast, lightweight precompiler for Swift

### Features

- Add Rust's `includeStr!` like functionality to Swift with the `precompileIncludeStr` function

### Installation

Cargo:
```shell
cargo install swift-precompiler
```

### Configuration

Run `swift-precompiler init` to initialise a config file `swift-precompiled.toml` with the default values

Available options:
- `dirs` - An array of directories to search for Swift source files that require precompilation
- `path_aliases` - A dictionary of path aliases to use in precompile calls

Example:
```toml
dirs = ["Cider/", "CiderPlaybackAgent/"]

[path_aliases]
# "@" as a path alias refers to the current working directory in most cases
"@" = "./"
```

### Usage

Including a file as a string literal at compile time:
```swift
let javaScript = precompileIncludeStr("path/to/file.js")
```

Include a file as a Data at compile time:
```swift
let image = precompileIncludeData("path/to/image.png")
```

Run `swift-precompiler` to precompile all Swift files in the directories specified in the config file
```shell
swift-precompiler precompile
```

<sub>You should add `Precompiled.swift` to your `.gitignore`</sub>
