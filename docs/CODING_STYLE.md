# Coding Style

1. Use `match` when possible.
2. Use early return when possible.
3. Use verbs for functions and methods names.
4. Prefer using `?` to handle errors. Discourage excessive usage of `.context()` and `.with_context()` etc.
5. Send errors with explicit `return Err()`. Discourage usage of `bail!`, `ensure!` and so on.
