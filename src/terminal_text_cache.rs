// Pötyi - Lightweight text editor
// Copyright (C) 2026 Attila Banko
// SPDX-License-Identifier: GPL-3.0-or-later

use sdl3::pixels::Color;
use sdl3::render::Texture;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

// Includes estimated RGBA texture storage and keys, rather than counting rows
// alone: a Retina row costs substantially more than a small low-DPI row.
pub(crate) const MAX_BYTES: usize = 8 * 1024 * 1024;
const MAX_ENTRIES: usize = 512;

struct Entry<'a> {
    text: String,
    color: Color,
    texture: Texture<'a>,
    bytes: usize,
    used: u64,
}

#[derive(Default)]
pub(crate) struct TextCache<'a> {
    entries: HashMap<u64, Entry<'a>>,
    bytes: usize,
    clock: u64,
    #[cfg(test)]
    pub misses: usize,
}

impl<'a> TextCache<'a> {
    pub fn clear(&mut self) {
        // Returning to the editor releases GPU textures and map allocation.
        *self = Self::default();
    }

    pub fn key(text: &str, color: Color) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        text.hash(&mut hasher);
        (color.r, color.g, color.b, color.a).hash(&mut hasher);
        hasher.finish()
    }

    pub fn get(&mut self, key: u64, text: &str, color: Color) -> Option<&Texture<'a>> {
        let entry = self.entries.get_mut(&key)?;
        // Hash collisions may cause eviction, but never display the wrong text.
        if entry.text != text || entry.color != color {
            return None;
        }
        self.clock = self.clock.wrapping_add(1);
        entry.used = self.clock;
        Some(&entry.texture)
    }

    pub fn insert(&mut self, key: u64, text: &str, color: Color, texture: Texture<'a>) {
        let query = texture.query();
        let bytes = (query.width as usize)
            .saturating_mul(query.height as usize)
            .saturating_mul(4)
            .saturating_add(text.len())
            .saturating_add(std::mem::size_of::<Entry>());
        if bytes > MAX_BYTES {
            return;
        }
        if let Some(old) = self.entries.remove(&key) {
            self.bytes -= old.bytes;
        }
        while self.bytes + bytes > MAX_BYTES || self.entries.len() >= MAX_ENTRIES {
            let Some((&oldest, _)) = self.entries.iter().min_by_key(|(_, entry)| entry.used) else {
                break;
            };
            let old = self.entries.remove(&oldest).unwrap();
            self.bytes -= old.bytes;
        }
        self.clock = self.clock.wrapping_add(1);
        self.bytes += bytes;
        self.entries.insert(
            key,
            Entry {
                text: text.into(),
                color,
                texture,
                bytes,
                used: self.clock,
            },
        );
    }

    #[cfg(test)]
    pub fn bytes(&self) -> usize {
        self.bytes
    }
}
