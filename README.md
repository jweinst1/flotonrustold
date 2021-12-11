# FlotonRustOld

This was the first implementation of a floton - wait free database in Rust. It features a wait free shared resource container, a wait free hash map implementation, as well as wait free spsc queues, and a rotating job executor. The main limitation of this earlier implemtation was a lack of a wait free memory allocator, and Rust's restrictions on singletons.

### Tests

Unit tests must be run with the cargo test command:

```
$ cargo test
```
