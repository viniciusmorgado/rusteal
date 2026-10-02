// The First Person template's gameplay classes in Rust.
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
// WASD / left stick move, mouse / right stick look, Space / A jump; touch
// controls on touch devices. The template's variants (Horror, Shooter) are
// their own projects: `rusteal new --variant`.

rusteal_runtime::entry!();

mod camera_manager;
mod character;
mod game_mode;
mod player_controller;
