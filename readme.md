# ran

ran (pronounced "rAen"), short for "Run Anything Now", is a command-line launcher tool for launching games and applications. It uses application definition files (TOML) to define how to launch applications, and supports features like command-line arguments, environment variables, and more.

## Features

- Launching games and applications from the command line
- TOML-based application definition files
- Custom globally-defined variables for use in application definitions
- Environment overriding (global and per-application)
- Support for command-line arguments passing
- Cross-platform support (Windows, macOS, Linux)
- Application aliases (and aliases for aliases)
- Config directory override (using $RANCFG)
  etc.

## Installation

Since ran is a single executable, you can just download it from the [releases page](https://github.com/hasibix/ran/releases/latest) and add it to your PATH to "install" ran.

Or you can skip adding it to PATH and just use from the directory you downloaded it into.

Alternatively, you can clone this repository using git and compile it yourself and install it by running:

```
cargo install --path .
```

(assuming you have Rust set up.)

## Usage

To use RAN, you need to create application definition files in the `apps` directory. Simply run:

```
ran app create <full name for app, e.g. games/mygame>
```

This should create a template app definition in <config location>/apps/<full name for app>.toml and open it inside your preferred text editor.
After you modify the template, save the file and exit. Now run:

```
ran launch <full name, alias (defined in config.toml), or simple name>
```

to launch your application.

For more details, you can run `ran help` to get info about ran or a specific command.

# License

```
 	 Copyright 2026 Hasibix Hasi

   Licensed under the Apache License, Version 2.0 (the "License");
   you may not use this file except in compliance with the License.
   You may obtain a copy of the License at

       http://www.apache.org/licenses/LICENSE-2.0

   Unless required by applicable law or agreed to in writing, software
   distributed under the License is distributed on an "AS IS" BASIS,
   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
   See the License for the specific language governing permissions and
   limitations under the License.

```

For more details, consult the [license file](https://github.com/hasibix/ran/blob/main/license).
