use std::sync::{Arc, Mutex};

use crate::FrameBuffer;

#[derive(Debug)]
struct PoolInner {
    buffers: Mutex<Vec<Vec<u8>>>,
    capacity: usize,
}

#[derive(Debug, Clone)]
pub struct FramePool {
    inner: Arc<PoolInner>,
}

impl FramePool {
    pub fn new(capacity: usize) -> Self {
        Self {
            inner: Arc::new(PoolInner {
                buffers: Mutex::new(Vec::with_capacity(capacity)),
                capacity,
            }),
        }
    }

    pub fn acquire(&self, len: usize) -> FrameBuffer {
        let mut data = self
            .inner
            .buffers
            .lock()
            .expect("frame pool mutex poisoned")
            .pop()
            .unwrap_or_default();
        data.resize(len, 0);
        FrameBuffer::new(data, self.clone())
    }

    pub(crate) fn recycle(&self, mut data: Vec<u8>) {
        let mut buffers = self
            .inner
            .buffers
            .lock()
            .expect("frame pool mutex poisoned");
        if buffers.len() < self.inner.capacity {
            data.clear();
            buffers.push(data);
        }
    }

    #[cfg(test)]
    fn available(&self) -> usize {
        self.inner.buffers.lock().unwrap().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dropped_buffers_return_to_pool() {
        let pool = FramePool::new(3);
        {
            let buffer = pool.acquire(128);
            assert_eq!(buffer.len(), 128);
        }
        assert_eq!(pool.available(), 1);
        assert_eq!(pool.acquire(64).len(), 64);
    }
}
