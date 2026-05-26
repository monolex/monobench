Serializing a data structure to JSON crashes — a segfault / use-after-free — when one of the values
being serialized runs Python code during serialization (a custom serializer hook, a value's
`__str__`/`__repr__`, or a mapping whose iteration triggers code) and that code mutates or clears the
very container being serialized (e.g. empties the list/dict that is mid-encode).

The serializer keeps walking items that were just freed. With a static structure, or without the
mutation, it never crashes. The faulting frame is inside the C serialization loop, dereferencing an
element that no longer exists.

End your reply with exactly:
ROOTCAUSE: <file path>::<function>
FIX: <one sentence>
