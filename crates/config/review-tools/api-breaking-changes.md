---
kind: tool
name: API breaking changes
---
Public shapes others depend on that the change alters or removes: wire fields and their names, exported types and signatures, command line flags, file formats a caller reads, configuration keys, database columns. Search the checkout for the callers and consumers of each shape. Report a break only when something that exists today stops working, and name it.
