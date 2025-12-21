use cucumber::{given, DataTable, World};

#[derive(Debug, Default, World)]
pub struct SimpleTableWorld {
    items_count: usize,
}

// Test direct DataTable parameter
#[given("the following items")]
async fn given_items(world: &mut SimpleTableWorld, table: DataTable) {
    world.items_count = table.rows().len();
}

// Test optional DataTable  
#[given("optional items")]
async fn given_optional_items(world: &mut SimpleTableWorld, table: Option<DataTable>) {
    world.items_count = table.map(|t| t.rows().len()).unwrap_or(0);
}

#[tokio::main]
async fn main() {
    println!("Running direct DataTable tests...");
    SimpleTableWorld::run("tests/features/table_direct_simple.feature").await;
    println!("Direct DataTable tests completed!");
}