# Stack local-drop filesystem fixture

From the repository root on Windows:

```powershell
cargo test --offline --manifest-path tests/native/stack-local-drop/Cargo.toml --target-dir node_modules/.cache/stack-local-drop-native
```

Omit `--offline` if the project's Windows crate is not cached. The fixture imports the production move module with stand-in result types. It operates only in disposable temporary directories and does not build or open JasonShell. Windows with junction support in the temporary directory is required; junction setup failure fails the test rather than silently skipping it. Non-Windows runs execute zero tests.

Checks: both files move with original bytes and no source copies; name conflicts preserve both files; replacing the destination with a junction before commit leaves the source intact and does not redirect the move. A separate check holds the actual production destination handle: a competing write open succeeds, but a competing rename fails with a sharing violation until the handle is released. While the handle is held, the production rename helper rejects a non-leaf name and successfully moves a file into the verified directory using a leaf name. The fixture does not mutate a reparse point in place while the handle is held.
