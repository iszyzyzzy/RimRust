use super::*;

use parking_lot::Mutex as SyncMutex;
use std::collections::BinaryHeap;
use std::time::SystemTime;
use tokio::sync::oneshot;
use std::cell::UnsafeCell;
use tracing::warn;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Priority(u8);

impl Priority {
    pub const HIGH: Option<Self> = Some(Self(200));
    pub const LOW: Option<Self> = Some(Self(50));
    pub fn _new(priority: u8) -> Option<Self> {
        Some(Self(priority))
    }
}

impl Default for Priority {
    fn default() -> Self {
        Self(50)
    }
}

#[derive(Debug)]
struct WaitEntry {
    priority: Priority,
    timestamp: SystemTime,
    sender: oneshot::Sender<()>,
}

impl PartialEq for WaitEntry {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority && self.timestamp == other.timestamp
    }
}

impl Eq for WaitEntry {}

impl Ord for WaitEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.priority
            .cmp(&other.priority)
            .then_with(|| self.timestamp.cmp(&other.timestamp))
    }
}

impl PartialOrd for WaitEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

pub struct PriorityMutex<T> {
    data: UnsafeCell<T>,
    waiters: SyncMutex<PriorityMutexWaiter>,
}

impl<T: Default> Default for PriorityMutex<T> {
    fn default() -> Self {
        Self::new(Default::default())
    }
}

pub struct PriorityMutexWaiter {
    waiters: BinaryHeap<WaitEntry>,
    free: bool,
}

pub struct PriorityMutexGuard<'a, T> {
    mutex: &'a PriorityMutex<T>,
    _marker: std::marker::PhantomData<&'a mut T>,
}

// 为 PriorityMutex 实现 Send 和 Sync
unsafe impl<T> Send for PriorityMutex<T> where T: Send {}
unsafe impl<T> Sync for PriorityMutex<T> where T: Send {}

impl<T> PriorityMutex<T> {
    const TIMEOUT:std::time::Duration = std::time::Duration::from_secs(60);
    pub fn new(value: T) -> Self {
        Self {
            data: UnsafeCell::new(value),
            waiters: SyncMutex::new(PriorityMutexWaiter {
                waiters: BinaryHeap::new(),
                free: true,
            }),
        }
    }

    pub async fn lock(&self, priority: Option<Priority>) -> PriorityMutexGuard<T> {
        let priority = priority.unwrap_or_default();

        loop {
            let (sender, receiver) = oneshot::channel();
            {
                let mut inner = self.waiters.lock();
                if inner.free {
                    inner.free = false;
                    return PriorityMutexGuard {
                        mutex: self,
                        _marker: std::marker::PhantomData,
                    };
                }
                inner.waiters.push(WaitEntry {
                    priority,
                    timestamp: SystemTime::now(),
                    sender,
                });
            }
            match tokio::time::timeout(Self::TIMEOUT, receiver).await {
                Ok(Ok(_)) => {
                }
                Ok(Err(_)) => {
                    warn!("PriorityMutex oneshot dropped without being resolved, retrying");
                }
                Err(_) => {
                    panic!("PriorityMutex lock timed out");
                }
            }
        }
    }

    pub async fn lock_h(&self) -> PriorityMutexGuard<T> {
        self.lock(Priority::HIGH).await
    }

    pub async fn _lock_l(&self) -> PriorityMutexGuard<T> {
        self.lock(Priority::LOW).await
    } 
}

impl<T> Drop for PriorityMutexGuard<'_, T> {
    fn drop(&mut self) {
        let mut inner = self.mutex.waiters.lock();
        loop {
            if let Some(entry) = inner.waiters.pop() {
                match entry.sender.send(()) {
                    Ok(_) => {
                        inner.free = true;
                        break;
                    },
                    Err(_) => {
                        continue;
                    },
                }
            } else {
                inner.free = true;
                break;
            }
        }
    }
}

use std::ops::{Deref, DerefMut};

impl<'a, T> Deref for PriorityMutexGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.mutex.data.get() }
    }
}

impl<'a, T> DerefMut for PriorityMutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.mutex.data.get() }
    }
}