Data tables
===========

[Data tables][table] represent a handy way for passing a list of values to a [step] definition (and so, to a [step] matching function). This is a vital ability for writing [table driven tests][tdt].

```gherkin
Feature: Animal feature

  Scenario: If we feed a hungry animal it will no longer be hungry
    Given a hungry animal
      | animal |
      | cat    |
      | dog    |
      | 🦀     |
    When I feed the animal multiple times
      | animal | times |
      | cat    | 2     |
      | dog    | 3     |
      | 🦀     | 4     |
    Then the animal is not hungry
```

Data, declared in the [table], may be accessed via [`Step`] argument. 

## Basic Table Access

The traditional approach accesses the raw Gherkin table:
```rust
# extern crate cucumber;
# extern crate tokio;
#
# use std::collections::HashMap;
#
use cucumber::{World, gherkin::Step, given, then, when};

#[given(regex = r"^a (hungry|satiated) animal$")]
async fn hungry_animal(world: &mut AnimalWorld, step: &Step, state: String) {
    let state = match state.as_str() {
        "hungry" => true,
        "satiated" => false,
        _ => unreachable!(),
    };

    if let Some(table) = step.table.as_ref() {
        for row in table.rows.iter().skip(1) { // NOTE: skip header
            let animal = &row[0];

            world
                .animals
                .entry(animal.clone())
                .or_insert(Animal::default())
                .hungry = state;
        }
    }
}
```

## Using the DataTable API

### Direct DataTable Parameters (Recommended)

The most canonical approach is to receive `DataTable` directly as a parameter:

```rust
use cucumber::{DataTable, given};

#[given(regex = r"^a (hungry|satiated) animal$")]
async fn hungry_animal(world: &mut AnimalWorld, state: String, table: DataTable) {
    // DataTable is provided directly - no manual extraction needed
    for animal_data in table.hashes() {
        let animal_name = animal_data.get("animal").unwrap();
        
        world.animals
            .entry(animal_name.clone())
            .or_insert(Animal::default())
            .hungry = state == "hungry";
    }
}

// Optional tables are also supported
#[when("I perform operations")]
async fn operations(world: &mut World, table: Option<DataTable>) {
    if let Some(table) = table {
        // Process table data
        for row in table.hashes() {
            // ...
        }
    } else {
        // Handle case when no table is provided
    }
}
```

### Alternative: Manual Extraction via Step

For backward compatibility, you can still access tables through the `Step` parameter:

```rust
use cucumber::{DataTable, gherkin::Step, given};

#[given(regex = r"^a (hungry|satiated) animal$")]
async fn hungry_animal(world: &mut AnimalWorld, step: &Step, state: String) {
    if let Some(table) = step.table.as_ref() {
        let data_table = DataTable::from(table);
        
        // Use hashes() for convenient column access
        for animal_data in data_table.hashes() {
            let animal_name = animal_data.get("animal").unwrap();
            
            world.animals
                .entry(animal_name.clone())
                .or_insert(Animal::default())
                .hungry = state == "hungry";
        }
    }
}

```

## DataTable Methods

The `DataTable` type provides several useful methods:

### `hashes()` - Array of HashMaps
Converts rows to hashmaps using the first row as keys:
```rust
let data_table = DataTable::from(table);
for item in data_table.hashes() {
    let name = item.get("name").unwrap();
    let value = item.get("value").unwrap();
}
```

### `rows()` - Rows without header
Returns all rows except the header:
```rust
let data_table = DataTable::from(table);
for row in data_table.rows() {
    let first_col = &row[0];
    let second_col = &row[1];
}
```

### `rows_hash()` - Two-column table as HashMap
Converts a two-column table to a key-value hashmap:
```rust
if let Some(config) = data_table.rows_hash() {
    let timeout = config.get("timeout").unwrap();
    let retries = config.get("retries").unwrap();
}
```

### `transpose()` - Swap rows and columns
```rust
let transposed = data_table.transpose();
// Rows are now columns and vice versa
```

### `columns()` - Select specific columns
```rust
let subset = data_table.columns(&["name", "quantity"]);
// Returns new DataTable with only specified columns
```

## Complete Example

Here's the complete animal feeding example using direct DataTable parameters:

```rust
use cucumber::{DataTable, given, when, then, World};
use std::collections::HashMap;

#[given(regex = r"^a (hungry|satiated) animal$")]
async fn hungry_animal(world: &mut AnimalWorld, state: String, table: DataTable) {
    let is_hungry = state == "hungry";
    
    // Direct DataTable access - clean and canonical
    for animal_data in table.hashes() {
        let animal_name = animal_data.get("animal").unwrap();
        
        world.animals
            .entry(animal_name.clone())
            .or_insert(Animal::default())
            .hungry = is_hungry;
    }
}

#[when("I feed the animal multiple times")]
async fn feed_animal(world: &mut AnimalWorld, table: DataTable) {
    // Using hashes() for cleaner access
    for feeding in table.hashes() {
        let animal = feeding.get("animal").unwrap();
        let times: usize = feeding.get("times").unwrap().parse().unwrap();
        
        for _ in 0..times {
            world.animals.get_mut(animal).map(Animal::feed);
        }
    }
}

#[then("the animal is not hungry")]
async fn animal_is_fed(world: &mut AnimalWorld) {
    for animal in world.animals.values() {
        assert!(!animal.hungry);
    }
}

#[derive(Debug, Default)]
struct Animal {
    pub hungry: bool,
}

impl Animal {
    fn feed(&mut self) {
        self.hungry = false;
    }
}

#[derive(Debug, Default, World)]
pub struct AnimalWorld {
    animals: HashMap<String, Animal>,
}
#
# #[tokio::main]
# async fn main() {
#     AnimalWorld::run("tests/features/book/writing/data_tables.feature").await;
# }
```

> __NOTE__: The whole table data is processed during a single [step] run.

![record](../rec/writing_data_tables.gif)




## Escaping

- To use a newline character in a table cell, write it as `\n`. 
- To use a `|` as a part in a table cell, escape it as `\|`. 
- And finally, to use a `\`, escape it as `\\`.




[`Step`]: https://docs.rs/gherkin/*/gherkin/struct.Step.html
[step]: https://cucumber.io/docs/gherkin/reference#steps
[table]: https://cucumber.io/docs/gherkin/reference#data-tables
[tdt]: https://dave.cheney.net/2019/05/07/prefer-table-driven-tests
