# Adding and using a UI language

OpenNote embeds its UI translations at compile time. Each language is a JSON object loaded into an in-memory `HashMap<String, String>` when the desktop application starts.

## How language lookup works

- Translation files live in `assets/languages/`.
- `crates/opennote-desktop/src/globals/assets.rs` embeds every JSON asset and loads files below `languages/` into `AssetsCollection::language_profiles`.
- The filename without `.json` is the profile name. For example, `languages/english.json` becomes `english`.
- `crates/opennote-desktop/src/globals/helpers.rs::get_language_profile` receives the GPUI app context, reads `GlobalApplicationBootStrap` and `AssetsCollection` from it, converts the configured `UserInterfaceLanguage` to a string, and looks up the profile with that name.

The configured language name and JSON filename must therefore match exactly.

## Add a new language

The examples below use French.

### 1. Create the translation profile

Copy `assets/languages/english.json` to:

```text
assets/languages/french.json
```

Translate every value, but keep every key unchanged:

```json
{
  "sidebar_title": "Notes",
  "search_bar_placeholder": "Rechercher...",
  "command_bar_placeholder": "Saisissez votre commande...",
  "default_block_title": "Sans titre"
}
```

The shortened object above is only an example. A real profile must contain all keys from `english.json`, including action keys such as `workspace::ToggleSidebar`.

Important rules:

- Use a lowercase, snake-case filename.
- Keep the file as a flat JSON object whose keys and values are strings.
- Keep the same set of keys in every language profile.
- Do not rename translated keys; only translate their values.
- Ensure the file is valid JSON. A malformed embedded language file prevents `AssetsCollection` from loading at application startup.

No manual asset registration is needed. The `rust-embed` configuration in `assets.rs` includes JSON files automatically, and `AssetsCollection::load` discovers files under `languages/`.

### 2. Register the language in the configuration model

Edit `crates/opennote-models/src/configurations/language.rs` and add a variant:

```rust
pub enum UserInterfaceLanguage {
    Chinese,
    English,
    French,
}
```

Then add the exact profile name to its `Display` implementation:

```rust
impl Display for UserInterfaceLanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UserInterfaceLanguage::Chinese => f.write_str("chinese"),
            UserInterfaceLanguage::English => f.write_str("english"),
            UserInterfaceLanguage::French => f.write_str("french"),
        }
    }
}
```

This mapping is the important contract:

```text
UserInterfaceLanguage::French -> "french" -> assets/languages/french.json
```

`UserInterfaceLanguage` uses Serde's `snake_case` representation, so persisted configuration for this example is also `french`.

If the application exposes a list of languages in settings or another UI, add the new variant to that list as well.

## Add a new translated string

Choose a descriptive snake-case key and add it to **every** file in `assets/languages/`:

```json
{
  "settings_title": "Settings"
}
```

```json
{
  "settings_title": "设置"
}
```

Missing keys currently cause a runtime panic because call sites index the profile directly, for example `language_profile["settings_title"]`. Treat `english.json` as the canonical key list and keep all profiles synchronized.

Action labels are a special case: `match_action_to_language` looks up `Action::name()` directly. Their translation keys must exactly match the action name, including its namespace and capitalization:

```json
{
  "workspace::ToggleSidebar": "Toggle sidebar"
}
```

## Use a translation in desktop code

Import the helper:

```rust
use crate::globals::helpers::get_language_profile;
```

In code with a GPUI context, pass the context directly to retrieve the active profile:

```rust
let language_profile = get_language_profile(cx)
    .context("Getting language profile failed")?;

let title = language_profile["settings_title"].clone();
```

`get_language_profile` reads both `GlobalApplicationBootStrap` and `AssetsCollection` from the supplied context, so callers do not need to retrieve or pass those globals separately.

Use the translated value in the component:

```rust
Label::new(&language_profile["settings_title"])
```

Existing code sometimes uses `unwrap()` instead of propagating the error. Follow the error-handling style of the surrounding function, but include enough context when the function can return an error.

`GlobalApplicationBootStrap` and `AssetsCollection` are already initialized in `crates/opennote-desktop/src/main.rs` before the UI state is created, so normal desktop components can pass their GPUI context directly to `get_language_profile(cx)`.

## Validate the change

1. Confirm that the new JSON parses:

   ```sh
   python3 -m json.tool assets/languages/french.json >/dev/null
   ```

2. Compare the keys with the canonical English profile:

   ```sh
   diff \
     <(python3 -c 'import json; print("\\n".join(sorted(json.load(open("assets/languages/english.json")))))') \
     <(python3 -c 'import json; print("\\n".join(sorted(json.load(open("assets/languages/french.json")))))')
   ```

   No output means that the profiles contain the same keys. This command requires a shell that supports process substitution, such as Bash or Zsh.

3. Format and check the affected Rust crates from the repository root:

   ```sh
   cargo fmt --all --check
   cargo check -p opennote-models -p opennote-desktop
   ```

4. Start the desktop application with its user language set to the new variant and inspect every changed label. Translation files are embedded at compile time, so rebuild the application after editing them.
