// The First Person template's and its Horror variant's gameplay classes in Rust.
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
// The Horror variant (`horror/`) adds `HorrorCharacter`, a
// `FirstPersonCharacter` with a flashlight and a sprint that runs on stamina,
// `HorrorPlayerController`, which shows the sprint meter (`HorrorUI`, the
// parent of `UI_Horror`), and `HorrorGameMode`, for local multiplayer. Their
// Blueprint children are in `Content/Variant_Horror/`, and the project opens
// and plays `Lvl_Horror`.
//
// WASD / left stick move, mouse / right stick look, Space / A jump, hold
// Shift / LB sprint; touch controls on touch devices.

rusteal_runtime::entry!();

mod camera_manager;
mod character;
mod game_mode;
mod horror;
mod player_controller;
