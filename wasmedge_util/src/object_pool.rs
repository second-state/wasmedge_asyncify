#[derive(Debug)]
pub struct ObjectPool<T> {
    stores: Vec<Vec<ObjectNode<T>>>,
}

impl<T> ObjectPool<T> {
    const DEFAULT_CAPACITY: usize = 512;

    pub fn new() -> Self {
        let mut pool = ObjectPool {
            stores: Vec::with_capacity(10),
        };
        pool.extend_stores();
        pool
    }

    #[inline]
    fn raw_index(index: usize) -> (usize, usize) {
        (
            index / Self::DEFAULT_CAPACITY,
            index % Self::DEFAULT_CAPACITY,
        )
    }

    fn extend_stores(&mut self) -> &mut [ObjectNode<T>] {
        let mut new_vec = Vec::with_capacity(Self::DEFAULT_CAPACITY);
        new_vec.resize_with(Self::DEFAULT_CAPACITY, ObjectNode::default);
        new_vec[0].header.next_chunk_offset = Self::DEFAULT_CAPACITY;
        self.stores.push(new_vec);
        self.stores.last_mut().unwrap()
    }

    pub fn push(&mut self, value: T) -> (usize, Option<T>) {
        if let Some(ObjectIndex {
            store_index,
            target_index,
            chunk_index,
        }) = self.first_none()
        {
            let v = self.stores[store_index][target_index].obj.replace(value);

            //try merge chunk
            {
                let current_chunk = &mut self.stores[store_index][chunk_index].header;
                current_chunk.next_none_offset += 1;
                if current_chunk.next_none_offset == current_chunk.next_chunk_offset
                    && current_chunk.next_chunk_offset < Self::DEFAULT_CAPACITY
                {
                    let next_chunk_offset = current_chunk.next_chunk_offset;
                    let next_chunk = self.stores[store_index][next_chunk_offset].header;

                    let current_chunk = &mut self.stores[store_index][chunk_index].header;
                    *current_chunk = next_chunk;
                }
            }

            (store_index * Self::DEFAULT_CAPACITY + target_index, v)
        } else {
            let store = self.extend_stores();
            let node = &mut store[0];
            node.header.next_none_offset += 1;
            let v = node.obj.replace(value);
            ((self.stores.len() - 1) * Self::DEFAULT_CAPACITY, v)
        }
    }

    pub fn remove(&mut self, index: usize) -> Option<T> {
        let (
            ObjectIndex {
                store_index,
                target_index,
                chunk_index,
            },
            last_chunk_index,
        ) = self.value_and_chunk(index)?;

        let v = self.stores[store_index][target_index].obj.take()?;
        let current_chunk = &mut self.stores[store_index][chunk_index].header;

        // try update chunk header
        {
            if target_index == current_chunk.next_none_offset - 1 {
                current_chunk.next_none_offset = target_index;
                if chunk_index == current_chunk.next_none_offset {
                    let next_chunk = current_chunk.next_chunk_offset;
                    let last_chunk = &mut self.stores[store_index][last_chunk_index].header;
                    last_chunk.next_chunk_offset = next_chunk;
                }
            } else if target_index == chunk_index {
                if index == 0 {
                    let new_chunk = *current_chunk;
                    current_chunk.next_none_offset = 0;
                    current_chunk.next_chunk_offset = 1;
                    let next_chunk = &mut self.stores[store_index][1].header;
                    *next_chunk = new_chunk;
                } else {
                    let new_chunk = *current_chunk;
                    let next_chunk = &mut self.stores[store_index][target_index + 1].header;
                    *next_chunk = new_chunk;
                    let last_chunk = &mut self.stores[store_index][last_chunk_index].header;
                    last_chunk.next_chunk_offset += 1;
                }
            } else {
                let new_chunk = *current_chunk;
                current_chunk.next_chunk_offset = target_index + 1;
                current_chunk.next_none_offset = target_index;
                let next_chunk = &mut self.stores[store_index][target_index + 1].header;
                *next_chunk = new_chunk;
            }
        }

        Some(v)
    }

    fn empty_chunk(&self) -> Option<(usize, bool)> {
        let chunk_headers = self.stores.iter().map(|s| s[0].header).rev();

        if chunk_headers.len() <= 1 {
            return None;
        }

        let empty_chunk = ChunkHead {
            next_none_offset: 0,
            next_chunk_offset: Self::DEFAULT_CAPACITY,
        };

        let mut empty_num = 0;
        let mut res_chunk_is_full = false;

        for chunk in chunk_headers {
            if chunk == empty_chunk {
                empty_num += 1;
            } else {
                if chunk.next_chunk_offset == chunk.next_none_offset {
                    res_chunk_is_full = true;
                }
                break;
            }
        }
        if empty_num == 0 {
            None
        } else {
            Some((empty_num, res_chunk_is_full))
        }
    }

    pub fn cleanup_stores(&mut self) {
        if let Some((mut n, res_is_full)) = self.empty_chunk() {
            if res_is_full {
                n -= 1;
            }
            for _ in 0..n {
                self.stores.pop();
            }
        }
    }

    fn first_none(&self) -> Option<ObjectIndex> {
        for (store_index, store) in self.stores.iter().enumerate() {
            'next_chunk: loop {
                let node = &store[0];
                let header = node.header;
                if header.next_none_offset == Self::DEFAULT_CAPACITY {
                    break 'next_chunk;
                }
                return Some(ObjectIndex {
                    store_index,
                    target_index: header.next_none_offset,
                    chunk_index: 0,
                });
            }
        }
        None
    }

    fn value_and_chunk(&self, index: usize) -> Option<(ObjectIndex, usize)> {
        let (store_index, value_index) = Self::raw_index(index);
        let store = self.stores.get(store_index)?;
        let mut last_chunk_index = 0;
        let mut chunk_index = 0;
        loop {
            let node = &store[chunk_index];
            debug_assert!(node.header.next_chunk_offset > chunk_index);
            if value_index < node.header.next_chunk_offset {
                return Some((
                    ObjectIndex {
                        store_index,
                        target_index: value_index,
                        chunk_index,
                    },
                    last_chunk_index,
                ));
            }
            last_chunk_index = chunk_index;
            chunk_index = node.header.next_chunk_offset;
            debug_assert!(chunk_index < Self::DEFAULT_CAPACITY);
            if chunk_index >= Self::DEFAULT_CAPACITY {
                return None;
            }
        }
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        let (store_index, target_index) = Self::raw_index(index);
        let store = self.stores.get(store_index)?;
        let obj_node = &store[target_index];
        obj_node.obj.as_ref()
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        let (store_index, target_index) = Self::raw_index(index);
        let store = self.stores.get_mut(store_index)?;
        let obj_node = &mut store[target_index];
        obj_node.obj.as_mut()
    }
}

impl<T: Clone> Clone for ObjectPool<T> {
    fn clone(&self) -> Self {
        ObjectPool {
            stores: self.stores.clone(),
        }
    }
}

impl<T> ObjectPool<T> {
    pub fn iter(&self) -> impl Iterator<Item = Option<&T>> {
        let skip_end = self.empty_chunk().map(|(n, _)| n).unwrap_or(0);
        let stores_len = self.stores.len();
        self.stores[0..(stores_len - skip_end)]
            .iter()
            .flat_map(|store| store.iter())
            .map(|node| node.obj.as_ref())
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = Option<&mut T>> {
        let skip_end = self.empty_chunk().map(|(n, _)| n).unwrap_or(0);
        let stores_len = self.stores.len();
        self.stores[0..(stores_len - skip_end)]
            .iter_mut()
            .flat_map(|store| store.iter_mut())
            .map(|node| node.obj.as_mut())
    }
}

struct ObjectIndex {
    store_index: usize,
    target_index: usize,
    chunk_index: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct ChunkHead {
    next_none_offset: usize,
    next_chunk_offset: usize,
}

#[derive(Debug)]
pub struct ObjectNode<T> {
    obj: Option<T>,
    header: ChunkHead,
}

impl<T> Default for ObjectNode<T> {
    fn default() -> Self {
        Self {
            obj: None,
            header: ChunkHead::default(),
        }
    }
}

impl<T: Clone> Clone for ObjectNode<T> {
    fn clone(&self) -> Self {
        ObjectNode {
            obj: self.obj.clone(),
            header: self.header.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iter() {
        let mut pool = ObjectPool::new();
        let _ = pool.push("hello");
        let (id2, _) = pool.push("world");
        pool.push("example");
        pool.remove(id2);
        pool.push("foo");

        let r = pool
            .iter()
            .take(4)
            .map(|s| s.map(|s| *s))
            .collect::<Vec<Option<&str>>>();

        assert_eq!(r, vec![Some("hello"), Some("foo"), Some("example"), None]);
    }

    #[test]
    fn test_push() {
        let mut pool = ObjectPool::new();
        assert_eq!(pool.push("0"), (0, None));
        assert_eq!(pool.push("1"), (1, None));
        assert_eq!(pool.push("2"), (2, None));
        assert_eq!(pool.push("3"), (3, None));
        assert_eq!(pool.push("4"), (4, None));
        assert_eq!(
            pool.stores[0][0].header,
            ChunkHead {
                next_none_offset: 5,
                next_chunk_offset: ObjectPool::<&str>::DEFAULT_CAPACITY
            }
        );
    }

    #[test]
    fn test_push_and_remove() {
        let mut pool = ObjectPool::new();

        assert_eq!(pool.push("0"), (0, None));
        assert_eq!(pool.push("1"), (1, None));
        assert_eq!(pool.push("2"), (2, None));
        assert_eq!(pool.push("3"), (3, None));
        assert_eq!(pool.push("4"), (4, None));
        // |----------*
        // |0|1|2|3|4|
        // |----------DEFAULT_CAPACITY--*
        assert_eq!(pool.remove(2), Some("2"));
        assert_eq!(
            pool.stores[0][0].header,
            ChunkHead {
                next_none_offset: 2,
                next_chunk_offset: 3
            }
        );
        assert_eq!(
            pool.stores[0][3].header,
            ChunkHead {
                next_none_offset: 5,
                next_chunk_offset: ObjectPool::<&str>::DEFAULT_CAPACITY
            }
        );
        // |----*|----*
        // |0|1|_|3|4|
        // |------|-----DEFAULT_CAPACITY--*

        assert_eq!(pool.remove(1), Some("1"));
        // |--*  |----*
        // |0|_|_|3|4|
        // |------|-----DEFAULT_CAPACITY--*
        assert_eq!(
            pool.stores[0][0].header,
            ChunkHead {
                next_none_offset: 1,
                next_chunk_offset: 3
            }
        );

        assert_eq!(pool.remove(3), Some("3"));
        // |--*    |--*
        // |0|_|_|_|4|
        // |--------|-----DEFAULT_CAPACITY--*
        assert_eq!(
            pool.stores[0][0].header,
            ChunkHead {
                next_none_offset: 1,
                next_chunk_offset: 4
            }
        );
        assert_eq!(
            pool.stores[0][4].header,
            ChunkHead {
                next_none_offset: 5,
                next_chunk_offset: ObjectPool::<&str>::DEFAULT_CAPACITY
            }
        );

        assert_eq!(pool.push("1"), (1, None));
        // |----*  |--*
        // |0|1|_|_|4|
        // |--------|-----DEFAULT_CAPACITY--*
        assert_eq!(
            pool.stores[0][0].header,
            ChunkHead {
                next_none_offset: 2,
                next_chunk_offset: 4
            }
        );

        assert_eq!(pool.remove(0), Some("0"));
        // |*|--*  |--*
        // |_|1|_|_|4|
        // |--|-----|-----DEFAULT_CAPACITY--*
        assert_eq!(
            pool.stores[0][0].header,
            ChunkHead {
                next_none_offset: 0,
                next_chunk_offset: 1
            }
        );
        assert_eq!(
            pool.stores[0][1].header,
            ChunkHead {
                next_none_offset: 2,
                next_chunk_offset: 4
            }
        );
        let v = pool
            .iter()
            .take(5)
            .map(|s| s.map(|s| *s))
            .collect::<Vec<Option<&str>>>();
        assert_eq!(v, vec![None, Some("1"), None, None, Some("4")]);

        assert_eq!(pool.remove(1), Some("1"));
        // |*      |--*
        // |_|_|_|_|4|
        // |--------|-----DEFAULT_CAPACITY--*
        assert_eq!(
            pool.stores[0][0].header,
            ChunkHead {
                next_none_offset: 0,
                next_chunk_offset: 4
            }
        );
        assert_eq!(
            pool.stores[0][4].header,
            ChunkHead {
                next_none_offset: 5,
                next_chunk_offset: ObjectPool::<&str>::DEFAULT_CAPACITY
            }
        );
        let v = pool
            .iter()
            .take(5)
            .map(|s| s.map(|s| *s))
            .collect::<Vec<Option<&str>>>();
        assert_eq!(v, vec![None, None, None, None, Some("4")]);

        assert_eq!(pool.push("0"), (0, None));
        assert_eq!(pool.push("1"), (1, None));
        assert_eq!(pool.push("2"), (2, None));
        // |------*|--*
        // |0|1|2|_|4|
        // |--------|-----DEFAULT_CAPACITY--*
        assert_eq!(
            pool.stores[0][0].header,
            ChunkHead {
                next_none_offset: 3,
                next_chunk_offset: 4
            }
        );
        assert_eq!(
            pool.stores[0][4].header,
            ChunkHead {
                next_none_offset: 5,
                next_chunk_offset: ObjectPool::<&str>::DEFAULT_CAPACITY
            }
        );

        assert_eq!(pool.push("3"), (3, None));
        // |----------*
        // |0|1|2|3|4|
        // |--------------DEFAULT_CAPACITY--*
        assert_eq!(
            pool.stores[0][0].header,
            ChunkHead {
                next_none_offset: 5,
                next_chunk_offset: ObjectPool::<&str>::DEFAULT_CAPACITY
            }
        );
    }

    #[test]
    fn test_store_extends() {
        let mut pool = ObjectPool::new();
        let cap = ObjectPool::<&str>::DEFAULT_CAPACITY;
        for i in 0..cap {
            assert_eq!(pool.push(format!("{i}")), (i, None));
        }

        assert_eq!(pool.push("example".to_string()), (cap, None));
        assert_eq!(pool.push("foo".to_string()), (cap + 1, None));
        assert_eq!(pool.push("bar".to_string()), (cap + 2, None));

        assert_eq!(
            pool.stores[1][0].header,
            ChunkHead {
                next_none_offset: 3,
                next_chunk_offset: ObjectPool::<&str>::DEFAULT_CAPACITY
            }
        );
    }

    #[test]
    fn test_cleanup() {
        let mut pool = ObjectPool::new();
        let cap = ObjectPool::<()>::DEFAULT_CAPACITY;
        for i in 0..cap {
            pool.push(i);
        }
        pool.extend_stores();
        pool.extend_stores();

        assert_eq!(pool.stores.len(), 3);

        assert_eq!(
            pool.stores[0][0].header,
            ChunkHead {
                next_none_offset: cap,
                next_chunk_offset: cap
            }
        );

        pool.cleanup_stores();
        assert_eq!(pool.stores.len(), 2);

        pool.remove(2);
        pool.cleanup_stores();
        assert_eq!(pool.stores.len(), 1);
    }
}
