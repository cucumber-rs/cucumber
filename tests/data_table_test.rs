use std::collections::HashMap;

use cucumber::{given, then, when, DataTable, World, gherkin::Step};

#[derive(Debug, Default, World)]
pub struct TableWorld {
    items: HashMap<String, Item>,
    total: f64,
}

#[derive(Debug, Default)]
struct Item {
    name: String,
    price: f64,
    quantity: u32,
}

// Example 1: Using DataTable with step parameter (current approach)
#[given("the following items in inventory")]
async fn given_items(world: &mut TableWorld, step: &Step) {
    if let Some(table) = step.table.as_ref() {
        let data_table = DataTable::from(table);
        
        for item in data_table.hashes() {
            let name = item.get("name").unwrap().clone();
            let price = item.get("price").unwrap().parse().unwrap();
            let quantity = item.get("quantity").unwrap().parse().unwrap();
            
            world.items.insert(
                name.clone(),
                Item { name, price, quantity }
            );
        }
    }
}

// Example 2: Using the helper function
#[when("I calculate the total value")]
async fn calculate_total(world: &mut TableWorld, step: &Step) {
    use cucumber::step::table::extract_table;
    
    if let Some(table) = extract_table(step) {
        // Process with rich DataTable API
        for row in table.hashes() {
            if let Some(item_name) = row.get("item") {
                if let Some(item) = world.items.get(item_name) {
                    world.total += item.price * item.quantity as f64;
                }
            }
        }
    } else {
        // Calculate all items if no table
        world.total = world.items.values()
            .map(|item| item.price * item.quantity as f64)
            .sum();
    }
}

// Example 3: Using rows_hash for configuration
#[given("the following configuration")]
async fn given_config(world: &mut TableWorld, step: &Step) {
    if let Some(table) = step.table.as_ref() {
        let data_table = DataTable::from(table);
        
        if let Some(config) = data_table.rows_hash() {
            // Process configuration key-value pairs
            if let Some(tax_rate) = config.get("tax_rate") {
                let rate: f64 = tax_rate.parse().unwrap();
                world.total *= 1.0 + rate;
            }
        }
    }
}

// Example 4: Using transpose
#[when("I process transposed data")]
async fn process_transposed(world: &mut TableWorld, step: &Step) {
    if let Some(table) = step.table.as_ref() {
        let data_table = DataTable::from(table);
        let transposed = data_table.transpose();
        
        // First row now contains what were column headers
        for row in transposed.rows() {
            // Process transposed data
            println!("Processing row: {:?}", row);
        }
    }
}

// Example 5: Using columns to select subset
#[then("the filtered items should match")]
async fn check_filtered(world: &mut TableWorld, step: &Step) {
    if let Some(table) = step.table.as_ref() {
        let data_table = DataTable::from(table);
        
        // Select only name and quantity columns
        let subset = data_table.columns(&["name", "quantity"]);
        
        for item in subset.hashes() {
            let name = item.get("name").unwrap();
            let expected_qty: u32 = item.get("quantity").unwrap().parse().unwrap();
            
            assert_eq!(
                world.items.get(name).map(|i| i.quantity),
                Some(expected_qty)
            );
        }
    }
}

#[tokio::test]
async fn test_data_table_api() {
    // Create test table
    let table = DataTable::from(vec![
        vec!["name", "price", "quantity"],
        vec!["apple", "1.50", "10"],
        vec!["banana", "0.75", "20"],
    ]);
    
    // Test raw
    assert_eq!(table.raw().len(), 3);
    
    // Test rows
    assert_eq!(table.rows().len(), 2);
    
    // Test hashes
    let hashes = table.hashes();
    assert_eq!(hashes[0].get("name"), Some(&"apple".to_string()));
    assert_eq!(hashes[1].get("price"), Some(&"0.75".to_string()));
    
    // Test transpose
    let transposed = table.transpose();
    assert_eq!(transposed.raw()[0], vec!["name", "apple", "banana"]);
    
    // Test columns
    let subset = table.columns(&["name", "quantity"]);
    assert_eq!(subset.width(), 2);
}

#[tokio::test]
async fn test_rows_hash() {
    let table = DataTable::from(vec![
        vec!["setting", "value"],
        vec!["timeout", "30"],
        vec!["retries", "3"],
    ]);
    
    let hash = table.rows_hash().unwrap();
    assert_eq!(hash.get("timeout"), Some(&"30".to_string()));
    assert_eq!(hash.get("retries"), Some(&"3".to_string()));
}