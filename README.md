🚀 Quick Start

```rust
use river_sdk::{register_plugin, client::{Filter, Config}};

struct MyFilter {
    name: String,
}

impl MyFilter {
    pub fn new(cfg: Config) -> Self {
        Self {
            name: cfg.get("name").cloned().unwrap_or("unknown".into()),
        }
    }
}

impl Filter for MyFilter {

    fn on_request(&mut self) -> Result<(), String> {
        println!("Filter {} handling request", self.name);
        Ok(())
    }
}

register_plugin!(
    "my_filter" => MyFilter::new
);
```

# 🛠️ Prerequisites & Setup
River SDK leverages the modern WASI Preview 2 standard. To compile your plugins, you need a recent Rust toolchain.
## 1. Install the Target
Rust 1.82+ supports the native wasm32-wasip2 target. Add it via rustup:
```bash
rustup target add wasm32-wasip2
```
## 2. Configure Cargo.toml
To generate a valid WebAssembly dynamic library, you must configure your crate type. Add this to your `Cargo.toml`:
```toml
[lib]
crate-type = ["cdylib"]
```
## 3. Build
Compile your plugin:
```bash
cargo build --release --target wasm32-wasip2
```
The resulting file will be located at `target/wasm32-wasip2/release/your_plugin_name.wasm`.

*See the [wit-bindgen](https://github.com/bytecodealliance/wit-bindgen) repository for further information.*

# ⚙️ Configuration & Running
Once you have your .wasm file, you need to configure River to load it. River uses KDL for configuration.

 - 1. Define the Plugin: In the definitions block, map a name to your .wasm file path.
 - 2. Use the Chain: In your service connectors, reference the filter using the format "plugin_name.filter_name".

Example `river.kdl`:
```kdl
system {
    threads-per-service 8
    daemonize #false 
    pid-file "/tmp/river.pidfile"
    upgrade-socket "/tmp/river-upgrade.sock"
}

definitions {
    plugins {
        // 1. Load the module
        plugin {
            name "example-plugin" // Namespace for this module
            load path="/path/to/target/wasm32-wasip2/release/my_plugin.wasm"
        }
    }
}

services {
    MyService {
        listeners {
            "0.0.0.0:8080"
        }
        connectors {
            section "/" {
                // 2. Apply the filter
                // Syntax: "plugin_definition_name.filter_registered_name"
                use-chain "example-plugin.my_filter"
                
                return code="200" response="OK"
            }
        }
    }
}
```
# 🚀 Running River
Start the server pointing to your configuration file:
```bash
river -- config-kdl=/path/to/river.kdl
```