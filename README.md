# Tree thinning

An exercise in appeasing the borrow checker in the context of transforming XML trees.

## Running this

You can run test with `cargo test` or run on an example XML file (while watching the memory management mechanics) with

```sh
RUST_LOG=debug cargo run
```

## Background

This came out of an ask for help from one of my friends, but became a multi-day puzzle which ultimately helped me understand better how the Rust borrow checker and memory management works. Hopefully the commit history might be interesting to someone.

Here is what I _think_ is happening:

### The exercise

The goal of the program is to parse an XML document and produce a "thinned" document tree - a tree where siblings of the same name are merged together recursively, producing a "maximum possible" tree with no duplicate siblings, effectively deriving the possible shape of the document, as far as is possible from the example given. The parser this uses is a streaming one, emmitting events as it finds opening and closing tags.

After trying in many ways, it seems like the only way to do this efficiently is to walk the XML tree and create nodes "on the way down", making sure that if we hit a node name we've alerady seen, we visit the existing node, instead of making a new one. That way the thinned tree is constructed in one pass.

The trouble is with the necessary book keeping in order to move up and down the tree.

### The problem

The initial stab at the implementation using mutable references and a stack of nodes to keep track of where in the tree we're currently located, so that when a node is finished, we can move up a level.

The main issue with this is with ownership - nodes own a `HashMap` of their children. The stack has mutable references to them and while walking the tree, we need to both _read_ nodes from the stack, _write_ (into) them in order to create new child nodes when discovered, and _write_ the stack in order to push a new child onto it. Borrow checker (correctly) isn't happy with this.

### The solution

In the end the problem with the reading and writing the stack got solved with `std::rc::Rc` - by holding `Rc<Node>` on the stack and in the children `HashMap` instead of a reference. When we take a node off the stack, find or create the child named as the one the parser has just come across, we can clone the `Rc` and decouple the child node from the stack it came from. This allows the borrow of the stack to be dropped, so we can borrow it again, this time, mutably and insert the child.

This of course makes it impossible to insert children, because [mutable borrow of Rc is not allowed][stack_overflow]. The solution to that side of the problem is the [interior mutability pattern][interior_mutability] using `std::cell::RefCell`. The children `HashMap` can be held inside a `RefCell`, allowing us to temporarily borrow it mutably even through an `Rc`.

[stack_overflow]: https://stackoverflow.com/questions/58599539/cannot-borrow-in-a-rc-as-mutable
[interior_mutability]: https://doc.rust-lang.org/std/cell/index.html#introducing-mutability-inside-of-something-immutable
