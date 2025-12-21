use cucumber::{given, DataTable, World};

#[derive(Debug, Default, World)]
pub struct SimpleTableWorld {
    items_count: usize,
}

// Test direct DataTable parameter
#[given("the following items")]
async fn given_items(world: &mut SimpleTableWorld, table: DataTable) {
    println!("Received table with {} rows", table.rows().len());
    world.items_count = table.rows().len();
}

// Test optional DataTable  
#[given("optional items")]
async fn given_optional_items(world: &mut SimpleTableWorld, table: Option<DataTable>) {
    let count = table.as_ref().map(|t| t.rows().len()).unwrap_or(0);
    println!("Received optional table with {} rows", count);
    world.items_count = count;
}

#[tokio::main]
async fn main() {
    println!("Running direct DataTable example...");
    SimpleTableWorld::run("tests/features/table_direct_simple.feature").await;
    println!("Direct DataTable example completed!");
}