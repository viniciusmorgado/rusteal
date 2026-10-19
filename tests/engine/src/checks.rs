use std::sync::{Arc, Mutex};

use bindings::core_ue::{FGuid, FInstancedStruct, FIntPoint, FIntPointExt};
use bindings::engine::{
    ActorComponent, BlendableInterface, CameraComponent, CameraComponentExt, CurveFloat,
    CurveFloatExt, FMinimalViewInfoExt, FPostProcessSettingsExt, FRichCurveExt, FRichCurveKey,
    FRichCurveKeyExt, FWeightedBlendableExt, FWeightedBlendablesExt, InputSettings,
    InputSettingsExt, KismetGuidLibrary, KismetMathLibrary, KismetStringLibrary,
    KismetSystemLibrary, KismetTextLibrary, Material, PlayerInput, PostProcessVolume,
    PostProcessVolumeExt, ProjectileMovementComponent, ProjectileMovementComponentExt,
    StaticMeshActor, TextRenderComponent, TextRenderComponentExt, TimelineComponent,
    TimelineComponentExt, VolumetricCloudComponent, VolumetricCloudComponentExt,
};
use bindings::prelude::*;
use bindings::slate_core::{
    EVerticalAlignment, FSlateBrush, FToolBarStyle, FToolBarStyleExt, FWrapButtonStyle,
    FWrapButtonStyleExt,
};
use rusteal_runtime::runtime::{
    DynamicCall, Inherits, OwnedStruct, SoftClassRef, SoftObjectRef, SubclassOf, TWeakObjectPtr,
    Text, Transform, UObjectRef, UeClass, UeStruct, task,
};

use crate::fixtures::TestActor;
use crate::runner::{Check, Context, Outcome};

pub const ALL: &[(&str, Check)] = &[
    ("rust_class_is_registered", rust_class_is_registered),
    ("spawn_rust_actor", spawn_rust_actor),
    ("scalar_properties_round_trip", scalar_properties_round_trip),
    ("string_property_round_trip", string_property_round_trip),
    (
        "ufunction_called_by_reflection",
        ufunction_called_by_reflection,
    ),
    (
        "ufunction_return_by_reflection",
        ufunction_return_by_reflection,
    ),
    (
        "ufunction_param_by_reflection",
        ufunction_param_by_reflection,
    ),
    ("ufunction_echo_by_reflection", ufunction_echo_by_reflection),
    ("component_is_created", component_is_created),
    ("udelegate_reaches_a_closure", udelegate_reaches_a_closure),
    (
        "udelegate_listener_survives_garbage_collection",
        udelegate_listener_survives_garbage_collection,
    ),
    ("engine_function_call", engine_function_call),
    ("engine_string_is_utf8", engine_string_is_utf8),
    (
        "engine_string_longer_than_512_bytes",
        engine_string_longer_than_512_bytes,
    ),
    (
        "rust_string_property_longer_than_512_bytes",
        rust_string_property_longer_than_512_bytes,
    ),
    (
        "engine_function_takes_an_interface",
        engine_function_takes_an_interface,
    ),
    (
        "engine_function_takes_any_object_as_iinterface",
        engine_function_takes_any_object_as_iinterface,
    ),
    (
        "engine_soft_property_keeps_the_path",
        engine_soft_property_keeps_the_path,
    ),
    (
        "engine_soft_reference_crosses_as_a_path",
        engine_soft_reference_crosses_as_a_path,
    ),
    (
        "engine_weak_property_follows_its_object",
        engine_weak_property_follows_its_object,
    ),
    (
        "engine_struct_with_an_array_returned_by_a_function",
        engine_struct_with_an_array_returned_by_a_function,
    ),
    (
        "engine_struct_passed_by_reference",
        engine_struct_passed_by_reference,
    ),
    (
        "engine_soft_class_reference_crosses_as_a_path",
        engine_soft_class_reference_crosses_as_a_path,
    ),
    (
        "engine_soft_class_property_keeps_the_path",
        engine_soft_class_property_keeps_the_path,
    ),
    ("engine_text_round_trip", engine_text_round_trip),
    ("engine_optional_fields", engine_optional_fields),
    (
        "instanced_struct_holds_a_struct",
        instanced_struct_holds_a_struct,
    ),
    (
        "engine_structs_without_static_struct",
        engine_structs_without_static_struct,
    ),
    (
        "engine_text_keeps_its_localization",
        engine_text_keeps_its_localization,
    ),
    (
        "engine_function_takes_a_closure",
        engine_function_takes_a_closure,
    ),
    (
        "engine_member_function_takes_a_closure_with_arguments",
        engine_member_function_takes_a_closure_with_arguments,
    ),
    (
        "task_comes_back_to_the_game_thread",
        task_comes_back_to_the_game_thread,
    ),
];

fn ensure(condition: bool, what: impl Into<String>) -> Result<(), String> {
    if condition { Ok(()) } else { Err(what.into()) }
}

fn done(result: Result<(), String>) -> Outcome {
    Outcome::Done(result)
}

fn spawn(context: &Context) -> Result<UObjectRef<TestActor>, String> {
    let transform = FTransform::from_transform(Transform::IDENTITY);

    context
        .world
        .spawn_actor::<TestActor>(&transform)
        .map_err(|e| format!("spawn: {e:?}"))
}

fn rust_class_is_registered(_: &Context) -> Outcome {
    done(ensure(
        !TestActor::static_class().is_null(),
        "TestActor has no UClass",
    ))
}

fn spawn_rust_actor(context: &Context) -> Outcome {
    done(spawn(context).map(|_| ()))
}

fn scalar_properties_round_trip(context: &Context) -> Outcome {
    done((|| {
        let actor = TestActor::from_obj(spawn(context)?).map_err(|e| format!("{e:?}"))?;
        actor.set_count(41);
        actor.set_speed(2.5);
        actor.set_b_enabled(true);
        ensure(actor.count() == 41, format!("count is {}", actor.count()))?;
        ensure(actor.speed() == 2.5, format!("speed is {}", actor.speed()))?;
        ensure(actor.b_enabled(), "enabled is false")
    })())
}

fn string_property_round_trip(context: &Context) -> Outcome {
    done((|| {
        let actor = TestActor::from_obj(spawn(context)?).map_err(|e| format!("{e:?}"))?;
        let text = "héllo wörld";
        actor.set_label(text);
        ensure(
            actor.label() == text,
            format!("label is {:?}", actor.label()),
        )
    })())
}

fn ufunction_called_by_reflection(context: &Context) -> Outcome {
    done((|| {
        let actor = spawn(context)?;
        let mut call = DynamicCall::new(&actor, "Add").map_err(|e| format!("{e:?}"))?;
        call.set("A", 2i32).map_err(|e| format!("{e:?}"))?;
        call.set("B", 3i32).map_err(|e| format!("{e:?}"))?;
        let result = call.call().map_err(|e| format!("{e:?}"))?;
        let sum: i32 = result.get("ReturnValue").map_err(|e| format!("{e:?}"))?;
        ensure(sum == 5, format!("Add returned {sum}"))
    })())
}

fn ufunction_return_by_reflection(context: &Context) -> Outcome {
    done((|| {
        let actor = TestActor::from_obj(spawn(context)?).map_err(|e| format!("{e:?}"))?;
        actor.set_count(9);
        let call = DynamicCall::new(&actor.as_ref(), "ReadCount").map_err(|e| format!("{e:?}"))?;
        let result = call.call().map_err(|e| format!("{e:?}"))?;
        let value: i32 = result.get("ReturnValue").map_err(|e| format!("{e:?}"))?;
        ensure(value == 9, format!("ReadCount returned {value}"))
    })())
}

fn ufunction_param_by_reflection(context: &Context) -> Outcome {
    done((|| {
        let actor = TestActor::from_obj(spawn(context)?).map_err(|e| format!("{e:?}"))?;
        let mut call = DynamicCall::new(&actor.as_ref(), "Store").map_err(|e| format!("{e:?}"))?;
        call.set("Value", 13i32).map_err(|e| format!("{e:?}"))?;
        call.call().map_err(|e| format!("{e:?}"))?;
        ensure(
            actor.count() == 13,
            format!("Store left count at {}", actor.count()),
        )
    })())
}

fn ufunction_echo_by_reflection(context: &Context) -> Outcome {
    done((|| {
        let actor = spawn(context)?;
        let mut call = DynamicCall::new(&actor, "Echo").map_err(|e| format!("{e:?}"))?;
        call.set("Value", 21i32).map_err(|e| format!("{e:?}"))?;
        let result = call.call().map_err(|e| format!("{e:?}"))?;
        let value: i32 = result.get("ReturnValue").map_err(|e| format!("{e:?}"))?;
        ensure(value == 21, format!("Echo returned {value}"))
    })())
}

fn component_is_created(context: &Context) -> Outcome {
    done((|| {
        let actor = TestActor::from_obj(spawn(context)?).map_err(|e| format!("{e:?}"))?;
        let root = actor.root().map_err(|e| format!("{e:?}"))?;
        ensure(root.checked().is_ok(), "no root component")
    })())
}

fn udelegate_reaches_a_closure(context: &Context) -> Outcome {
    done((|| {
        let actor = TestActor::from_obj(spawn(context)?).map_err(|e| format!("{e:?}"))?;
        let seen = Arc::new(Mutex::new(0));
        let sink = seen.clone();

        actor
            .on_ping_add(move |value| *sink.lock().unwrap() = value)
            .map_err(|e| format!("{e:?}"))?
            .detach();

        actor.ping(7);
        let value = *seen.lock().unwrap();
        ensure(value == 7, format!("the listener saw {value}"))
    })())
}

fn udelegate_listener_survives_garbage_collection(context: &Context) -> Outcome {
    let bound = (|| {
        let actor = spawn(context)?;
        let seen = Arc::new(Mutex::new(0));
        let sink = seen.clone();

        TestActor::from_obj(actor)
            .map_err(|e| format!("{e:?}"))?
            .on_ping_add(move |value| *sink.lock().unwrap() = value)
            .map_err(|e| format!("{e:?}"))?
            .detach();

        Ok::<_, String>((actor, seen))
    })();

    let (actor, seen) = match bound {
        Ok(bound) => bound,
        Err(e) => return done(Err(e)),
    };

    KismetSystemLibrary::collect_garbage();
    let mut frames = 0;

    Outcome::Pending(Box::new(move || {
        frames += 1;

        if frames < 5 {
            return None;
        }

        let fired = TestActor::from_obj(actor).map(|actor| actor.ping(9));

        if let Err(e) = fired {
            return Some(Err(format!("{e:?}")));
        }

        let value = *seen.lock().unwrap();
        Some(ensure(
            value == 9,
            format!("after a garbage collection the listener saw {value}"),
        ))
    }))
}

fn engine_function_call(_: &Context) -> Outcome {
    let sum = KismetMathLibrary::add_int_int(40, Some(2));
    done(ensure(sum == 42, format!("Add_IntInt returned {sum}")))
}

fn engine_string_is_utf8(_: &Context) -> Outcome {
    let joined = KismetStringLibrary::concat_str_str("ação", "ñü");
    done(ensure(joined == "açãoñü", format!("got {joined:?}")))
}

fn engine_string_longer_than_512_bytes(_: &Context) -> Outcome {
    let a = "ação-".repeat(120);
    let b = "ñü-".repeat(150);
    let joined = KismetStringLibrary::concat_str_str(&a, &b);

    done(ensure(
        joined == format!("{a}{b}"),
        format!("got {} bytes of {}", joined.len(), a.len() + b.len()),
    ))
}

fn rust_string_property_longer_than_512_bytes(context: &Context) -> Outcome {
    done((|| {
        let actor = TestActor::from_obj(spawn(context)?).map_err(|e| format!("{e:?}"))?;
        let text = "çãõ-".repeat(300);
        actor.set_label(&text);
        ensure(
            actor.label() == text,
            format!("got {} bytes", actor.label().len()),
        )
    })())
}

const DEFAULT_MATERIAL: &str = "/Engine/EngineMaterials/DefaultMaterial.DefaultMaterial";

fn default_material() -> Result<UObjectRef<Material>, String> {
    SoftObjectRef::<Material>::new(DEFAULT_MATERIAL)
        .load_synchronous()
        .map_err(|e| format!("load: {e:?}"))
}

fn add_component<T: UeClass + Inherits<ActorComponent>>(
    context: &Context,
) -> Result<UObjectRef<T>, String> {
    let actor = spawn(context)?
        .checked()
        .map_err(|e| format!("checked: {e:?}"))?;
    let transform = FTransform::from_transform(Transform::IDENTITY);

    actor
        .add_component_by_class(
            SubclassOf::<T>::base().upcast_to::<ActorComponent>(),
            false,
            &transform,
            false,
        )
        .cast::<T>()
        .map_err(|e| format!("add component: {e:?}"))
}

fn engine_soft_property_keeps_the_path(context: &Context) -> Outcome {
    done((|| {
        let material = default_material()?;
        let cloud = add_component::<VolumetricCloudComponent>(context)?
            .checked()
            .map_err(|e| format!("checked: {e:?}"))?;

        cloud.set_material(material.upcast());

        let soft = cloud.get_material();
        ensure(
            soft.path() == DEFAULT_MATERIAL,
            format!("path {:?}", soft.path()),
        )?;
        ensure(
            soft.get().raw() == material.raw(),
            "the soft reference does not resolve to the material",
        )
    })())
}

fn engine_soft_reference_crosses_as_a_path(_: &Context) -> Outcome {
    done((|| {
        let material = default_material()?;
        let loaded =
            KismetSystemLibrary::load_asset_blocking(&SoftObjectRef::new(DEFAULT_MATERIAL));
        ensure(
            loaded.raw() == material.raw(),
            "LoadAsset_Blocking did not load the path",
        )?;

        let soft =
            KismetSystemLibrary::conv_object_to_soft_object_reference(material.upcast().upcast());
        ensure(
            soft.path() == DEFAULT_MATERIAL,
            format!("path {:?}", soft.path()),
        )
    })())
}

fn engine_weak_property_follows_its_object(context: &Context) -> Outcome {
    done((|| {
        let movement = add_component::<ProjectileMovementComponent>(context)?
            .checked()
            .map_err(|e| format!("checked: {e:?}"))?;
        let target = spawn(context)?;
        let root = TestActor::from_obj(target)
            .map_err(|e| format!("{e:?}"))?
            .root()
            .map_err(|e| format!("{e:?}"))?;

        movement.set_homing_target_component(TWeakObjectPtr::from_ref(&root));

        let followed = movement.get_homing_target_component().get();
        ensure(
            followed.map(|c| c.raw()) == Some(root.raw()),
            "the weak reference does not reach the component",
        )?;

        target
            .checked()
            .map_err(|e| format!("checked: {e:?}"))?
            .k2_destroy_actor();

        ensure(
            movement.get_homing_target_component().get().is_none(),
            "the weak reference outlived the destroyed component",
        )
    })())
}

fn engine_struct_with_an_array_returned_by_a_function(context: &Context) -> Outcome {
    done((|| {
        let material = default_material()?;
        let camera = add_component::<CameraComponent>(context)?
            .checked()
            .map_err(|e| format!("checked: {e:?}"))?;
        let blendable = material
            .cast::<BlendableInterface>()
            .map_err(|e| format!("cast: {e:?}"))?;

        camera.add_or_update_blendable(blendable, Some(0.25));

        let mut kept = Vec::new();

        for _ in 0..200 {
            let view = camera.get_camera_view(0.0);
            kept.push(view.clone());
        }

        for view in &kept {
            let settings = view.as_ref().get_post_process_settings();
            let blendables = settings.as_ref().get_weighted_blendables();
            let array = blendables.as_ref().array();
            let len = array.len().map_err(|e| format!("len: {e:?}"))?;
            ensure(len == 1, format!("{len} blendables"))?;
            let first = array.get(0).map_err(|e| format!("get: {e:?}"))?;
            ensure(
                first.as_ref().get_object().raw() == material.raw()
                    && first.as_ref().get_weight() == 0.25,
                "the returned view lost its blendable",
            )?;
        }

        Ok(())
    })())
}

fn engine_struct_passed_by_reference(_: &Context) -> Outcome {
    done((|| {
        let mut guid = KismetGuidLibrary::new_guid();
        ensure(
            KismetGuidLibrary::is_valid_guid(&guid),
            "a new guid is not valid",
        )?;

        KismetGuidLibrary::invalidate_guid(&mut guid);
        ensure(
            !KismetGuidLibrary::is_valid_guid(&guid),
            "invalidate_guid did not change the guid",
        )
    })())
}

const STATIC_MESH_ACTOR: &str = "/Script/Engine.StaticMeshActor";

fn engine_soft_class_reference_crosses_as_a_path(_: &Context) -> Outcome {
    done((|| {
        let loaded =
            KismetSystemLibrary::load_class_asset_blocking(&SoftClassRef::new(STATIC_MESH_ACTOR));
        ensure(
            loaded.raw() == StaticMeshActor::static_class(),
            "LoadClassAsset_Blocking did not load the class",
        )?;

        let typed = SoftClassRef::<Actor>::new(STATIC_MESH_ACTOR)
            .load_synchronous()
            .map_err(|e| format!("load: {e:?}"))?;
        ensure(
            typed.raw() == StaticMeshActor::static_class(),
            "SoftClassRef::load_synchronous gave another class",
        )?;

        let soft = KismetSystemLibrary::conv_class_to_soft_class_reference(
            SubclassOf::<StaticMeshActor>::base().upcast_to::<bindings::core_ue::Object>(),
        );
        ensure(
            soft.path() == STATIC_MESH_ACTOR,
            format!("path {:?}", soft.path()),
        )
    })())
}

fn engine_soft_class_property_keeps_the_path(_: &Context) -> Outcome {
    done((|| {
        let input = InputSettings::get_input_settings()
            .checked()
            .map_err(|e| format!("input settings: {e:?}"))?;
        let class: SoftClassRef<PlayerInput> = input.get_default_player_input_class();
        ensure(
            !class.is_null(),
            "the default player input class has no path",
        )?;

        let loaded = class
            .load_synchronous()
            .map_err(|e| format!("load {:?}: {e:?}", class.path()))?;
        ensure(
            !loaded.is_null(),
            format!("{:?} does not resolve to a class", class.path()),
        )
    })())
}

fn engine_text_round_trip(_: &Context) -> Outcome {
    done((|| {
        let text = KismetTextLibrary::conv_string_to_text("ação");
        ensure(text.to_string() == "ação", format!("got {text}"))?;

        let upper = KismetTextLibrary::text_to_upper(&text);
        ensure(upper.to_string() == "AÇÃO", format!("got {upper}"))?;
        ensure(
            KismetTextLibrary::conv_text_to_string("ñü") == "ñü",
            "Conv_TextToString lost the text",
        )
    })())
}

fn engine_text_keeps_its_localization(context: &Context) -> Outcome {
    done((|| {
        let text = Text::localized("RustealTests", "Greeting", "Hello");
        let (found, namespace, key) = KismetTextLibrary::get_text_id(&text);
        ensure(
            found && namespace == "RustealTests" && key == "Greeting",
            format!("id {found} {namespace:?} {key:?}"),
        )?;

        let label = add_component::<TextRenderComponent>(context)?
            .checked()
            .map_err(|e| format!("checked: {e:?}"))?;
        label.set_text(text.clone());

        let read = label.get_text();
        ensure(read == text, format!("read {read}"))?;
        let (found, namespace, key) = KismetTextLibrary::get_text_id(read);
        ensure(
            found && namespace == "RustealTests" && key == "Greeting",
            format!("after the property, id {found} {namespace:?} {key:?}"),
        )
    })())
}

fn engine_structs_without_static_struct(_: &Context) -> Outcome {
    done((|| {
        let span = KismetMathLibrary::from_minutes(1.5);
        let seconds = KismetMathLibrary::get_total_seconds(&span);
        ensure(seconds == 90.0, format!("{seconds} seconds"))?;

        let a = OwnedStruct::<FIntPoint>::new();
        a.as_ref().set_x(2);
        a.as_ref().set_y(3);
        let b = OwnedStruct::<FIntPoint>::new();
        b.as_ref().set_x(10);
        b.as_ref().set_y(20);

        let sum = KismetMathLibrary::add_int_point_int_point(&a, &b);
        ensure(
            (sum.as_ref().get_x(), sum.as_ref().get_y()) == (12, 23),
            format!("sum ({}, {})", sum.as_ref().get_x(), sum.as_ref().get_y()),
        )
    })())
}

fn instanced_struct_holds_a_struct(_: &Context) -> Outcome {
    done((|| {
        let guid = KismetGuidLibrary::new_guid();
        let instanced = FInstancedStruct::make(&guid);
        let held = instanced.as_ref();
        ensure(held.is_set(), "the instanced struct is empty")?;
        ensure(
            held.script_struct() == FGuid::static_struct(),
            "the instanced struct holds another type",
        )?;

        let back = held.get::<FGuid>().ok_or("get::<FGuid> gave nothing")?;
        ensure(
            KismetGuidLibrary::equal_equal_guid_guid(&guid, &back),
            "the guid changed",
        )?;
        ensure(
            held.get::<FVector>().is_none(),
            "get::<FVector> gave a value",
        )?;

        let copy = instanced.clone();
        held.reset();
        ensure(!held.is_set(), "reset left a value")?;
        ensure(
            copy.as_ref().get::<FGuid>().is_some(),
            "the clone lost its value with the original",
        )
    })())
}

fn engine_optional_fields(_: &Context) -> Outcome {
    done((|| {
        let toolbar = OwnedStruct::<FToolBarStyle>::new();
        let toolbar = toolbar.as_ref();
        ensure(
            toolbar.get_vertical_alignment_override().is_none(),
            "a new toolbar style has an alignment",
        )?;
        toolbar.set_vertical_alignment_override(Some(EVerticalAlignment::VAlign_Center));
        ensure(
            toolbar.get_vertical_alignment_override() == Some(EVerticalAlignment::VAlign_Center),
            "the alignment was not kept",
        )?;
        toolbar.set_vertical_alignment_override(None);
        ensure(
            toolbar.get_vertical_alignment_override().is_none(),
            "the alignment was not cleared",
        )?;

        let wrap = OwnedStruct::<FWrapButtonStyle>::new();
        let wrap = wrap.as_ref();
        wrap.set_separator_thickness(Some(2.5));
        ensure(
            wrap.get_separator_thickness() == Some(2.5),
            format!("thickness {:?}", wrap.get_separator_thickness()),
        )?;

        let brush = OwnedStruct::<FSlateBrush>::new();
        wrap.set_separator_brush(Some(&brush));
        ensure(
            wrap.get_separator_brush().is_some(),
            "the separator brush was not kept",
        )
    })())
}

fn engine_function_takes_an_interface(context: &Context) -> Outcome {
    done((|| {
        let material = default_material()?;
        let blendable = material
            .cast::<BlendableInterface>()
            .map_err(|e| format!("cast: {e:?}"))?;
        let transform = FTransform::from_transform(Transform::IDENTITY);
        let volume = context
            .world
            .spawn_actor::<PostProcessVolume>(&transform)
            .map_err(|e| format!("spawn: {e:?}"))?
            .checked()
            .map_err(|e| format!("checked: {e:?}"))?;

        volume.add_or_update_blendable(blendable, Some(0.5));

        let settings = volume.get_settings();
        let blendables = settings.as_ref().get_weighted_blendables();
        let array = blendables.as_ref().array();
        let len = array.len().map_err(|e| format!("len: {e:?}"))?;
        ensure(len == 1, format!("{len} blendables"))?;
        let first = array.get(0).map_err(|e| format!("get: {e:?}"))?;
        let object = first.as_ref().get_object();
        ensure(
            object.raw() == material.raw(),
            "the blendable is not the material",
        )?;
        ensure(
            first.as_ref().get_weight() == 0.5,
            format!("weight {}", first.as_ref().get_weight()),
        )
    })())
}

fn engine_function_takes_any_object_as_iinterface(context: &Context) -> Outcome {
    done((|| {
        let actor = spawn(context)?;
        let object = KismetSystemLibrary::conv_interface_to_object(actor.upcast().upcast());
        ensure(
            object.raw() == actor.raw(),
            "Conv_InterfaceToObject returned another object",
        )
    })())
}

fn engine_function_takes_a_closure(_: &Context) -> Outcome {
    let fired = Arc::new(Mutex::new(0));
    let sink = fired.clone();

    let _handle = KismetSystemLibrary::k2_set_timer_delegate(
        move || *sink.lock().unwrap() += 1,
        0.05,
        false,
        None,
        None,
        None,
    );

    KismetSystemLibrary::collect_garbage();

    Outcome::Pending(Box::new(move || {
        let count = *fired.lock().unwrap();
        (count > 0).then(|| ensure(count == 1, format!("the timer fired {count} times")))
    }))
}

fn engine_member_function_takes_a_closure_with_arguments(context: &Context) -> Outcome {
    let started = (|| {
        let timeline = add_component::<TimelineComponent>(context)?
            .checked()
            .map_err(|e| format!("checked: {e:?}"))?;
        let curve = rusteal_runtime::runtime::world::new_object_raw(
            timeline.raw(),
            CurveFloat::static_class(),
        )
        .and_then(UObjectRef::<CurveFloat>::from_handle)
        .map_err(|e| format!("curve: {e:?}"))?;
        let checked = curve.checked().map_err(|e| format!("checked: {e:?}"))?;

        let rich = checked.get_float_curve();
        let keys = rich.as_ref().keys();

        for (time, value) in [(0.0, 0.0), (0.1, 1.0)] {
            let key = OwnedStruct::<FRichCurveKey>::new();
            key.as_ref().set_time(time);
            key.as_ref().set_value(value);
            keys.push(&key).map_err(|e| format!("key: {e:?}"))?;
        }

        checked.set_float_curve(&rich);

        let seen = Arc::new(Mutex::new(-1.0f32));
        let sink = seen.clone();
        timeline.add_interp_float(
            curve,
            move |value| *sink.lock().unwrap() = value,
            None,
            None,
        );
        timeline.play();

        Ok::<_, String>(seen)
    })();

    let seen = match started {
        Ok(seen) => seen,
        Err(e) => return done(Err(e)),
    };

    Outcome::Pending(Box::new(move || {
        let value = *seen.lock().unwrap();
        (value >= 0.999).then_some(Ok(()))
    }))
}

fn task_comes_back_to_the_game_thread(_: &Context) -> Outcome {
    let result = Arc::new(Mutex::new(None));
    let sink = result.clone();

    task::spawn(|| 6 * 7, move |value| *sink.lock().unwrap() = Some(value));

    Outcome::Pending(Box::new(move || {
        result
            .lock()
            .unwrap()
            .map(|value| ensure(value == 42, format!("the task gave {value}")))
    }))
}
