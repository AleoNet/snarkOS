# Contributing

Thank you for your interest in contributing to snarkOS! Below you can find some guidelines that the project strives to follow.

## Pull requests

Please follow the instructions below when filing pull requests:

- ensure that your branch is forked from the current [master](https://github.com/AleoHQ/snarkOS/tree/master) branch
- run `cargo fmt` before you commit; we use the `nightly` version of `rustfmt` to format the code, so you'll need to have the `nightly` toolchain installed on your machine; there's a [git hook](https://git-scm.com/docs/githooks) that ensures proper formatting before any commits can be made, and [`.rustfmt.toml`](https://github.com/AleoHQ/snarkOS/blob/master/.rustfmt.toml) specifies some of the formatting conventions
- run `cargo clippy --all-targets --all-features` to ensure that popular correctness and performance pitfalls are avoided

## Coding conventions

snarkOS is a big project, so (non-)adherence to best practices related to performance can have a considerable impact; below are the rules we try to follow at all times in order to ensure high quality of the code:

### Memory handling
- if the final size is known, pre-allocate the collections (`Vec`, `HashMap` etc.) using `with_capacity` or `reserve` - this ensures that there are both fewer allocations (which involve system calls) and that the final allocated capacity is as close to the required size as possible
- create the collections right before they are populated/used, as opposed to e.g. creating a few big ones at the beginning of a function and only using them later on; this reduces the amount of time they occupy memory
- if an intermediate vector is avoidable, use an `Iterator` instead; most of the time this just amounts to omitting the call to `.collect()` if a single-pass iteraton follows afterwards, or returning an `impl Iterator<Item = T>` from a function when the caller only needs to iterate over that result once
- when possible, fill/resize collections "in bulk" instead of pushing a single element in a loop; this is usually (but not always) detected by `clippy`, suggesting to create vectors containing a repeated value with `vec![x; N]` or extending them with `.resize(N, x)`
- when a value is to eventually be consumed in a chain of function calls, pass it by value instead of by reference; this has the following benefits:
  * it makes the fact that the value is needed by value clear to the caller, who can then potentially reclaim it from the object afterwards if it is "heavy", limiting allocations
  * it often enables the value to be cloned fewer times (whenever it's no longer needed at the callsite)
  * when the value is consumed and is not needed afterwards, the memory it occupies is freed, improving memory utilization
- if a slice may or may _not_ be extended (which requires a promotion to a vector) and does not need to be consumed afterwards, consider using a [`Cow<'a, [T]>`](https://doc.rust-lang.org/std/borrow/enum.Cow.html) combined with `Cow::to_mut` instead to potentially avoid an extra allocation; an example in snarkOS could be conditional padding of bits
- prefer arrays and temporary slices to vectors where possible; arrays are often a good choice if their final size is known in advance and isn't too great (as they are stack-bound), and a small temporary slice `&[x, y, z]` is preferable to a `vec![x, y, z]` if it's applicable
- if a reference is sufficient, don't use `.clone()`/`to_vec()`, which is often the case with methods on `struct`s that provide access to their contents; if they only need to be referenced, there's no need for the extra allocation
- use `into_iter()` instead of `iter().cloned()` where possible, i.e. whenever the values being iterated over can be consumed altogether
- if possible, reuse collections; an example would be a loop that needs a clean vector on each iteration: instead of creating and allocating it over and over, create it _before_ the loop and use `.clear()` on every iteration instead
- try to keep the sizes of `enum` variants uniform; use `Box<T>` on ones that are large

### Misc. performance

- avoid the `format!()` macro; if it is used only to convert a single value to a `String`, use `.to_string()` instead, which is also available to all the implementors of `Display`
- don't check if an element belongs to a map (using `contains` or `get`) if you want to conditionally insert it too, as the return value of `insert` already indicates whether the value was present or not; use that or the `Entry` API instead
- if a reference is sufficient as a function parameter, use:
  * `&[T]` instead of `&Vec<T>`
  * `&str` instead of `&String`
  * `&Path` instead of `&PathBuf`
- if a lot of computational power is needed, consider parallelizing that workload with [`rayon`](https://crates.io/crates/rayon) - it's not always a viable solution, but can yield great performance improvements when used in the right context
- for `struct`s that can be compared/discerned based on some specific field(s), consider hand-written implementations of `PartialEq` **and** `Hash` ([they must match](https://doc.rust-lang.org/std/hash/trait.Hash.html#hash-and-eq)) for faster comparison and hashing
- if possible, ensure that the results of your changes are not detrimental to performance using [`criterion`](https://crates.io/crates/criterion) (for smaller, fine-grained adjustments) and [`valgrind --tool={cachegrind | massif}`](https://valgrind.org/info/tools.html) (for larger-scale changes)
