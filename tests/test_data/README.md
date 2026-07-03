# Test Data Files

IDL schema files from the Go [idlparser](https://github.com/Yisaer/idlparser) test suite.
These are reference files only — all Rust tests use inline IDL strings and construct
binary data in code. No test in this crate reads from disk.

| File | Used in Go test | What it defines |
|---|---|---|
| `test.idl` | `TestConverterDecode` | Simple struct with two octets |
| `test_sts.idl` | `TestConverterStsDecode` | Struct with short, octet fields |
| `test_complex.idl` | `TestConverterDecodeComplexStruct` | Nested module with short+long+float |
| `s1_test.idl` | `TestS1` | WiFi AP list with Chinese strings |
| `test_string_padding.idl` | `TestStructPaddingIntegration` | Structs with dynamic strings + padding |
| `test_array_padding.idl` | `TestStructPaddingIntegration` | Structs with dynamic sequences + padding |
