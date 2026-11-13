use bindings::engine::{
    ActorComponent, ActorComponentExt, CameraComponent, EDrawDebugTrace, EObjectTypeQuery,
    FHitResultExt, GameplayStatics, KismetSystemLibrary, Pawn, PawnExt, PrimitiveComponentExt,
    SceneComponent, SceneComponentExt, SkeletalMeshComponentExt, StaticMeshComponent,
};
use bindings::enhanced_input::{
    ETriggerEvent, EnhancedInputLocalPlayerSubsystemExt, FInputActionValue, InputAction,
    InputMappingContext,
};
use bindings::head_mounted_display::{
    EHMDTrackingOrigin, MotionControllerComponent, MotionControllerComponentExt,
};
use bindings::navigation::NavigationSystemV1;
use bindings::niagara::{NiagaraComponent, NiagaraDataInterfaceArrayFunctionLibrary};
use bindings::prelude::*;
use bindings::umg::{WidgetInteractionComponent, WidgetInteractionComponentExt};
use bindings::xr_base::{
    HeadMountedDisplayFunctionLibrary, VRNotificationsComponent, XRDeviceVisualizationComponent,
    XRLoadingScreenFunctionLibrary,
};
use glam::DVec3;
use rusteal_runtime::runtime::{
    FName, OwnedStruct, Rotator, RustealResult, SubclassOf, UObjectRef, UStructRef, method,
};
use rusteal_runtime::{uclass, uclass_impl};

use crate::grab_component::GrabComponent;
use crate::hand::{HandAnimInstance, MannequinsXR};
use crate::menu::Menu;
use crate::model;
use crate::teleport_visualizer::TeleportVisualizer;

#[uclass(parent = Pawn, implements = ["/Game/XRFramework/Blueprints/BPI_PawnAnim.BPI_PawnAnim_C"])]
pub struct XRPawn {
    #[component(root, name = "VROrigin")]
    vr_origin: SceneComponent,

    #[component(attach = "vr_origin")]
    camera: CameraComponent,

    #[component(attach = "camera")]
    head_mounted_display_mesh: StaticMeshComponent,

    #[component(attach = "vr_origin")]
    motion_controller_left_grip: MotionControllerComponent,

    #[component(attach = "motion_controller_left_grip")]
    hand_left: MannequinsXR,

    #[component(
        attach = "motion_controller_left_grip",
        name = "XRDeviceVisualizationLeft"
    )]
    xr_device_visualization_left: XRDeviceVisualizationComponent,

    #[component(attach = "vr_origin")]
    motion_controller_right_grip: MotionControllerComponent,

    #[component(attach = "motion_controller_right_grip")]
    hand_right: MannequinsXR,

    #[component(
        attach = "motion_controller_right_grip",
        name = "XRDeviceVisualizationRight"
    )]
    xr_device_visualization_right: XRDeviceVisualizationComponent,

    #[component(attach = "vr_origin")]
    teleport_trace_niagara_system: NiagaraComponent,

    #[component(attach = "vr_origin")]
    motion_controller_left_aim: MotionControllerComponent,

    #[component(attach = "motion_controller_left_aim")]
    widget_interaction_left: WidgetInteractionComponent,

    #[component(attach = "vr_origin")]
    motion_controller_right_aim: MotionControllerComponent,

    #[component(attach = "motion_controller_right_aim")]
    widget_interaction_right: WidgetInteractionComponent,

    #[component(name = "VRNotifications")]
    vr_notifications: VRNotificationsComponent,

    #[uproperty(EditAnywhere, BlueprintReadWrite, category = "Grab", default = 6.0)]
    grab_radius_from_grip_position: f32,

    #[uproperty(EditAnywhere, BlueprintReadWrite, category = "Turn", default = -45.0)]
    snap_turn_degrees: f32,

    #[uproperty(EditAnywhere, BlueprintReadWrite, category = "Teleport")]
    teleport_project_point_to_navigation_query_extent: OwnedStruct<FVector>,

    #[uproperty(EditAnywhere, category = "Teleport")]
    teleport_visualizer_class: SubclassOf<TeleportVisualizer>,

    #[uproperty(EditAnywhere, category = "Menu")]
    menu_class: SubclassOf<Menu>,

    #[uproperty(EditAnywhere, category = "Input")]
    default_mapping_context: UObjectRef<InputMappingContext>,

    #[uproperty(EditAnywhere, category = "Input")]
    hands_mapping_context: UObjectRef<InputMappingContext>,

    #[uproperty(EditAnywhere, category = "Input")]
    move_action: UObjectRef<InputAction>,

    #[uproperty(EditAnywhere, category = "Input")]
    turn_action: UObjectRef<InputAction>,

    #[uproperty(EditAnywhere, category = "Input")]
    grab_left_pressed_action: UObjectRef<InputAction>,

    #[uproperty(EditAnywhere, category = "Input")]
    grab_left_released_action: UObjectRef<InputAction>,

    #[uproperty(EditAnywhere, category = "Input")]
    grab_right_pressed_action: UObjectRef<InputAction>,

    #[uproperty(EditAnywhere, category = "Input")]
    grab_right_released_action: UObjectRef<InputAction>,

    #[uproperty(EditAnywhere, category = "Input")]
    menu_toggle_left_action: UObjectRef<InputAction>,

    #[uproperty(EditAnywhere, category = "Input")]
    menu_toggle_right_action: UObjectRef<InputAction>,

    #[uproperty(EditAnywhere, category = "Input|Hands")]
    hand_grasp_left_action: UObjectRef<InputAction>,

    #[uproperty(EditAnywhere, category = "Input|Hands")]
    hand_grasp_right_action: UObjectRef<InputAction>,

    #[uproperty(EditAnywhere, category = "Input|Hands")]
    hand_index_curl_left_action: UObjectRef<InputAction>,

    #[uproperty(EditAnywhere, category = "Input|Hands")]
    hand_index_curl_right_action: UObjectRef<InputAction>,

    #[uproperty(EditAnywhere, category = "Input|Hands")]
    hand_point_left_action: UObjectRef<InputAction>,

    #[uproperty(EditAnywhere, category = "Input|Hands")]
    hand_point_right_action: UObjectRef<InputAction>,

    #[uproperty(EditAnywhere, category = "Input|Hands")]
    hand_thumb_up_left_action: UObjectRef<InputAction>,

    #[uproperty(EditAnywhere, category = "Input|Hands")]
    hand_thumb_up_right_action: UObjectRef<InputAction>,

    #[uproperty(BlueprintReadOnly, category = "Teleport")]
    projected_teleport_location: OwnedStruct<FVector>,

    #[uproperty(BlueprintReadOnly, category = "Teleport")]
    b_valid_teleport_location: bool,

    #[uproperty(BlueprintReadOnly, category = "Teleport")]
    b_teleport_trace_active: bool,

    #[uproperty(BlueprintReadOnly, category = "Teleport")]
    teleport_visualizer_reference: UObjectRef<TeleportVisualizer>,

    #[uproperty(BlueprintReadOnly, category = "Grab")]
    held_component_left: UObjectRef<GrabComponent>,

    #[uproperty(BlueprintReadOnly, category = "Grab")]
    held_component_right: UObjectRef<GrabComponent>,

    #[uproperty(BlueprintReadOnly, category = "Menu")]
    menu_reference: UObjectRef<Menu>,

    teleport_trace_path_positions: Vec<DVec3>,
}

#[uclass_impl]
impl XRPawn {
    #[class_defaults]
    fn class_defaults(&mut self) -> RustealResult<()> {
        let hmd = self.head_mounted_display_mesh()?.checked()?;
        hmd.set_owner_no_see(true);
        hmd.set_collision_profile_name(FName::new("NoCollision").handle(), Some(true));
        hmd.k2_set_relative_location(
            &FVector::from_dvec3(DVec3::new(8.0, 0.0, 0.0)),
            false,
            false,
        );
        hmd.set_relative_scale3_d(&FVector::from_dvec3(DVec3::splat(1.5)));

        for (controller, source) in [
            (self.motion_controller_left_grip()?, "LeftGrip"),
            (self.motion_controller_right_grip()?, "RightGrip"),
            (self.motion_controller_left_aim()?, "LeftAim"),
            (self.motion_controller_right_aim()?, "RightAim"),
        ] {
            controller
                .checked()?
                .set_tracking_motion_source(FName::new(source).handle());
        }

        let left = self.hand_left()?.checked()?;
        left.k2_set_relative_location_and_rotation(
            &FVector::from_dvec3(DVec3::new(-2.981260, -3.5, 4.561753)),
            &FRotator::from_rotator(Rotator::new(-25.0, -179.999999, 89.999998)),
            false,
            false,
        );

        let right = self.hand_right()?.checked()?;
        right.k2_set_relative_location_and_rotation(
            &FVector::from_dvec3(DVec3::new(-2.981260, 3.5, 4.561753)),
            &FRotator::from_rotator(Rotator::new(25.0, 0.0, 89.999999)),
            false,
            false,
        );

        MannequinsXR::from_obj(self.hand_left()?)?.set_b_mirror(true);

        self.teleport_trace_niagara_system()?
            .checked()?
            .set_visibility(false, Some(false));

        let channel = bindings::engine::ECollisionChannel::ECC_GameTraceChannel1;
        let left_interaction = self.widget_interaction_left()?.checked()?;
        left_interaction.set_pointer_index(0);
        left_interaction.set_trace_channel(channel);

        let right_interaction = self.widget_interaction_right()?.checked()?;
        right_interaction.set_pointer_index(1);
        right_interaction.set_trace_channel(channel);

        Ok(())
    }

    #[ufunction(Override)]
    fn receive_begin_play(&mut self) {
        if let Err(e) = self.begin_play() {
            log::warn!("[VR] begin play failed: {e}");
        }
    }

    #[ufunction(Override)]
    fn receive_restarted(&mut self) {
        if let Err(e) = self.bind_input() {
            log::warn!("[VR] failed to bind the input: {e}");
        }
    }

    #[ufunction]
    fn move_started(&mut self) {
        self.start_teleport_trace();
    }

    #[ufunction]
    fn move_triggered(&mut self) {
        let Ok(aim) = self
            .motion_controller_right_aim()
            .and_then(|aim| aim.checked())
        else {
            return;
        };

        let start = aim.k2_get_component_location().to_dvec3();
        let forward = aim.get_forward_vector().to_dvec3();
        self.teleport_trace(
            FVector::from_dvec3(start).as_ref(),
            FVector::from_dvec3(forward).as_ref(),
        );
    }

    #[ufunction]
    fn move_completed(&mut self) {
        self.end_teleport_trace();
        self.try_teleport();
    }

    #[ufunction]
    fn turn_triggered(&mut self, value: UStructRef<FInputActionValue>) {
        self.snap_turn(value.axis1d() > 0.0);
    }

    #[ufunction]
    fn grab_left_pressed(&mut self) {
        self.grab(model::Hand::Left);
    }

    #[ufunction]
    fn grab_right_pressed(&mut self) {
        self.grab(model::Hand::Right);
    }

    #[ufunction]
    fn grab_left_released(&mut self) {
        self.release(model::Hand::Left);
    }

    #[ufunction]
    fn grab_right_released(&mut self) {
        self.release(model::Hand::Right);
    }

    #[ufunction]
    fn menu_toggle_left(&mut self) {
        self.toggle_menu(false);
    }

    #[ufunction]
    fn menu_toggle_right(&mut self) {
        self.toggle_menu(true);
    }

    #[ufunction]
    fn hand_grasp_left(&mut self, value: UStructRef<FInputActionValue>) {
        self.set_hand_pose(
            model::Hand::Left,
            |hand, alpha| hand.set_pose_alpha_grasp(alpha),
            value.axis1d(),
        );
    }

    #[ufunction]
    fn hand_grasp_left_end(&mut self) {
        self.set_hand_pose(
            model::Hand::Left,
            |hand, alpha| hand.set_pose_alpha_grasp(alpha),
            0.0,
        );
    }

    #[ufunction]
    fn hand_grasp_right(&mut self, value: UStructRef<FInputActionValue>) {
        self.set_hand_pose(
            model::Hand::Right,
            |hand, alpha| hand.set_pose_alpha_grasp(alpha),
            value.axis1d(),
        );
    }

    #[ufunction]
    fn hand_grasp_right_end(&mut self) {
        self.set_hand_pose(
            model::Hand::Right,
            |hand, alpha| hand.set_pose_alpha_grasp(alpha),
            0.0,
        );
    }

    #[ufunction]
    fn hand_index_curl_left(&mut self, value: UStructRef<FInputActionValue>) {
        self.set_hand_pose(
            model::Hand::Left,
            |hand, alpha| hand.set_pose_alpha_index_curl(alpha),
            value.axis1d(),
        );
    }

    #[ufunction]
    fn hand_index_curl_left_end(&mut self) {
        self.set_hand_pose(
            model::Hand::Left,
            |hand, alpha| hand.set_pose_alpha_index_curl(alpha),
            0.0,
        );
    }

    #[ufunction]
    fn hand_index_curl_right(&mut self, value: UStructRef<FInputActionValue>) {
        self.set_hand_pose(
            model::Hand::Right,
            |hand, alpha| hand.set_pose_alpha_index_curl(alpha),
            value.axis1d(),
        );
    }

    #[ufunction]
    fn hand_index_curl_right_end(&mut self) {
        self.set_hand_pose(
            model::Hand::Right,
            |hand, alpha| hand.set_pose_alpha_index_curl(alpha),
            0.0,
        );
    }

    #[ufunction]
    fn hand_point_left_touched(&mut self) {
        self.set_hand_pose(
            model::Hand::Left,
            |hand, alpha| hand.set_pose_alpha_point(alpha as f32),
            0.0,
        );
    }

    #[ufunction]
    fn hand_point_left_untouched(&mut self) {
        self.set_hand_pose(
            model::Hand::Left,
            |hand, alpha| hand.set_pose_alpha_point(alpha as f32),
            1.0,
        );
    }

    #[ufunction]
    fn hand_point_right_touched(&mut self) {
        self.set_hand_pose(
            model::Hand::Right,
            |hand, alpha| hand.set_pose_alpha_point(alpha as f32),
            0.0,
        );
    }

    #[ufunction]
    fn hand_point_right_untouched(&mut self) {
        self.set_hand_pose(
            model::Hand::Right,
            |hand, alpha| hand.set_pose_alpha_point(alpha as f32),
            1.0,
        );
    }

    #[ufunction]
    fn hand_thumb_up_left_touched(&mut self) {
        self.set_hand_pose(
            model::Hand::Left,
            |hand, alpha| hand.set_pose_alpha_thumb_up(alpha as f32),
            0.0,
        );
    }

    #[ufunction]
    fn hand_thumb_up_left_untouched(&mut self) {
        self.set_hand_pose(
            model::Hand::Left,
            |hand, alpha| hand.set_pose_alpha_thumb_up(alpha as f32),
            1.0,
        );
    }

    #[ufunction]
    fn hand_thumb_up_right_touched(&mut self) {
        self.set_hand_pose(
            model::Hand::Right,
            |hand, alpha| hand.set_pose_alpha_thumb_up(alpha as f32),
            0.0,
        );
    }

    #[ufunction]
    fn hand_thumb_up_right_untouched(&mut self) {
        self.set_hand_pose(
            model::Hand::Right,
            |hand, alpha| hand.set_pose_alpha_thumb_up(alpha as f32),
            1.0,
        );
    }

    #[ufunction]
    pub fn hide_unhide_hand(&mut self, b_hide: bool, b_right_hand: bool) {
        let (held, other) = if b_right_hand {
            (self.hand_right(), self.hand_left())
        } else {
            (self.hand_left(), self.hand_right())
        };

        if let Ok(hand) = held.and_then(|hand| hand.checked()) {
            hand.set_visibility(!b_hide, Some(false));
        }

        if let Ok(hand) = other.and_then(|hand| hand.checked()) {
            hand.set_visibility(true, Some(false));
        }
    }

    #[ufunction(BlueprintCallable)]
    pub fn start_teleport_trace(&mut self) {
        self.set_b_teleport_trace_active(true);

        if let Ok(trace) = self
            .teleport_trace_niagara_system()
            .and_then(|trace| trace.checked())
        {
            trace.set_visibility(true, Some(false));
        }

        match self.spawn_teleport_visualizer() {
            Ok(visualizer) => self.set_teleport_visualizer_reference(visualizer),
            Err(e) => log::warn!("[VR] no teleport visualizer: {e}"),
        }
    }

    #[ufunction(BlueprintCallable)]
    pub fn teleport_trace(
        &mut self,
        start_pos: UStructRef<FVector>,
        forward_vector: UStructRef<FVector>,
    ) {
        if let Err(e) = self.trace_teleport(
            start_pos.to_owned().to_dvec3(),
            forward_vector.to_owned().to_dvec3(),
        ) {
            log::warn!("[VR] teleport trace failed: {e}");
        }
    }

    #[ufunction(BlueprintCallable)]
    pub fn end_teleport_trace(&mut self) {
        self.set_b_teleport_trace_active(false);

        if let Ok(visualizer) = self.teleport_visualizer_reference().checked() {
            visualizer.k2_destroy_actor();
        }

        if let Ok(trace) = self
            .teleport_trace_niagara_system()
            .and_then(|trace| trace.checked())
        {
            trace.set_visibility(false, Some(false));
        }
    }

    #[ufunction(BlueprintCallable)]
    pub fn try_teleport(&mut self) {
        if !self.b_valid_teleport_location() {
            return;
        }

        self.set_b_valid_teleport_location(false);

        let (Ok(me), Ok(camera)) = (
            self.as_ref().checked(),
            self.camera().and_then(|c| c.checked()),
        ) else {
            return;
        };

        let yaw = me.k2_get_actor_rotation().to_rotator().yaw;
        let destination = model::teleport_destination(
            self.projected_teleport_location().to_dvec3(),
            camera.get_relative_location().to_dvec3(),
            yaw,
        );

        me.k2_teleport_to(
            &FVector::from_dvec3(destination),
            &FRotator::from_rotator(Rotator::new(0.0, yaw, 0.0)),
        );
    }

    #[ufunction(BlueprintCallable)]
    pub fn snap_turn(&mut self, b_right_turn: bool) {
        if let Err(e) = self.turn(b_right_turn) {
            log::warn!("[VR] snap turn failed: {e}");
        }
    }

    #[ufunction(BlueprintCallable)]
    pub fn toggle_menu(&mut self, b_active_menu_hand_right: bool) {
        if let Ok(menu) = Menu::from_obj(self.menu_reference()) {
            menu.close_menu();
            return;
        }

        match Menu::spawn_for(
            self.as_ref().upcast_to(),
            self.menu_class(),
            b_active_menu_hand_right,
        ) {
            Ok(menu) => self.set_menu_reference(menu),
            Err(e) => log::warn!("[VR] could not open the menu: {e}"),
        }
    }

    #[ufunction(BlueprintPure)]
    pub fn get_grab_component_near_motion_controller(
        &self,
        motion_controller: UObjectRef<MotionControllerComponent>,
    ) -> UObjectRef<GrabComponent> {
        self.nearest_grab_component(motion_controller)
            .unwrap_or_default()
    }
}

impl XRPawn {
    fn begin_play(&mut self) -> RustealResult<()> {
        let me = self.as_ref();

        if HeadMountedDisplayFunctionLibrary::is_head_mounted_display_enabled() {
            HeadMountedDisplayFunctionLibrary::set_tracking_origin(EHMDTrackingOrigin::LocalFloor);
            KismetSystemLibrary::execute_console_command(
                me.upcast_to(),
                "xr.SecondaryScreenPercentage.HMDRenderTarget 100",
                None,
            );

            let controller = GameplayStatics::get_player_controller(me.upcast_to(), 0);
            let subsystem = enhanced_input_subsystem(controller)?.checked()?;
            let options = OwnedStruct::new();
            subsystem.add_mapping_context(self.default_mapping_context(), 0, &options);
            subsystem.add_mapping_context(self.hands_mapping_context(), 0, &options);
        }

        XRLoadingScreenFunctionLibrary::hide_loading_screen();
        Ok(())
    }

    fn bind_input(&self) -> RustealResult<()> {
        let me = self.as_ref();
        let pawn = me.checked()?;

        if !pawn.is_player_controlled() || !pawn.is_locally_controlled() {
            return Ok(());
        }

        let bindings_with_value = [
            (
                self.turn_action(),
                ETriggerEvent::Triggered,
                method(self, Self::turn_triggered),
            ),
            (
                self.hand_grasp_left_action(),
                ETriggerEvent::Triggered,
                method(self, Self::hand_grasp_left),
            ),
            (
                self.hand_grasp_right_action(),
                ETriggerEvent::Triggered,
                method(self, Self::hand_grasp_right),
            ),
            (
                self.hand_index_curl_left_action(),
                ETriggerEvent::Triggered,
                method(self, Self::hand_index_curl_left),
            ),
            (
                self.hand_index_curl_right_action(),
                ETriggerEvent::Triggered,
                method(self, Self::hand_index_curl_right),
            ),
        ];

        for (action, event, callback) in bindings_with_value {
            if action.is_valid() {
                bind_action(&me, action, event, callback)?;
            }
        }

        let bindings = [
            (
                self.move_action(),
                ETriggerEvent::Started,
                method(self, Self::move_started),
            ),
            (
                self.move_action(),
                ETriggerEvent::Triggered,
                method(self, Self::move_triggered),
            ),
            (
                self.move_action(),
                ETriggerEvent::Completed,
                method(self, Self::move_completed),
            ),
            (
                self.grab_left_pressed_action(),
                ETriggerEvent::Triggered,
                method(self, Self::grab_left_pressed),
            ),
            (
                self.grab_right_pressed_action(),
                ETriggerEvent::Triggered,
                method(self, Self::grab_right_pressed),
            ),
            (
                self.grab_left_released_action(),
                ETriggerEvent::Triggered,
                method(self, Self::grab_left_released),
            ),
            (
                self.grab_right_released_action(),
                ETriggerEvent::Triggered,
                method(self, Self::grab_right_released),
            ),
            (
                self.menu_toggle_left_action(),
                ETriggerEvent::Triggered,
                method(self, Self::menu_toggle_left),
            ),
            (
                self.menu_toggle_right_action(),
                ETriggerEvent::Triggered,
                method(self, Self::menu_toggle_right),
            ),
            (
                self.hand_grasp_left_action(),
                ETriggerEvent::Canceled,
                method(self, Self::hand_grasp_left_end),
            ),
            (
                self.hand_grasp_left_action(),
                ETriggerEvent::Completed,
                method(self, Self::hand_grasp_left_end),
            ),
            (
                self.hand_grasp_right_action(),
                ETriggerEvent::Canceled,
                method(self, Self::hand_grasp_right_end),
            ),
            (
                self.hand_grasp_right_action(),
                ETriggerEvent::Completed,
                method(self, Self::hand_grasp_right_end),
            ),
            (
                self.hand_index_curl_left_action(),
                ETriggerEvent::Canceled,
                method(self, Self::hand_index_curl_left_end),
            ),
            (
                self.hand_index_curl_left_action(),
                ETriggerEvent::Completed,
                method(self, Self::hand_index_curl_left_end),
            ),
            (
                self.hand_index_curl_right_action(),
                ETriggerEvent::Canceled,
                method(self, Self::hand_index_curl_right_end),
            ),
            (
                self.hand_index_curl_right_action(),
                ETriggerEvent::Completed,
                method(self, Self::hand_index_curl_right_end),
            ),
            (
                self.hand_point_left_action(),
                ETriggerEvent::Started,
                method(self, Self::hand_point_left_touched),
            ),
            (
                self.hand_point_left_action(),
                ETriggerEvent::Canceled,
                method(self, Self::hand_point_left_touched),
            ),
            (
                self.hand_point_left_action(),
                ETriggerEvent::Completed,
                method(self, Self::hand_point_left_untouched),
            ),
            (
                self.hand_point_right_action(),
                ETriggerEvent::Started,
                method(self, Self::hand_point_right_touched),
            ),
            (
                self.hand_point_right_action(),
                ETriggerEvent::Canceled,
                method(self, Self::hand_point_right_touched),
            ),
            (
                self.hand_point_right_action(),
                ETriggerEvent::Completed,
                method(self, Self::hand_point_right_untouched),
            ),
            (
                self.hand_thumb_up_left_action(),
                ETriggerEvent::Started,
                method(self, Self::hand_thumb_up_left_touched),
            ),
            (
                self.hand_thumb_up_left_action(),
                ETriggerEvent::Canceled,
                method(self, Self::hand_thumb_up_left_touched),
            ),
            (
                self.hand_thumb_up_left_action(),
                ETriggerEvent::Completed,
                method(self, Self::hand_thumb_up_left_untouched),
            ),
            (
                self.hand_thumb_up_right_action(),
                ETriggerEvent::Started,
                method(self, Self::hand_thumb_up_right_touched),
            ),
            (
                self.hand_thumb_up_right_action(),
                ETriggerEvent::Canceled,
                method(self, Self::hand_thumb_up_right_touched),
            ),
            (
                self.hand_thumb_up_right_action(),
                ETriggerEvent::Completed,
                method(self, Self::hand_thumb_up_right_untouched),
            ),
        ];

        for (action, event, mut callback) in bindings {
            if action.is_valid() {
                bind_action(&me, action, event, move |_| callback())?;
            }
        }

        Ok(())
    }

    fn spawn_teleport_visualizer(&self) -> RustealResult<UObjectRef<TeleportVisualizer>> {
        let me = self.as_ref();
        let world = me.get_world()?;
        let transform = FTransform::from_transform(rusteal_runtime::runtime::Transform::new(
            glam::DQuat::IDENTITY,
            DVec3::ZERO,
            DVec3::ONE,
        ));

        let visualizer =
            world.spawn_actor_of_class(self.teleport_visualizer_class(), &transform)?;
        visualizer.checked()?.set_owner(me.upcast_to());
        Ok(visualizer)
    }

    fn trace_teleport(&mut self, start: DVec3, forward: DVec3) -> RustealResult<()> {
        let me = self.as_ref();
        let launch_velocity = forward * model::TELEPORT_LAUNCH_SPEED;

        let (_, hit, path, _) = GameplayStatics::blueprint_predict_projectile_path_by_object_type(
            me.upcast_to(),
            &FVector::from_dvec3(start),
            &FVector::from_dvec3(launch_velocity),
            true,
            model::TELEPORT_PROJECTILE_RADIUS,
            &[EObjectTypeQuery::ObjectTypeQuery1],
            false,
            &[],
            EDrawDebugTrace::None,
            0.0,
            Some(model::TELEPORT_SIM_FREQUENCY),
            Some(model::TELEPORT_MAX_SIM_TIME),
            Some(0.0),
        );

        let mut positions = Vec::with_capacity(path.len() + 1);
        positions.push(FVector::from_dvec3(start));
        positions.extend(path);

        let (valid, nav_point) = NavigationSystemV1::k2_project_point_to_navigation(
            me.upcast_to(),
            &FVector::from_dvec3(hit.as_ref().get_location().to_dvec3()),
            UObjectRef::null(),
            SubclassOf::null(),
            &self.teleport_project_point_to_navigation_query_extent(),
        );

        self.set_projected_teleport_location(&FVector::from_dvec3(
            model::projected_teleport_location(nav_point.to_dvec3()),
        ));

        if self.b_valid_teleport_location() != valid {
            self.set_b_valid_teleport_location(valid);

            if let Ok(root) = self
                .teleport_visualizer_reference()
                .checked()
                .and_then(|v| v.k2_get_root_component().checked())
            {
                root.set_visibility(valid, Some(true));
            }
        }

        if let Ok(visualizer) = self.teleport_visualizer_reference().checked() {
            visualizer.k2_set_actor_location(&self.projected_teleport_location(), false, false);
        }

        NiagaraDataInterfaceArrayFunctionLibrary::set_niagara_array_vector(
            self.teleport_trace_niagara_system()?,
            FName::new("User.PointArray").handle(),
            &positions,
        );

        self.set_teleport_trace_path_positions(positions.iter().map(|p| p.to_dvec3()).collect());

        Ok(())
    }

    fn turn(&self, right_turn: bool) -> RustealResult<()> {
        let me = self.as_ref().checked()?;
        let camera = self.camera()?.checked()?;

        let delta = model::snap_turn_delta(f64::from(self.snap_turn_degrees()), right_turn);
        let camera_world = camera.k2_get_component_location().to_dvec3();
        let camera_relative = camera.get_relative_location().to_dvec3();
        let new_yaw = me.k2_get_actor_rotation().to_rotator().yaw + delta;

        me.k2_add_actor_world_rotation(
            &FRotator::from_rotator(Rotator::new(0.0, delta, 0.0)),
            false,
            true,
        );

        me.k2_set_actor_location(
            &FVector::from_dvec3(model::snap_turn_location(
                camera_world,
                camera_relative,
                new_yaw,
            )),
            false,
            true,
        );

        Ok(())
    }

    fn controllers(
        &self,
        hand: model::Hand,
    ) -> RustealResult<UObjectRef<MotionControllerComponent>> {
        match hand {
            model::Hand::Left => self.motion_controller_left_grip(),
            model::Hand::Right => self.motion_controller_right_grip(),
        }
    }

    fn nearest_grab_component(
        &self,
        motion_controller: UObjectRef<MotionControllerComponent>,
    ) -> RustealResult<UObjectRef<GrabComponent>> {
        let me = self.as_ref();
        let grip = motion_controller
            .checked()?
            .k2_get_component_location()
            .to_dvec3();
        let point = FVector::from_dvec3(grip);
        let white = FLinearColor::from_linear_color(rusteal_runtime::runtime::LinearColor::new(
            0.0, 1.0, 1.0, 1.0,
        ));
        let green = FLinearColor::from_linear_color(rusteal_runtime::runtime::LinearColor::new(
            0.0, 1.0, 0.0, 1.0,
        ));

        let (hit, result) = KismetSystemLibrary::sphere_trace_single_for_objects(
            me.upcast_to(),
            &point,
            &point,
            self.grab_radius_from_grip_position(),
            &[EObjectTypeQuery::ObjectTypeQuery4],
            false,
            &[],
            EDrawDebugTrace::None,
            true,
            &white,
            &green,
            Some(1.0),
        );

        if !hit {
            return Ok(UObjectRef::null());
        }

        let actor = result.as_ref().get_component().checked()?.get_owner();

        let candidates: Vec<(UObjectRef<GrabComponent>, DVec3)> = actor
            .checked()?
            .k2_get_components_by_class(
                SubclassOf::<GrabComponent>::base().upcast_to::<ActorComponent>(),
            )
            .into_iter()
            .filter_map(|component| {
                let grab = component.cast::<GrabComponent>().ok()?;
                let location = grab
                    .upcast_to::<SceneComponent>()
                    .checked()
                    .ok()?
                    .k2_get_component_location()
                    .to_dvec3();
                Some((grab, location))
            })
            .collect();

        Ok(model::nearest(grip, &candidates).unwrap_or_default())
    }

    fn grab(&mut self, hand: model::Hand) {
        let Ok(controller) = self.controllers(hand) else {
            return;
        };

        let nearest = self.get_grab_component_near_motion_controller(controller);

        let Ok(mut grab) = GrabComponent::from_obj(nearest) else {
            return;
        };

        if !grab.try_grab(controller) {
            return;
        }

        match hand {
            model::Hand::Left => {
                self.set_held_component_left(nearest);

                if self.held_component_left() == self.held_component_right() {
                    self.set_held_component_right(UObjectRef::null());
                }
            }
            model::Hand::Right => {
                self.set_held_component_right(nearest);

                if self.held_component_right() == self.held_component_left() {
                    self.set_held_component_left(UObjectRef::null());
                }
            }
        }
    }

    fn release(&mut self, hand: model::Hand) {
        let held = match hand {
            model::Hand::Left => self.held_component_left(),
            model::Hand::Right => self.held_component_right(),
        };

        let Ok(mut grab) = GrabComponent::from_obj(held) else {
            return;
        };

        if grab.try_release() {
            match hand {
                model::Hand::Left => self.set_held_component_left(UObjectRef::null()),
                model::Hand::Right => self.set_held_component_right(UObjectRef::null()),
            }
        }
    }

    fn set_hand_pose(
        &self,
        hand: model::Hand,
        set: impl Fn(&mut HandAnimInstance, f64),
        alpha: f64,
    ) {
        let mesh = match hand {
            model::Hand::Left => self.hand_left(),
            model::Hand::Right => self.hand_right(),
        };

        let anim = mesh
            .and_then(|mesh| {
                mesh.upcast_to::<bindings::engine::SkeletalMeshComponent>()
                    .checked()
            })
            .map(|mesh| mesh.get_anim_instance());

        if let Ok(anim) = anim
            && let Ok(mut hand_anim) = HandAnimInstance::from_obj(anim)
        {
            set(&mut hand_anim, alpha);
        }
    }
}
