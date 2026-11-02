//! Bounded text caches with allocation-free borrowed lookups.
//!
//! Grouping by font/style keeps the owned `String` at the inner map, so a cache
//! hit can use `&str`. Values may own SDL resources: callers explicitly dispose
//! of drained/rejected values on the renderer thread.

use std::{borrow::Cow, collections::HashMap, hash::Hash};

pub struct TextCache<K, V> {
    styles: HashMap<K, HashMap<String, V>>,
    entries: usize,
    limit: usize,
}

impl<K: Copy + Eq + Hash, V> TextCache<K, V> {
    pub fn new(limit: usize) -> Self {
        Self {
            styles: HashMap::new(),
            entries: 0,
            limit,
        }
    }

    pub fn get(&self, style: K, text: &str) -> Option<&V> {
        self.styles.get(&style)?.get(text)
    }

    pub fn is_full(&self) -> bool {
        self.entries >= self.limit
    }

    /// Reject a full cache or duplicate without dropping the supplied value.
    /// This keeps both the bound and external resource ownership explicit.
    pub fn try_insert(&mut self, style: K, text: &str, value: V) -> Result<(), V> {
        if self.is_full() || self.get(style, text).is_some() {
            return Err(value);
        }
        self.styles
            .entry(style)
            .or_default()
            .insert(text.to_owned(), value);
        self.entries += 1;
        Ok(())
    }

    pub fn drain(&mut self) -> impl Iterator<Item = V> + '_ {
        self.entries = 0;
        self.styles
            .drain()
            .flat_map(|(_, values)| values.into_values())
    }
}

/// Preserve the existing SDL_ttf input policy without allocating for normal
/// labels. Interior NULs are removed, rather than truncating at the first NUL.
pub fn clean_text(text: &str) -> Cow<'_, str> {
    if text.as_bytes().contains(&0) {
        Cow::Owned(text.replace('\0', ""))
    } else {
        Cow::Borrowed(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn borrowed_utf8_lookup_keeps_font_and_theme_identity() {
        let mut cache = TextCache::new(4);
        assert_eq!(cache.try_insert((48, 550, false), "Café 🌙", 110), Ok(()));
        assert_eq!(cache.try_insert((48, 650, false), "Café 🌙", 118), Ok(()));
        assert_eq!(cache.try_insert((48, 550, true), "Café 🌙", 112), Ok(()));
        let separately_owned = String::from("Café 🌙");
        assert_eq!(cache.get((48, 550, false), &separately_owned), Some(&110));
        assert_eq!(cache.get((48, 650, false), &separately_owned), Some(&118));
        assert_eq!(cache.get((48, 550, true), &separately_owned), Some(&112));
        assert_eq!(cache.get((49, 550, false), &separately_owned), None);
        assert_eq!(cache.get((48, 550, false), "Cafe 🌙"), None);
    }

    #[test]
    fn bound_rejects_new_values_without_losing_cached_resources() {
        let mut cache = TextCache::new(2);
        assert_eq!(cache.try_insert(1, "first", 10), Ok(()));
        assert_eq!(cache.try_insert(2, "second", 20), Ok(()));
        assert!(cache.is_full());
        assert_eq!(cache.try_insert(1, "third", 30), Err(30));
        assert_eq!(cache.get(1, "first"), Some(&10));
        assert_eq!(cache.get(2, "second"), Some(&20));
        assert_eq!(cache.get(1, "third"), None);
        let mut released: Vec<_> = cache.drain().collect();
        released.sort_unstable();
        assert_eq!(released, vec![10, 20]);
        assert!(!cache.is_full());
        assert_eq!(cache.get(1, "first"), None);
        assert_eq!(cache.try_insert(1, "third", 30), Ok(()));
    }

    #[test]
    fn duplicate_does_not_replace_or_implicitly_drop_a_resource() {
        let mut cache = TextCache::new(2);
        assert_eq!(cache.try_insert(1, "same", 10), Ok(()));
        assert_eq!(cache.try_insert(1, "same", 20), Err(20));
        assert_eq!(cache.get(1, "same"), Some(&10));
        assert!(!cache.is_full());
        assert_eq!(cache.drain().collect::<Vec<_>>(), vec![10]);
    }

    #[test]
    fn zero_capacity_cannot_grow() {
        let mut cache = TextCache::new(0);
        assert!(cache.is_full());
        assert_eq!(cache.try_insert(1, "text", 10), Err(10));
        assert_eq!(cache.get(1, "text"), None);
        assert_eq!(cache.drain().count(), 0);
    }

    #[test]
    fn nul_cleaning_borrows_normal_text_and_preserves_all_other_codepoints() {
        let label = "Café e\u{301} 🌙";
        assert!(matches!(clean_text(label), Cow::Borrowed(_)));
        assert_eq!(clean_text("\0Café\0 e\u{301} 🌙\0"), label);
        assert_eq!(clean_text("\0\0"), "");
        assert!(matches!(clean_text(""), Cow::Borrowed("")));
    }
}
