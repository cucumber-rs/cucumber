# ADR-0019: DataTable API and Direct Parameter Support

## Status

Accepted

## Context

Cucumber implementations in other languages (particularly cucumber-js and cucumber-ruby) provide rich APIs for working with data tables in step definitions. These implementations allow:

1. **Direct parameter injection** - Tables are passed directly as parameters to step functions
2. **Rich transformation APIs** - Methods like `hashes()`, `rows()`, `transpose()`, etc.
3. **Type safety** - Tables are first-class typed objects, not raw arrays

The Rust implementation previously required:
- Manual extraction from `step.table` 
- Working with raw `Vec<Vec<String>>` structures
- Boilerplate code for common operations

This made the Rust implementation less ergonomic and more error-prone compared to other Cucumber implementations.

## Decision

We implemented a comprehensive DataTable API with direct parameter support following these principles:

### 1. DataTable Type
Created a first-class `DataTable` type with methods matching cucumber-js:
- `raw()` - Returns the complete 2D array
- `rows()` - Returns rows without the header
- `hashes()` - Converts to an array of HashMaps using headers as keys
- `rows_hash()` - Converts a 2-column table to a HashMap
- `transpose()` - Transposes rows and columns
- `columns()` - Selects specific columns by name

### 2. Direct Parameter Support
Extended the proc macro system to support DataTable as a direct parameter:
```rust
// Required table
#[given("items")]
async fn items(world: &mut World, table: DataTable) { }

// Optional table
#[when("data")]
async fn data(world: &mut World, table: Option<DataTable>) { }
```

### 3. Macro System Integration
Modified the step attribute macros to:
- Detect DataTable parameters in function signatures
- Generate extraction code from `__cucumber_ctx.step.table`
- Handle both required and optional DataTable parameters
- Maintain backward compatibility with existing `step: &Step` approach

### 4. Implementation Architecture
```
┌─────────────────┐
│  Step Function  │
│  with DataTable │
└────────┬────────┘
         │
    ┌────▼─────┐
    │  Macro   │ Detects DataTable params
    │  System  │ Generates extraction code
    └────┬─────┘
         │
    ┌────▼──────┐
    │ Generated │ Extracts table from step
    │   Code    │ Creates DataTable instance
    └────┬──────┘
         │
    ┌────▼────────┐
    │  DataTable  │ Rich API for table ops
    │    Type     │ (hashes, rows, etc.)
    └─────────────┘
```

## Consequences

### Positive

1. **Improved Ergonomics** - Clean, intuitive API matching other Cucumber implementations
2. **Type Safety** - DataTable is a proper type with compile-time checking
3. **Reduced Boilerplate** - No manual extraction or parsing needed
4. **Feature Parity** - Brings Rust implementation closer to cucumber-js/ruby
5. **Backward Compatible** - Existing code using `step: &Step` continues to work
6. **Rich Operations** - Common table operations are built-in and optimized

### Negative

1. **Increased Complexity** - Proc macro system is more complex
2. **Compilation Time** - Additional macro processing may slightly increase compile times
3. **Breaking Change Potential** - Future changes to DataTable API could impact many users

### Neutral

1. **Migration Path** - Users can gradually migrate from `step: &Step` to direct parameters
2. **Documentation** - Requires comprehensive documentation of both approaches
3. **Testing** - Needs extensive test coverage for all parameter combinations

## Implementation Details

### Detection Logic
The macro system detects DataTable parameters by:
1. Examining function parameter types
2. Checking for `DataTable` or `Option<DataTable>` types
3. Generating appropriate extraction code based on optionality

### Code Generation
For each DataTable parameter, the macro generates:
```rust
// Required DataTable
let table = __cucumber_ctx.step.table.as_ref()
    .map(::cucumber::DataTable::from)
    .expect("Step requires DataTable but none provided");

// Optional DataTable  
let table = __cucumber_ctx.step.table.as_ref()
    .map(::cucumber::DataTable::from);
```

### Error Handling
- Required tables panic with clear message if missing
- Optional tables gracefully handle absence
- Type mismatches caught at compile time

## Examples

### Before (Manual Extraction)
```rust
#[given("items")]
async fn items(world: &mut World, step: &Step) {
    if let Some(table) = step.table.as_ref() {
        for row in table.rows.iter().skip(1) {
            let name = &row[0];
            let value = &row[1];
            // Process...
        }
    }
}
```

### After (Direct Parameter)
```rust
#[given("items")]
async fn items(world: &mut World, table: DataTable) {
    for item in table.hashes() {
        let name = item.get("name").unwrap();
        let value = item.get("value").unwrap();
        // Process...
    }
}
```

## References

- [cucumber-js DataTable API](https://github.com/cucumber/cucumber-js/blob/main/src/models/data_table.ts)
- [cucumber-ruby Table API](https://github.com/cucumber/cucumber-ruby/blob/main/lib/cucumber/multiline_argument/data_table.rb)
- [Gherkin data tables specification](https://cucumber.io/docs/gherkin/reference/#data-tables)
- Related PRs: [Initial DataTable implementation], [Direct parameter support]

## Decision Makers

- Architecture Team
- Cucumber-rs maintainers
- Community feedback via GitHub issues

## Date

2024-12-21