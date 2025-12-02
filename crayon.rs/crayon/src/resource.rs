use std::any::Any;
use std::ops::{Deref, DerefMut};
use std::sync::{RwLockReadGuard, RwLockWriteGuard};

pub trait Resource: Send + Sync + 'static {}

pub struct Res<'w, T: Resource> {
    guard: RwLockReadGuard<'w, dyn Any + Send + Sync>,
    _phantom: std::marker::PhantomData<&'w T>,
}

impl<'w, T: Resource> Res<'w, T> {
    pub fn new(guard: RwLockReadGuard<'w, dyn Any + Send + Sync>) -> Self {
        Self {
            guard,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<'w, T: Resource> Deref for Res<'w, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.guard.downcast_ref::<T>().expect("Type mismatch")
    }
}

pub struct ResMut<'w, T: Resource> {
    guard: RwLockWriteGuard<'w, dyn Any + Send + Sync>,
    _phantom: std::marker::PhantomData<&'w mut T>,
}

impl<'w, T: Resource> ResMut<'w, T> {
    pub fn new(guard: RwLockWriteGuard<'w, dyn Any + Send + Sync>) -> Self {
        Self {
            guard,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<'w, T: Resource> Deref for ResMut<'w, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.guard.downcast_ref::<T>().expect("Type mismatch")
    }
}

impl<'w, T: Resource> DerefMut for ResMut<'w, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.guard.downcast_mut::<T>().expect("Type mismatch")
    }
}

pub trait ResourceContext {
    fn read<T: Resource>(&self) -> Option<Res<'_, T>>;
    fn write<T: Resource>(&self) -> Option<ResMut<'_, T>>;
    fn insert_resource<T: Resource>(&mut self, resource: T) -> &mut Self;
}
