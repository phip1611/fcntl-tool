# fcntl-tool

Your small yet useful swiss army knife for the `fcntl` system call,
specifically for acquiring and testing file locks, but not limited to those.

This tool only works on UNIX or POSIX-like systems. Please find more
information in the following resources:

- <https://man7.org/linux/man-pages/man2/fcntl.2.html>
- <https://apenwarr.ca/log/20101213>

## 🔍 What Problem Does It Solve?

This is for example useful for testing during development. Imagine you want
to check how your program behaves, when a lock is already held. Using
`fcntl-tool`, you can acquire these locks!

## Supported Platforms

This crate works on all platforms that Rust code can be compiled to. It,
however, only makes sense on UNIX or POSIX-like systems.

## Supported `fcntl` Operations

| operation     | supported |
|---------------|-----------|
| `F_GETLK`     | ✅         |
| `F_SETLK`     | ✅         |
| `F_OFD_GETLK` | ✅         |
| `F_OFD_SETLK` | ✅         |
| ...           | Not yet   |


## CLI Usage

Type `fcntl-tool --help` to get guidance.

## Example

```console
$ fcntl-tool --help

# Terminal 1
$ fcntl-tool write-lock ./foo.txt
Please press enter to release the Write lock ...


# Terminal 2
$ fcntl-tool test-lock ./foo.txt pull
state: ExclusiveWrite
```

You can also take a look onto the integration test in the repository.

## MSRV

The MSRV of the binary is `1.85.1` stable.
