use std::collections::HashMap;

use cucumber::{given, then, when, DataTable, World};

#[derive(Debug, Default, World)]
pub struct DirectTableWorld {
    products: HashMap<String, Product>,
    cart: Vec<CartItem>,
    total: f64,
}

#[derive(Debug)]
struct Product {
    name: String,
    price: f64,
    stock: u32,
}

#[derive(Debug)]
struct CartItem {
    product_name: String,
    quantity: u32,
}

// Test 1: Direct DataTable parameter (required table)
#[given("the following products exist")]
async fn given_products(world: &mut DirectTableWorld, table: DataTable) {
    for product_data in table.hashes() {
        let name = product_data.get("name").unwrap_or(&String::new()).clone();
        let price = product_data.get("price").unwrap_or(&String::from("0")).parse().unwrap_or(0.0);
        let stock = product_data.get("stock").unwrap_or(&String::from("0")).parse().unwrap_or(0);
        
        world.products.insert(
            name.clone(),
            Product { name, price, stock }
        );
    }
}

// Test 2: Optional DataTable parameter
#[when("I add items to cart")]
async fn add_to_cart(world: &mut DirectTableWorld, table: Option<DataTable>) {
    if let Some(table) = table {
        for item in table.hashes() {
            let product_name = item.get("product").unwrap().clone();
            let quantity = item.get("quantity").unwrap().parse().unwrap();
            
            world.cart.push(CartItem { product_name, quantity });
        }
    } else {
        // Add all products with quantity 1 if no table provided
        for name in world.products.keys() {
            world.cart.push(CartItem {
                product_name: name.clone(),
                quantity: 1,
            });
        }
    }
}

// Test 3: DataTable with captured parameters
#[when(regex = r"I apply a (\d+)% discount with exclusions")]
async fn apply_discount(
    world: &mut DirectTableWorld,
    discount_percent: u32,
    table: DataTable,
) {
    let excluded: Vec<String> = table.hashes()
        .iter()
        .filter_map(|row| row.get("excluded_product").cloned())
        .collect();
    
    world.total = world.cart.iter()
        .map(|item| {
            let product = &world.products[&item.product_name];
            let price = product.price * item.quantity as f64;
            
            if excluded.contains(&item.product_name) {
                price
            } else {
                price * (1.0 - discount_percent as f64 / 100.0)
            }
        })
        .sum();
}

// Test 4: Using rows_hash for configuration
#[given("the store configuration")]
async fn store_config(world: &mut DirectTableWorld, table: DataTable) {
    if let Some(config) = table.rows_hash() {
        // Process store configuration
        if let Some(tax_rate) = config.get("tax_rate") {
            let rate: f64 = tax_rate.parse().unwrap();
            world.total *= 1.0 + rate;
        }
        
        if let Some(min_order) = config.get("minimum_order") {
            let min: f64 = min_order.parse().unwrap();
            if world.total < min {
                world.total = 0.0; // Order rejected
            }
        }
    }
}

// Test 5: Using transpose
#[when("I process transposed inventory")]
async fn process_inventory(world: &mut DirectTableWorld, table: DataTable) {
    let transposed = table.transpose();
    
    // First row now contains product names
    // Second row contains quantities
    let rows = transposed.rows();
    if rows.len() >= 2 {
        let names = &rows[0];
        let quantities = &rows[1];
        
        for (name, qty_str) in names.iter().zip(quantities.iter()) {
            if let Ok(qty) = qty_str.parse::<u32>() {
                if let Some(product) = world.products.get_mut(name) {
                    product.stock += qty;
                }
            }
        }
    }
}

// Test 6: Columns selection
#[then("the order summary should contain")]
async fn check_summary(world: &mut DirectTableWorld, table: DataTable) {
    // Select only relevant columns for validation
    let summary_table = table.columns(&["product", "quantity"]);
    
    for expected in summary_table.hashes() {
        let product_name = expected.get("product").unwrap();
        let expected_qty: u32 = expected.get("quantity").unwrap().parse().unwrap();
        
        let actual_qty = world.cart
            .iter()
            .filter(|item| &item.product_name == product_name)
            .map(|item| item.quantity)
            .sum::<u32>();
        
        assert_eq!(actual_qty, expected_qty);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_direct_data_table() {
        // Test that DataTable can be created and used directly
        let table = DataTable::from(vec![
            vec!["product", "price", "stock"],
            vec!["apple", "1.50", "100"],
            vec!["banana", "0.75", "200"],
        ]);
        
        let mut world = DirectTableWorld::default();
        given_products(&mut world, table).await;
        
        assert_eq!(world.products.len(), 2);
        assert!(world.products.contains_key("apple"));
        assert_eq!(world.products["apple"].price, 1.50);
    }
    
    #[tokio::test]
    async fn test_optional_data_table() {
        let mut world = DirectTableWorld::default();
        
        // Test with Some(table)
        let table = DataTable::from(vec![
            vec!["product", "quantity"],
            vec!["apple", "5"],
        ]);
        add_to_cart(&mut world, Some(table)).await;
        assert_eq!(world.cart.len(), 1);
        
        // Test with None
        world.products.insert(
            "orange".to_string(),
            Product { name: "orange".to_string(), price: 2.0, stock: 50 }
        );
        world.cart.clear();
        add_to_cart(&mut world, None).await;
        assert_eq!(world.cart.len(), 1);
    }
    
    #[tokio::test]
    async fn test_rows_hash() {
        let config_table = DataTable::from(vec![
            vec!["setting", "value"],
            vec!["tax_rate", "0.08"],
            vec!["minimum_order", "10.00"],
        ]);
        
        let mut world = DirectTableWorld::default();
        world.total = 15.0;
        store_config(&mut world, config_table).await;
        
        // Tax should be applied (with floating point tolerance)
        assert!((world.total - 16.2).abs() < 0.0001); // 15 * 1.08
    }
}