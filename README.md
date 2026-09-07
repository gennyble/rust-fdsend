Attempting to send File Descriptors over Unix Sockets in Rust.

References [this gist from domfarolino](https://gist.github.com/domfarolino/4293951bd95082125f2b9931cab1de40)

---

The binary that's generated is called `january`. It expects to run in debug, so just do this to see it in action:
```
cargo run -- send
```

Also: The unix socket file is not cleaned up. In between tests you should `rm socket`.

---

**What's happening?**

In `send`:
- It opens a Unix Socket (with `UnixListener`) with a path of `socket`.
- Launches the `recv` instance using `std::process::Command`. The actual binary it's running is `target/debug/january` with `recv` as the first and only argument
- Opens a file (just with `File::open`) and reads the first three bytes, printing them to prove it.
- Turns the `File` into a raw file descriptor with `File::into_raw_fd`.
- Creates the neccesary message/control message structures that we're going to send. This is the first place `libc` is used and where the file descriptor we're sending is input.
- Waits for `recv` to connect over a Unix Socket and immediatly grabs a reference to the raw fd with `UnixStream::as_raw_fd`
- Calls `libc::sendmsg` passing the raw fd of the accepted Unix Socket and the message we created earlier.
- Waits 2 seconds for `recv` to read the message.

In `recv`:
- Attempts to connect to `send` through the Unix Socket it opens and converts it into a raw fd on success.
- Create the neccesary structures for receiving the message/control message.
- Calls `libc::recvmsg` passing the raw fd of the unix socket and a pointer to the message structures.
- Extracts the file descriptor from the control message and uses `File::from_raw_fd` to get a Rust type back.
- Reads the *next* three bytes of the file, printing them.
