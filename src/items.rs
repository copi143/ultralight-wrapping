use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs::File, path::Path};

use crate::{helper::VecMap, read_c_string};

#[repr(u8)]
#[derive(Serialize, Deserialize)]
pub enum ItemType {
    Placeholder = 0, // 占位符
    Solid = 1,       // 固体
    Fluid = 2,       // 液体
    Gas = 3,         // 气体 (给 Mek 用)
    Energy = 4,      // 能量 (各种电力和魔力)
}

#[derive(Serialize, Deserialize)]
pub struct Item {
    /// 物品ID
    #[serde(skip_deserializing)]
    #[serde(default)]
    pub id: u32,

    /// 物品类型
    #[serde(rename = "type")]
    pub ty: ItemType,

    /// 物品名称 (按照 MC 中的注册名)
    pub name: String,

    /// 物品国际化名称
    pub i18n: String,

    /// 物品本地化名称
    pub l10n: String,

    /// 最大堆叠数量
    pub max_stack: u64,

    /// 物品描述
    pub description: String,
}

#[derive(Serialize, Deserialize)]
pub struct Recipe {
    #[serde(skip_deserializing)]
    pub id: u32,
    // 配方名称
    pub name: Option<String>,
    // 输入
    pub material: VecMap<u32, u64>,
    // 输出
    pub products: VecMap<u32, u64>,
    // 制作时间 (ticks)
    pub timecost: u64,
}

pub struct ItemManager {
    pub names: BTreeMap<String, u32>,
    pub i18ns: BTreeMap<String, u32>,
    pub l10ns: BTreeMap<String, u32>,
    pub items: Vec<Item>,
}

impl ItemManager {
    pub const fn new() -> Self {
        Self {
            items: Vec::new(),
            i18ns: BTreeMap::new(),
            l10ns: BTreeMap::new(),
            names: BTreeMap::new(),
        }
    }

    pub fn insert(&mut self, mut item: Item) -> Result<(), String> {
        if self.names.contains_key(&item.name) {
            return Err(format!("Item with name '{}' already exists!", item.name));
        }
        item.id = self.items.len() as u32;
        self.names.insert(item.name.clone(), item.id);
        self.i18ns.insert(item.i18n.clone(), item.id);
        self.l10ns.insert(item.l10n.clone(), item.id);
        self.items.push(item);
        Ok(())
    }

    pub fn get_by_id(&self, id: u32) -> Option<&Item> {
        self.items.get(id as usize)
    }

    pub fn get_by_name(&self, name: &str) -> Option<&Item> {
        if let Some(&id) = self.names.get(name) {
            return self.get_by_id(id);
        }
        None
    }

    pub fn get_by_i18n(&self, i18n: &str) -> Option<&Item> {
        if let Some(&id) = self.i18ns.get(i18n) {
            return self.get_by_id(id);
        }
        None
    }

    pub fn get_by_l10n(&self, l10n: &str) -> Option<&Item> {
        if let Some(&id) = self.l10ns.get(l10n) {
            return self.get_by_id(id);
        }
        None
    }

    pub fn id_by_name(&self, name: &str) -> Option<u32> {
        self.names.get(name).copied()
    }

    pub fn id_by_i18n(&self, i18n: &str) -> Option<u32> {
        self.i18ns.get(i18n).copied()
    }

    pub fn id_by_l10n(&self, l10n: &str) -> Option<u32> {
        self.l10ns.get(l10n).copied()
    }
}

static ITEMS: Mutex<ItemManager> = Mutex::new(ItemManager::new());
static RECIPES: Mutex<BTreeMap<String, Recipe>> = Mutex::new(BTreeMap::new());

pub fn add_item(item: Item) {
    let mut items = ITEMS.lock();
    items
        .insert(item)
        .unwrap_or_else(|e| eprintln!("Failed to add item: {}", e));
}

#[unsafe(no_mangle)]
extern "C" fn ultralightui_add_item(
    ty: ItemType,
    name: *const u8,
    i18n: *const u8,
    l10n: *const u8,
    max_stack: u64,
    description: *const u8,
) {
    let name = read_c_string(name).to_string();
    let i18n = read_c_string(i18n).to_string();
    let l10n = read_c_string(l10n).to_string();
    let description = read_c_string(description).to_string();

    add_item(Item {
        id: 0,
        ty,
        name,
        i18n,
        l10n,
        max_stack,
        description,
    });
}

#[unsafe(no_mangle)]
extern "C" fn ultralightui_save_items(path: *const u8) {
    let path = Path::new(crate::read_c_string(path));
    let items = ITEMS.lock();
    let file = File::create(path).unwrap();
    // serde_json::to_writer_pretty(file, &*items).unwrap();
}

#[unsafe(no_mangle)]
extern "C" fn ultralightui_save_recipes(path: *const u8) {
    let path = Path::new(crate::read_c_string(path));
    let recipes = RECIPES.lock();
    let file = File::create(path).unwrap();
    serde_json::to_writer_pretty(file, &*recipes).unwrap();
}
