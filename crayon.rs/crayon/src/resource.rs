use std::any::Any;
use std::ops::{Deref, DerefMut};
use std::sync::{RwLockReadGuard, RwLockWriteGuard};

pub trait Resource: Send + Sync + 'static {}

/// A read-only reference to a resource of type `T`.
///
/// This type provides immutable access to the underlying stored data.
/// It dereferences to `&T`, allowing access to the resource's methods and fields.
///
/// # Examples
///
/// ```ignore
/// let res: Res<MyResource> = world.read::<MyResource>()?;
/// println!("Value: {}", res.value);
/// ```
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

/// A mutable reference to a resource of type `T`.
///
/// This type provides mutable access to the underlying stored data.
/// It dereferences to `&T` for read access and `&mut T` for write access.
///
/// # Examples
///
/// ```ignore
/// let mut res: ResMut<MyResource> = world.write::<MyResource>()?;
/// res.value = 42;
/// ```
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

/// Encapsulation layer for managing resources of any type that implements the `Resource` trait.
/// Resources are stored and accessed using their concrete types.
///
/// # Examples
///
/// ```ignore
/// let mut context = MyContext::new();
/// context.insert_resource(MyResource { value: 42 });
///
/// let ctx_res = context.read::<MyResource>()?;
/// println!("Value: {}", ctx_res.value);
///
/// let mut ctx_res = context.write::<MyResource>()?;
/// ctx_res.value = 100;
/// ```
pub trait ResourceContext {
    fn read<T: Resource>(&self) -> Option<Res<'_, T>>;
    fn write<T: Resource>(&self) -> Option<ResMut<'_, T>>;
    fn insert_resource<T: Resource>(&mut self, resource: T) -> &mut Self;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::any::TypeId;
    use std::collections::HashMap;
    use std::sync::{Arc, RwLock};

    struct TestResource {
        value: i32,
    }

    impl Resource for TestResource {}

    struct TestContext {
        resources: HashMap<TypeId, Arc<RwLock<dyn Any + Send + Sync>>>,
    }

    impl TestContext {
        fn new() -> Self {
            Self {
                resources: HashMap::new(),
            }
        }
    }

    impl ResourceContext for TestContext {
        fn read<T: Resource>(&self) -> Option<Res<'_, T>> {
            let guard = self.resources.get(&TypeId::of::<T>())?.read().ok()?;

            Some(Res::new(guard))
        }

        fn write<T: Resource>(&self) -> Option<ResMut<'_, T>> {
            let guard = self.resources.get(&TypeId::of::<T>())?.write().ok()?;

            Some(ResMut::new(guard))
        }

        fn insert_resource<T: Resource>(&mut self, resource: T) -> &mut Self {
            self.resources
                .insert(TypeId::of::<T>(), Arc::new(RwLock::new(resource)));

            self
        }
    }

    #[test]
    fn test_resource_context_insert_and_read() {
        let mut ctx = TestContext::new();
        ctx.insert_resource(TestResource { value: 42 });

        let res = ctx.read::<TestResource>().expect("Resource should exist");
        assert_eq!(res.value, 42);
    }
    #[test]

    fn test_resource_context_insert_and_write() {
        let mut ctx = TestContext::new();
        ctx.insert_resource(TestResource { value: 42 });

        let mut res = ctx.write::<TestResource>().expect("Resource should exist");
        res.value = 24;
        assert_eq!(res.value, 24);
    }
}
