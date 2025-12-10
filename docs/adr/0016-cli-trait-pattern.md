# ADR-0016: CLI Trait Pattern for Component Configuration

## Status
Accepted (Original Design)

## Context
Each component (Parser, Runner, Writer) needs different CLI options:
- Parser: Feature file paths, filters
- Runner: Concurrency, retry settings, fail-fast
- Writer: Output format, verbosity, colors

Hardcoding CLI options in the main Cucumber struct would:
- Violate separation of concerns
- Make custom components unable to add their options
- Create option conflicts between components
- Reduce flexibility

## Decision
Each component trait has an associated `Cli` type:
- Components define their own CLI structure
- CLI types compose when components compose
- `cli::Empty` for components without CLI options
- Derive from `clap` for automatic parsing

## Consequences

### Positive
- Components fully control their configuration
- Clean separation of concerns
- Extensible without modifying core
- Type-safe CLI composition
- Automatic help generation

### Negative
- More complex type signatures
- CLI conflicts need manual resolution
- Learning curve for trait associated types
- Potential for confusing option combinations

## Implementation
```rust
pub trait Parser<I> {
    type Cli: clap::Args;  // Associated CLI type
    type Output: Stream<Item = Result<Feature>>;
    
    fn parse(self, input: I, cli: Self::Cli) -> Self::Output;
}

// Basic parser with CLI options
impl Parser<I> for Basic {
    type Cli = BasicCli;
    // ...
}

#[derive(clap::Args)]
struct BasicCli {
    /// Regex to filter scenarios
    #[arg(long)]
    filter: Option<Regex>,
    
    /// Tags to include
    #[arg(long)]
    tags: Option<String>,
}

// Custom parser without CLI
impl Parser<I> for CustomParser {
    type Cli = cli::Empty;  // No CLI options
    // ...
}
```

## Composition
```rust
// Combined CLI from all components
#[derive(clap::Parser)]
struct CucumberCli {
    #[command(flatten)]
    parser: <P as Parser>::Cli,
    
    #[command(flatten)]
    runner: <R as Runner>::Cli,
    
    #[command(flatten)]
    writer: <W as Writer>::Cli,
}
```

## Usage
```bash
# Parser options
cargo test -- --filter "user.*" --tags "@smoke"

# Runner options  
cargo test -- --fail-fast --retries 3 --concurrency 10

# Writer options
cargo test -- --format json --verbose --no-color

# Combined
cargo test -- --tags "@critical" --fail-fast --format junit
```

## Design Principles
1. **Component Autonomy**: Each component owns its configuration
2. **Type Safety**: CLI structure verified at compile time
3. **Composability**: CLI types compose like components
4. **Zero Cost**: No runtime overhead for CLI parsing

## Conflict Resolution
When options conflict, later components take precedence:
```rust
// Writer's --verbose overrides Runner's --quiet
cucumber
    .with_runner(runner.quiet())
    .with_writer(writer.verbose())
```

## References
- Clap derive API
- Type-driven design
- Associated types in Rust