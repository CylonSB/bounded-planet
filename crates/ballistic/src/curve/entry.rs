use super::Curve;

/// 2d lerp producing value y that corresponds with interpolation parameter x
/// input value x is in the range [x0, x1], output value y is in the range [y0, y1]
fn lerp_2d((x0, y0): (f32, f32), (x1, y1): (f32, f32), x: f32) -> f32 {
    y0 + (x - x0) * ((y1-y0) / (x1-x0))
}

pub trait EntryApi<V>
where
    Self: Sized
{
    /// Current index of the predecessor element in the curve.
    fn predecessor(&self) -> usize;
    /// Current index of the successor element in the curve.
    fn successor(&self) -> usize;

    /// The value of this entry in the curve.
    /// 
    /// If the entry is occupied, returns exactly the value at that point.
    /// If the entry is vacant though, this interpolates from the two nearest values.
    fn value(&self) -> V
    where
        V: Copy;

    fn insert(self, value: V);
}

#[derive(Debug)]
pub enum Entry<'a, K, V> {
    Vacant(VacantEntry<'a, K, V>),
    Occupied(OccupiedEntry<'a, K, V>),
}

impl EntryApi<f32> for Entry<'_, f32, f32> {
    fn predecessor(&self) -> usize {
        match self {
            Entry::Vacant(entry) => entry.predecessor(),
            Entry::Occupied(entry) => entry.predecessor()
        }
    }

    fn successor(&self) -> usize {
        match self {
            Entry::Vacant(entry) => entry.successor(),
            Entry::Occupied(entry) => entry.successor()
        }
    }

    fn value(&self) -> f32 {
        match self {
            Entry::Vacant(entry) => entry.value(),
            Entry::Occupied(entry) => entry.value()
        }
    }

    fn insert(self, value: f32) {
        match self {
            Entry::Vacant(entry) => entry.insert(value),
            Entry::Occupied(entry) => entry.insert(value)
        }
    }
}

#[derive(Debug)]
pub struct VacantEntry<'a, K, V> {
    pub curve: &'a mut Curve<K, V>,
    /// The index where this entry _would_ be found if it actually existed.
    /// The element currently at that index would be the successor of this entry if it were to exist.
    pub index: usize,
    pub key: K,
}

impl EntryApi<f32> for VacantEntry<'_, f32, f32> {
    fn predecessor(&self) -> usize {
        self.index - 1
    }

    fn successor(&self) -> usize {
        // As this entry does not exist, the index where it _would_ exist is the index of the element that would be its successor.
        self.index
    }

    fn value(&self) -> f32
    {
        let &(pk, pv) = self.curve.values.get(self.predecessor()).unwrap();
        let &(sk, sv) = self.curve.values.get(self.successor()).unwrap();

        lerp_2d((pk, pv), (sk, sv), self.key)
    }

    fn insert(self, value: f32) {
        self.curve.insert_index(self.index, (self.key, value));
    }
}

#[derive(Debug)]
pub struct OccupiedEntry<'a, K, V> {
    pub curve: &'a mut Curve<K, V>,
    /// The index where this entry is actually found.
    pub index: usize
}

impl<K, V> EntryApi<V> for OccupiedEntry<'_, K, V> {
    fn predecessor(&self) -> usize {
        self.index - 1
    }

    fn successor(&self) -> usize {
        self.index + 1
    }

    fn value(&self) -> V
    where
        V: Copy
    {
        self.curve.values[self.index].1
    }

    fn insert(self, value: V) {
        self.curve.values[self.index].1 = value;
    }
}