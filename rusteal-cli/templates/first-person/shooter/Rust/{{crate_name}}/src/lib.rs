// The First Person template's and its Shooter variant's gameplay classes in Rust.
//
// UE's First Person C++ template ships `ATP_FirstPersonCharacter`,
// `ATP_FirstPersonCameraManager`, `ATP_FirstPersonGameMode` and
// `ATP_FirstPersonPlayerController`; here they are `FirstPersonCharacter`,
// `FirstPersonCameraManager`, `FirstPersonGameMode` and
// `FirstPersonPlayerController`. As in the C++ template, a Blueprint child of
// the character, the game mode and the controller holds the assets and the
// classes, in `Content/FirstPerson/Blueprints/`:
// - `BP_FirstPersonCharacter`: the arms and the mannequin (skeletal meshes,
//   anim classes) and the input actions (IA_Jump, IA_Move, IA_Look,
//   IA_MouseLook);
// - `BP_FirstPersonPlayerController`: the mapping contexts (Default =
//   IMC_Default, Mobile Excluded = IMC_MouseLook) and the touch controls;
// - `BP_FirstPersonGameMode`: Default Pawn Class = BP_FirstPersonCharacter,
//   Player Controller Class = BP_FirstPersonPlayerController. It is the
//   project's default game mode (Config/DefaultEngine.ini).
// The controller makes `FirstPersonCameraManager`, which limits the look
// pitch, its camera manager.
//
// The Shooter variant (`shooter/`) adds `ShooterCharacter`, a
// `FirstPersonCharacter` that picks up and switches weapons and has health,
// `ShooterWeapon`s that shoot `ShooterProjectile`s, `ShooterPickup`s whose
// weapon and mesh are rows of a data table (`WeaponTableRow`, a struct
// declared in Rust), and `ShooterNPC`s, run by `ShooterAIController`'s
// StateTree with Rust tasks and conditions, AI perception and an EnvQuery
// context, spawned by `ShooterNPCSpawner`s. `ShooterGameMode` keeps the team
// scores on `ShooterUI`, `ShooterPlayerController` the bullet counter and life
// on `ShooterBulletCounterUI`, and respawns the player. Their Blueprint
// children are in `Content/Variant_Shooter/`, and the project opens and plays
// `Lvl_Shooter`.
//
// WASD / left stick move, mouse / right stick look, Space / A jump, left
// click / RT shoot, Left Shift / Y switch weapons; touch controls on touch
// devices.

rusteal_runtime::entry!();

mod camera_manager;
mod character;
mod game_mode;
mod player_controller;
mod shooter;
