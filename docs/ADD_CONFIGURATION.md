# Adding a configuration item

OpenNote defines its JSON configuration schema in `crates/opennote-models/src/configurations/`. Both applications use `configurations.json`, but load it from separate application directories.

## Choose the configuration scope

| Scope                                         | Add the field to                                              | JSON path        |
| --------------------------------------------- | ------------------------------------------------------------- | ---------------- |
| Shared startup setting for desktop and server | `SystemConfigurations` in `system.rs`                         | `system.<field>` |
| Desktop user setting                          | `UserConfigurations` in `user.rs`, or one of its nested types | `user.<field>`   |
| Desktop-only application setting              | `DesktopConfigurations` in `desktop.rs`                       | `<field>`        |
| Server-only setting                           | `ServerConfigurations` in `server.rs`                         | `<field>`        |

`SystemConfigurations` is shared and is intended for settings fixed at startup, such as storage or model selection. Do not put a desktop-only setting there.

For a group of related fields, define a separate type under `configurations/` or `configurations/fields/`, export its module from the corresponding `mod.rs`, and place that type in the parent configuration.

## How configurations are loaded

- Desktop: `GlobalApplicationBootStrap::load` in `crates/opennote-desktop/src/globals/bootstrap.rs` loads and migrates `DesktopConfigurations`, then `DesktopBootstrap::new` initializes dependent services.
- Server: `load_configurations` in `crates/opennote-server/src/initialization.rs` loads `ServerConfigurations`, then `ServerBootstrap::new` initializes dependent services.
- Persistence: `LoadFromAndSaveToFile` in `crates/opennote-models/src/traits.rs` creates a default file when none exists and uses Serde for JSON loading and saving.

The configuration directories come from `dirs::config_dir()`:

| Application | Directory name    |
| ----------- | ----------------- |
| Desktop     | `opennote`        |
| Server      | `opennote_server` |

See the root `README.md` and `crates/opennote-server/README.md` for platform-specific locations.

## Add the model field

Every configuration field must:

1. Be serializable and deserializable through its containing type.
2. Have an application default.
3. Have a Serde fallback so an existing configuration without the field can still load.

For example, a new shared setting can follow this pattern in `system.rs`:

```rust
const DEFAULT_REQUEST_TIMEOUT_SECONDS: u64 = 30;

fn default_request_timeout_seconds() -> u64 {
    DEFAULT_REQUEST_TIMEOUT_SECONDS
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemConfigurations {
    // Existing fields...

    #[serde(default = "default_request_timeout_seconds")]
    pub request_timeout_seconds: u64,
}

impl Default for SystemConfigurations {
    fn default() -> Self {
        Self {
            // Existing defaults...
            request_timeout_seconds: default_request_timeout_seconds(),
        }
    }
}
```

Use `#[serde(default)]` when the Rust `Default` value is exactly the desired application default. Use a named default function when it is not, or when making the persisted default explicit is clearer. Enums persisted as readable names should normally use `#[serde(rename_all = "snake_case")]` and implement `Default` when used with `#[serde(default)]`.

For a nested configuration type, derive `Serialize` and `Deserialize`, implement `Default`, and usually add `#[serde(default)]` to the type or its parent field. `UserSearchConfiguration` in `search.rs` is an existing example.

### Preserve old configuration files

Do not rely on migration alone to supply a newly required field:

- Deserialization happens before desktop migration, so a missing required field can prevent migration from running.
- Server startup currently loads without calling `MigrateConfigurationFileStructure::migrate`.

Therefore, add both the `Default` value and a Serde default for every new field. Desktop migration will then save the completed structure. The server will use the fallback in memory; if the server must also rewrite the file, call `.migrate(&config_path)` during server loading and import `MigrateConfigurationFileStructure`.

Avoid renaming or removing persisted keys without an explicit compatibility plan. For a rename, consider `#[serde(alias = "old_name")]`; for a Rust-only rename that must preserve JSON, use `#[serde(rename = "existing_name")]`.

## Validate values

Serde validates JSON shape and types, not application constraints. If a value has limits or related fields must agree:

1. Add a `validate` method to the relevant configuration type.
2. Propagate it through the parent configuration if needed.
3. Call it explicitly at every input boundary:
   - desktop startup in `globals/bootstrap.rs`;
   - desktop editor parsing in `views/settings.rs` before saving;
   - server startup in `initialization.rs` before returning the configuration.

A method named `validate` is not called automatically. Return contextual errors that identify the invalid key and accepted values.

## Connect the field to behavior

Adding the model only changes the JSON schema. Read and apply the value where the feature runs.

### Desktop

Desktop code can access the current model through `GlobalApplicationBootStrap`:

```rust
let bootstrap: &GlobalApplicationBootStrap = cx.global();
let configurations = bootstrap.get_configurations();
let timeout = configurations.system.request_timeout_seconds;
```

Copy or clone the needed values before entering asynchronous work so the configuration lock is not held longer than necessary.

The JSON settings editor in `crates/opennote-desktop/src/views/settings.rs` serializes the complete `DesktopConfigurations`, so no field-specific editor registration is required. Saving replaces the in-memory model and writes it to disk, but it does not generally rebuild resources created by `DesktopBootstrap::new`. For startup settings, require an application restart. For a live setting, add the state-update or resource-rebuild path explicitly.

### Server

Startup code can read the loaded `ServerConfigurations` directly. Request handlers can access the shared model through `actix_web::web::Data<ServerBootstrap>`:

```rust
let configurations = data.configurations.lock().await;
let timeout = configurations.system.request_timeout_seconds;
```

Copy or clone the value and release the lock before long asynchronous operations. Server configuration is loaded once, so changes to `configurations.json` require a restart unless a reload mechanism is implemented.

## Update user documentation

If users can set the new item, document its JSON path, type, default, valid range, whether it is desktop/server/shared, and whether a restart is required. Update:

- `README.md` for desktop-facing settings;
- `crates/opennote-server/README.md` for server-facing settings;
- deployment examples when the setting affects containers or environment variables.

Do not put real credentials or production secrets in defaults, tests, or examples.

## Verify the change

Test all affected scopes from the repository root:

```sh
cargo fmt --all --check
cargo test -p opennote-models
cargo check -p opennote-models -p opennote-bootstrap -p opennote-server -p opennote-desktop
```

Also verify these cases manually or with serialization tests:

- No file: a valid file containing the new default is generated.
- Existing file without the new key: loading succeeds and uses the default.
- Explicit value: JSON round-trips without changing the value.
- Invalid value: validation returns a useful error.
- Desktop editor: the field can be saved and reloaded.
- Runtime: both applications apply the value at the intended time, including any documented restart.
