use std::collections::BTreeMap;

pub struct Item {
    pub id: u32,
    pub name: String,
    pub max_stack: u64,
    pub description: String,
}

pub struct Recipe {
    pub name: String,
    pub material: BTreeMap<String, u64>,
    pub products: BTreeMap<String, u64>,
    pub timecost: u64,
}

// static ITEMS: OnceLock<BTreeMap<String, Item>> = OnceLock::new();
