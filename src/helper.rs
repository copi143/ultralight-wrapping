use serde::{Deserialize, Serialize, ser::SerializeMap as _};
use std::fmt::Debug;

pub struct VecMap<K: PartialEq, V> {
    items: Vec<(K, V)>,
}

impl<K: PartialEq, V> VecMap<K, V> {
    pub const fn new() -> Self {
        VecMap { items: Vec::new() }
    }

    pub fn insert(&mut self, key: K, value: V) {
        for (k, v) in &mut self.items {
            if *k == key {
                *v = value;
                return;
            }
        }
        self.items.push((key, value));
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        for (k, v) in &self.items {
            if k == key {
                return Some(v);
            }
        }
        None
    }

    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.items.iter().map(|(k, v)| (k, v))
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }
}

impl<K: PartialEq + Debug, V: Debug> Debug for VecMap<K, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut dm = f.debug_map();
        for (k, v) in &self.items {
            dm.entry(k, v);
        }
        dm.finish()
    }
}

impl<K: PartialEq, V> Default for VecMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: PartialEq + Clone, V: Clone> Clone for VecMap<K, V> {
    fn clone(&self) -> Self {
        VecMap {
            items: self.items.clone(),
        }
    }
}

impl<K: PartialEq, V: PartialEq> PartialEq for VecMap<K, V> {
    fn eq(&self, other: &Self) -> bool {
        if self.items.len() != other.items.len() {
            return false;
        }
        for (k, v) in &self.items {
            match other.get(k) {
                Some(ov) if ov == v => continue,
                _ => return false,
            }
        }
        true
    }
}

impl<K: PartialEq + Serialize, V: Serialize> Serialize for VecMap<K, V> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.items.len()))?;
        for (k, v) in &self.items {
            map.serialize_entry(k, v)?;
        }
        map.end()
    }
}

impl<'de, K: PartialEq + Deserialize<'de>, V: Deserialize<'de>> Deserialize<'de> for VecMap<K, V> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct VecMapVisitor<K, V> {
            marker: std::marker::PhantomData<(K, V)>,
        }

        impl<'de, K, V> serde::de::Visitor<'de> for VecMapVisitor<K, V>
        where
            K: PartialEq + serde::Deserialize<'de>,
            V: serde::Deserialize<'de>,
        {
            type Value = VecMap<K, V>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a map to deserialize into VecMap")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                let mut items = Vec::with_capacity(map.size_hint().unwrap_or(0));
                while let Some((k, v)) = map.next_entry()? {
                    items.push((k, v));
                }
                Ok(VecMap { items })
            }
        }

        deserializer.deserialize_map(VecMapVisitor {
            marker: std::marker::PhantomData,
        })
    }
}
