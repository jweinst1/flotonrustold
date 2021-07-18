# Floton

*The database of ultimate freedom*

### Tests

Unit tests must be run with a single thread, due to atomic states shared between threads, with the command:

```
$ cargo test -- --test-threads=1
```