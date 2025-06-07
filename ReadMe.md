| From                                   | To                                                                                                  | Relation     |
|----------------------------------------|-----------------------------------------------------------------------------------------------------|--------------|
| any type                               | same type                                                                                           | ↔ (identity) |
| any type                               | `nil`                                                                                               | →            |
| `nil`                                  | `Object`                                                                                            | →            |
| **Numeric clique**                     |                                                                                                     |              |
| `bool`                           | `int`                                                                                               | ↔            |
| `bool`                           | `float`                                                                                             | ↔            |
| `int`                            | `float`                                                                                             | ↔            |
| **Stringish types**                    |                                                                                                     |              |
| `string`                         | `StringName`                                                                                        | ↔            |
| `string`                         | `NodePath`                                                                                          | ↔            |
| *(StringName & NodePath have no direct edge)* |                                                                                             |              |
| **Vector/integral pairs**              |                                                                                                     |              |
| `Vector2`                        | `Vector2I`                                                                                          | ↔            |
| `Vector3`                        | `Vector3I`                                                                                          | ↔            |
| `Vector4`                        | `Vector4I`                                                                                          | ↔            |
| **Rect/integral pairs**                |                                                                                                     |              |
| `Rect2`                          | `Rect2I`                                                                                            | ↔            |
| **Transforms & rotations**             |                                                                                                     |              |
| `Transform2D`                    | `Transform3D`                                                                                       | ↔            |
| `Transform3D`                    | `Projection`                                                                                        | ↔            |
| `Quaternion`                     | `Basis`                                                                                             | ↔            |
| `Quaternion`                     | `Transform3D`                                                                                       | →            |
| `Basis`                          | `Transform3D`                                                                                       | →            |
| **Color sink**                         |                                                                                                     |              |
| `string`                         | `Color`                                                                                             | →            |
| `int`                            | `Color`                                                                                             | →            |
| **Object & resource**                  |                                                                                                     |              |
| `Object`                         | `Rid`                                                                                               | →            |
| **Arrays & packed arrays**             |                                                                                                     |              |
| `Array`                          | each `PackedByteArray`, `PackedInt32Array`, `PackedInt64Array`, `PackedFloat32Array`, `PackedFloat64Array`, `PackedStringArray`, `PackedColorArray`, `PackedVector2Array`, `PackedVector3Array`, `PackedVector4Array` | ↔ |
