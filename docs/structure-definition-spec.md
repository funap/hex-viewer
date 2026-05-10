# Binary Structure Definition Specification (Kaitai Struct)

XVI utilizes **Kaitai Struct** (.ksy) as its primary format for defining and parsing binary data structures.

## Overview

Kaitai Struct is a declarative language used to describe various binary data structures. XVI implements a dynamic interpreter that allows you to load `.ksy` files at runtime and visualize the structure of the binary file you are editing.

## Supported Features

- **Sequential Fields (`seq`)**: Define fields in order.
- **Dynamic Types (`types`)**: Define custom structures for reuse.
- **Conditional Fields (`if`)**: Parse fields only if a condition is met.
- **Repetitions (`repeat`)**:
  - `expr`: Repeat a fixed number of times.
  - `until`: Repeat until a condition is met.
  - `eos`: Repeat until the end of the stream.
- **Enums (`enums`)**: Map numeric values to human-readable labels.
- **Endianness Control**: Support for both Little Endian (`le`) and Big Endian (`be`).

## Example Definition (.ksy)

```yaml
meta:
  id: custom_protocol
  endian: be
seq:
  - id: version
    type: u1
  - id: flags
    type: u1
  - id: payload_length
    type: u2
  - id: payload
    size: payload_length
  - id: extended_header
    type: ext_header
    if: version >= 2
  - id: checksum
    type: u4
types:
  ext_header:
    seq:
      - id: ext_type
        type: u2
      - id: ext_length
        type: u2
      - id: ext_data
        size: ext_length
```

## How to use in XVI

1. Create a `.ksy` file using YAML syntax.
2. Open the file in XVI via **Action: Load Structure Definition**.
3. The structure will be displayed in the left panel and highlighted in the HexView.
4. Clicking a field in the tree will jump the cursor to that offset.
