use rusteal_core::{OwnedStruct, RustealError, RustealResult, SubclassOf, UObjectRef, UeClass};

use crate::core_ue::FTransform;
use crate::engine::{Actor, ActorExt, Pawn, World};

fn warn_no_root_component(handle: rusteal_ffi::UObjectHandle) {
    let actor_ref: UObjectRef<Actor> = unsafe { UObjectRef::from_raw(handle) };

    if let Ok(actual_class) = actor_ref.get_class() {
        if actual_class == Actor::static_class()
            || actual_class == Pawn::static_class()
        {
            return;
        }
    }

    if let Ok(checked) = actor_ref.checked() {
        let root = checked.k2_get_root_component();

        if !root.is_valid() {
            rusteal_core::ulog!(rusteal_core::LOG_WARNING,
                "[Rusteal] spawn_actor: spawned actor has no RootComponent — \
                 spawn transform will be ignored and set_actor_location will fail. \
                 Consider inheriting from a class with a default RootComponent."
            );
        }
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, Default)]
pub enum SpawnCollisionMethod {
    #[default]
    Undefined = 0,
    AlwaysSpawn = 1,
    AdjustIfPossibleButAlwaysSpawn = 2,
    AdjustIfPossibleButDontSpawnIfColliding = 3,
    DontSpawnIfColliding = 4,
}

pub trait ObjectWorldExt {
    fn get_world(&self) -> RustealResult<UObjectRef<World>>;
}

impl<T: UeClass> ObjectWorldExt for UObjectRef<T> {
    fn get_world(&self) -> RustealResult<UObjectRef<World>> {
        let handle = rusteal_core::world::get_world_raw(self.checked()?.raw())?;

        Ok(unsafe { UObjectRef::from_raw(handle) })
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SpawnParams {
    pub owner: Option<UObjectRef<Actor>>,
    pub instigator: Option<UObjectRef<Pawn>>,
    pub collision: SpawnCollisionMethod,
}

impl SpawnParams {
    fn owner_handle(&self) -> RustealResult<rusteal_ffi::UObjectHandle> {
        match self.owner {
            Some(owner) if !owner.raw().is_null() => Ok(owner.checked()?.raw()),
            _ => Ok(rusteal_ffi::UObjectHandle::null()),
        }
    }

    fn instigator_handle(&self) -> RustealResult<rusteal_ffi::UObjectHandle> {
        match self.instigator {
            Some(instigator) if !instigator.raw().is_null() => Ok(instigator.checked()?.raw()),
            _ => Ok(rusteal_ffi::UObjectHandle::null()),
        }
    }
}

pub trait WorldSpawnExt {
    fn spawn_actor<T: UeClass>(
        &self,
        transform: &OwnedStruct<FTransform>,
    ) -> RustealResult<UObjectRef<T>>;

    fn spawn_actor_with_owner<T: UeClass>(
        &self,
        transform: &OwnedStruct<FTransform>,
        owner: &UObjectRef<Actor>,
    ) -> RustealResult<UObjectRef<T>>;

    fn spawn_actor_of_class<T: UeClass>(
        &self,
        class: SubclassOf<T>,
        transform: &OwnedStruct<FTransform>,
    ) -> RustealResult<UObjectRef<T>>;

    fn spawn_actor_deferred<T: UeClass>(
        &self,
        transform: &OwnedStruct<FTransform>,
    ) -> RustealResult<UObjectRef<T>>;

    fn spawn_actor_deferred_full<T: UeClass>(
        &self,
        transform: &OwnedStruct<FTransform>,
        params: SpawnParams,
    ) -> RustealResult<UObjectRef<T>>;

    fn spawn_actor_deferred_of_class<T: UeClass>(
        &self,
        class: SubclassOf<T>,
        transform: &OwnedStruct<FTransform>,
        params: SpawnParams,
    ) -> RustealResult<UObjectRef<T>>;

    fn finish_spawning(
        &self,
        actor: &UObjectRef<Actor>,
        transform: &OwnedStruct<FTransform>,
    ) -> RustealResult<()>;

    fn get_all_actors_of_class<T: UeClass>(&self) -> RustealResult<Vec<UObjectRef<T>>>;
}

impl WorldSpawnExt for UObjectRef<World> {
    fn spawn_actor<T: UeClass>(
        &self,
        transform: &OwnedStruct<FTransform>,
    ) -> RustealResult<UObjectRef<T>> {
        let world = self.checked()?.raw();
        let class = T::static_class();
        let null_owner = rusteal_ffi::UObjectHandle::null();

        let handle = rusteal_core::world::spawn_actor_raw(
            world,
            class,
            &transform.to_bytes(),
            null_owner,
        )?;

        warn_no_root_component(handle);

        Ok(unsafe { UObjectRef::from_raw(handle) })
    }

    fn spawn_actor_with_owner<T: UeClass>(
        &self,
        transform: &OwnedStruct<FTransform>,
        owner: &UObjectRef<Actor>,
    ) -> RustealResult<UObjectRef<T>> {
        let world = self.checked()?.raw();
        let class = T::static_class();
        let owner_handle = owner.checked()?.raw();

        let handle = rusteal_core::world::spawn_actor_raw(
            world,
            class,
            &transform.to_bytes(),
            owner_handle,
        )?;

        warn_no_root_component(handle);

        Ok(unsafe { UObjectRef::from_raw(handle) })
    }

    fn spawn_actor_of_class<T: UeClass>(
        &self,
        class: SubclassOf<T>,
        transform: &OwnedStruct<FTransform>,
    ) -> RustealResult<UObjectRef<T>> {
        let world = self.checked()?.raw();

        if class.is_null() {
            return Err(RustealError::null_argument());
        }

        let handle = rusteal_core::world::spawn_actor_raw(
            world,
            class.raw(),
            &transform.to_bytes(),
            rusteal_ffi::UObjectHandle::null(),
        )?;

        warn_no_root_component(handle);

        Ok(unsafe { UObjectRef::from_raw(handle) })
    }

    fn spawn_actor_deferred<T: UeClass>(
        &self,
        transform: &OwnedStruct<FTransform>,
    ) -> RustealResult<UObjectRef<T>> {
        let world = self.checked()?.raw();
        let class = T::static_class();
        let null = rusteal_ffi::UObjectHandle::null();

        let handle = rusteal_core::world::spawn_actor_deferred_raw(
            world, class, &transform.to_bytes(), null, null, 0,
        )?;

        warn_no_root_component(handle);

        Ok(unsafe { UObjectRef::from_raw(handle) })
    }

    fn spawn_actor_deferred_full<T: UeClass>(
        &self,
        transform: &OwnedStruct<FTransform>,
        params: SpawnParams,
    ) -> RustealResult<UObjectRef<T>> {
        self.spawn_actor_deferred_of_class(SubclassOf::base(), transform, params)
    }

    fn spawn_actor_deferred_of_class<T: UeClass>(
        &self,
        class: SubclassOf<T>,
        transform: &OwnedStruct<FTransform>,
        params: SpawnParams,
    ) -> RustealResult<UObjectRef<T>> {
        let world = self.checked()?.raw();

        if class.is_null() {
            return Err(RustealError::null_argument());
        }

        let handle = rusteal_core::world::spawn_actor_deferred_raw(
            world,
            class.raw(),
            &transform.to_bytes(),
            params.owner_handle()?,
            params.instigator_handle()?,
            params.collision as u8,
        )?;

        warn_no_root_component(handle);

        Ok(unsafe { UObjectRef::from_raw(handle) })
    }

    fn finish_spawning(
        &self,
        actor: &UObjectRef<Actor>,
        transform: &OwnedStruct<FTransform>,
    ) -> RustealResult<()> {
        let actor_handle = actor.checked()?.raw();

        rusteal_core::world::finish_spawning_raw(actor_handle, &transform.to_bytes())
    }

    fn get_all_actors_of_class<T: UeClass>(&self) -> RustealResult<Vec<UObjectRef<T>>> {
        let world = self.checked()?.raw();
        let class = T::static_class();
        let handles = rusteal_core::world::get_all_actors_of_class_raw(world, class)?;

        Ok(handles
            .into_iter()
            .map(|h| unsafe { UObjectRef::from_raw(h) })
            .collect())
    }
}

pub fn find_object<T: UeClass>(path: &str) -> RustealResult<UObjectRef<T>> {
    let class = T::static_class();
    let handle = rusteal_core::world::find_object_raw(class, path)?;
    Ok(unsafe { UObjectRef::from_raw(handle) })
}

pub fn load_object<T: UeClass>(path: &str) -> RustealResult<UObjectRef<T>> {
    let class = T::static_class();
    let handle = rusteal_core::world::load_object_raw(class, path)?;
    Ok(unsafe { UObjectRef::from_raw(handle) })
}

pub fn load_class<T: UeClass>(path: &str) -> RustealResult<SubclassOf<T>> {
    let class = crate::core_ue::Class::static_class();
    let handle = rusteal_core::world::load_object_raw(class, path)?;
    SubclassOf::from_class(rusteal_ffi::UClassHandle(handle.0))
}

pub fn new_object<T: UeClass>(outer: &UObjectRef<impl UeClass>) -> RustealResult<UObjectRef<T>> {
    let outer_handle = outer.checked()?.raw();
    let class = T::static_class();
    let handle = rusteal_core::world::new_object_raw(outer_handle, class)?;
    Ok(unsafe { UObjectRef::from_raw(handle) })
}

pub fn new_object_transient<T: UeClass>() -> RustealResult<UObjectRef<T>> {
    let class = T::static_class();
    let null_outer = rusteal_ffi::UObjectHandle::null();
    let handle = rusteal_core::world::new_object_raw(null_outer, class)?;
    Ok(unsafe { UObjectRef::from_raw(handle) })
}

pub fn spawn_actor_dynamic(
    world: &UObjectRef<World>,
    class: rusteal_ffi::UClassHandle,
    transform: &OwnedStruct<FTransform>,
) -> RustealResult<rusteal_ffi::UObjectHandle> {
    let world_handle = world.checked()?.raw();
    let null_owner = rusteal_ffi::UObjectHandle::null();
    rusteal_core::world::spawn_actor_raw(world_handle, class, &transform.to_bytes(), null_owner)
}

pub fn new_object_dynamic(
    outer: rusteal_ffi::UObjectHandle,
    class: rusteal_ffi::UClassHandle,
) -> RustealResult<rusteal_ffi::UObjectHandle> {
    rusteal_core::world::new_object_raw(outer, class)
}
