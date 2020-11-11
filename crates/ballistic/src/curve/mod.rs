mod entry;
pub use entry::{Entry, VacantEntry, OccupiedEntry, EntryApi};

#[derive(Debug, Clone, Default)]
pub struct Curve<K, V> {
    values: Vec<(K, V)>
}

pub type FloatCurve = Curve<f32, f32>;

impl FloatCurve {
    pub fn new() -> Self {
        Self {
            values: Default::default()
        }
    }

    pub fn get_value(&mut self, key: f32) -> f32 {
        let entry = self.get_entry(key);
        entry.value()
    }

    pub fn get_entry(&mut self, key: f32) -> Entry<'_, f32, f32> {
        let index = self.find_entry_index(key);

        match self.values.get(index) {
            // We found an equivalent key, so we found the actual location of an entry with this key
            Some(&(k, _)) if epsilon_eq(key, k) => {
                Entry::Occupied(OccupiedEntry {
                    curve: self,
                    index,
                })
            },

            // There's no matching key that already exists in the curve, so we have where the entry _should_ be
            _ => {
                Entry::Vacant(VacantEntry {
                    curve: self,
                    index,
                    key
                })
            }
        }
    }

    pub(crate) fn insert_index(&mut self, index: usize, element: (f32, f32)) {
        self.values.insert(index, element);
    }

    /// Finds the index where an entry with that key either is or would be located.
    fn find_entry_index(&self, key: f32) -> usize {
        let entry = self.values.iter().enumerate()
            // Find either the index the key is at, or the index it _would_ be at if it existed
            .find(|&(_, &(k, _))| epsilon_eq(key, k) || k > key);

        match entry {
            // If we found an index, it's simply that
            Some((index, _)) => {
                index
            },
            // Otherwise we hit the end of the curve, indicating the key goes at the very end
            None => {
                self.values.len()
            }
        }
    }

    fn find_by_key(&self, key: f32) -> Option<usize> {
        self.values.iter()
            // Find point where the key difference is less than machine epsilon. Aka, if they're "equal"
            .position(|&(k, _)| epsilon_eq(key, k))
    }

    /// Find the index where the supplied key should be inserted into the curve.
    /// 
    /// Note: does NOT check for equality. If the key already exists, this will arbitrarily
    /// return either side of it depending on accumulated error.
    fn find_insertion_point(&self, key: f32) -> usize {
        self.values.iter()
            // Find the first point whose key is greater than the key to insert
            .position(|&(k, _)| k > key)
            // If that point was found, insert at its index (shifting others right). Otherwise insert at the end
            .unwrap_or_else(|| self.values.len())
    }
}

/// Performs proper float "equality" by checking if absolute difference is less than machine epsilon.
#[inline]
fn epsilon_eq(a: f32, b: f32) -> bool {
    (a - b).abs() <= f32::EPSILON
}

#[cfg(test)]
mod tests {
    use super::{
        FloatCurve,
        EntryApi,
    };

    #[test]
    fn basic() {
        let mut curve = FloatCurve::new();

        curve.get_entry(1.0).insert(5.0);
        curve.get_entry(3.0).insert(10.0);
        curve.get_entry(5.0).insert(20.0);

        for index in 0..=16 {
            let key = 1.0 + (index as f32 * 0.25);
            let value = curve.get_value(key);
            println!("Point: ({}, {})", key, value);
        }
    }
}