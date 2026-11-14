//! One complete pending frame per window. Callers provide synchronization.

use std::collections::HashMap;

pub struct QuadrosPendentes<T> {
    quadros: HashMap<u64, T>,
}

impl<T> Default for QuadrosPendentes<T> {
    fn default() -> Self {
        Self {
            quadros: HashMap::new(),
        }
    }
}

impl<T> QuadrosPendentes<T> {
    /// Replaces an older frame. True only when a consumer notification is needed.
    pub fn publique(&mut self, id: u64, quadro: T) -> bool {
        let notifique = self.quadros.is_empty();
        self.quadros.insert(id, quadro);
        notifique
    }

    pub fn retire(&mut self) -> Vec<T> {
        self.quadros.drain().map(|(_, q)| q).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flood_keeps_only_the_newest_complete_frame_for_each_window() {
        let mut q = QuadrosPendentes::default();
        for n in 0..100_000 {
            assert_eq!(q.publique(1, (1, n)), n == 0);
            assert!(!q.publique(2, (2, n)));
        }
        let mut frames = q.retire();
        frames.sort();
        assert_eq!(frames, [(1, 99_999), (2, 99_999)]);
        assert!(q.retire().is_empty());
        assert!(q.publique(1, (1, 100_000)));
    }
}
