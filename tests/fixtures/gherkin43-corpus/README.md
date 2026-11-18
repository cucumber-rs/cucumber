# Complete pinned plain-text official corpus

Repository cucumber/gherkin, tagv43.0.0,
commit3d70e2bb4e0f16a568ee539a5ce0b03fb247a777.

All50 testdata/good/*.feature and12 testdata/bad/*.feature inputs are included,
with their published AST/Pickle/error NDJSON expectations (175 artifacts total).
Markdown .feature.md fixtures, tokens and source-envelope fixtures are outside
this explicit plain-text AST/Pickle/native-rejection contract.

manifest.json records the complete selected Git-tree paths/blobSHA and adopted
SHA256. Sources were bulk downloaded from the pinned official source archive.
Three *.crlf.feature files are CRLF in the official checkout/export, while Git
stores normalizedLF: upstream testdata/.gitattributes is retained, and their
normalized bytes were compared with the Git blob identity. All other adopted
files match their raw Git blobs. No oracle expectation has been edited.

Each input has its own BDD example in tests/features/messages_corpus. Good graph
comparison permits only URI normalization and bijective IDs with all references;
all other fields and ordering remain meaningful. Bad acceptance means native
Document parsing rejects the input. Error count, wording and exact position are
not claimed by this phase; their published expectations are preserved for later
work. Node is not needed by this offline corpus target.
