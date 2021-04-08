# Learning Rust

All the tool does right now is visit urls and get the request data. It could be HTML or JSON responses from API calls.
It has a `profile` flag to profile its own performance.

Apart from learning Rust, the "difficulty" of this tool, so to speak, is in attempting to use low level primitives to create and send my own HTTP GET requests 
over a TCP Stream.


**Version information**:
`
rustc 1.47.0 (18bf6b4f0 2020-10-07)
cargo 1.47.0 (f3c7e066a 2020-08-28)
`

I've named the tool **visit**, so it  would be (profile flag is optional):

`visit.exe --url <the-url> --profile <count>`
