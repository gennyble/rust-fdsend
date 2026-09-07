Attempting to send File Descriptors over Unix Sockets in Rust.

References [this gist from domfarolino](https://gist.github.com/domfarolino/4293951bd95082125f2b9931cab1de40)

---

The binary that's generated is called `january`. It expects to run in debug, so just do this to see it in action:
```
cargo run -- send
```

Also: The unix socket file is not cleaned up. In between tests you should `rm socket`.
