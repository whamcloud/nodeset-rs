[![crates.io](https://img.shields.io/crates/v/nodeset.svg)](https://crates.io/crates/nodeset)
[![docs.rs](https://docs.rs/nodeset/badge.svg)](https://docs.rs/nodeset)

# Description

`nodeset` is a Rust library and command-line tool (`ns`) that deals with
nodesets. It is heavily inspired by
[clustershell](https://cea-hpc.github.io/clustershell/) and aims at supporting
the same nodeset representation in Rust. It also provides a C API for use in
other languages.

A nodeset is a set of names which are generally indexed over one or more integer
dimensions such as `node1` or `r1sw2-port0`. Large nodesets can be represented
in a compact way using a bracket notation such as `node[1-1000]`. When brackets
are used in multiple dimensions, it implies a cartesian product. For example:
`r[1-2]sw[1-2]-port[1-2]` represents 8 ports:
`r1sw1-port1,r1sw1-port2,r1sw2-port1,...,r2sw2-port2`.

`ns` allows to fold or expand nodesets, as well as to perform algebraic
operations on them (union, intersection, difference, ...)

# Command line examples

- Listing nodes:

```bash
$ ns list r[2-4/2]sw1-port[23-24]
r2esw1-port23 r2sw1-port24 r4sw1-port23 r4sw1-port24
```

- Folding nodes:

```bash
$ ns fold r2esw1-port23 r2sw1-port24 r4sw1-port23 r4sw1-port24
r[2,4]esw1-port[23-24]
```

- Counting nodes:

```bash
$ ns count r[2-4/2]esw1-port[23-24]
4
```

- Algebraic operations using operators:

```bash
$ ns fold 'node[0-10] - (node[0-5] node[7-10])'
node6

$ ns fold 'node[1-2] ^ node[2-3]'
node[1,3]
```

# Configuration files and groups

`ns` understands and uses clustershell's configuration files in which node
groups can be defined. Please refer to clustershell's documentation for a full
description of the configuration files syntax.

# Library usage example

To compute and display the intersection of two nodesets

```rust
    use nodeset::NodeSet;

    let ns1: NodeSet = "node[01-15]".parse().unwrap();
    let ns2: NodeSet = "node[10-30/2]".parse().unwrap();
    let inter = ns1.intersection(&ns2);
    assert_eq!(inter.to_string(), "node[10,12,14]");
```

# C bindings

Along with the CLI binary (`ns`), running `cargo build --all` from the crate
root generates the `libnodeset.so` C dynamic library. The C API is described in
the [nodeset.h](nodeset-capi/include/nodeset.h) header.

The following example shows how to iterate over a nodeset. More complete
examples are available in the [examples](nodeset-capi/examples) directory.

```C
#include <stdio.h>
#include "nodeset-capi/include/nodeset.h"

int main(int argc, char **argv)
{
    NodeSet *nodeset;
    NodeSetIter *iter;
    char *node;

    /* Parse a nodeset */
    if ((nodeset = ns_parse("node[1-5]&node[5-10],node4", NULL)) == NULL)
    {
        return 1;
    }

    /* Iterate over the nodes */
    iter = ns_iter(nodeset);
    while ((node = ns_iter_next(iter, NULL)) != NULL)
    {
        printf("%s\n", node);
        ns_free_node(node);
    }

    /* Check whether the iterator ended due to an error */
    if (ns_iter_status(iter) != 0)
    {
        return 1;
    }

    /* Free the iterator and nodeset */
    ns_free_iter(iter);
    ns_free_nodeset(nodeset);

    return 0;
}
```

# Why use this crate

This project is not as mature as clustershell and has fewer features. However,
compared to clustershell's `nodeset` tool written in Python, `ns` starts much
faster as it doesn't rely on an interpreter. It can save a lot of time when
performing multiple operations on nodesets in a shell script. It is also faster
at parsing and performing operations on large nodesets. Last but not least, the
library crate can be used to handle nodesets natively in Rust or C while sharing
group definitions with clustershell.

## Similar rust projects

- [nodeagg](https://crates.io/crates/nodeagg)
- [hostlist](https://crates.io/crates/hostlist)
- [hostlist-parser](https://crates.io/crates/hostlist-parser)
